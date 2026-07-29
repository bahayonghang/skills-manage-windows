use super::*;

pub(super) fn ssh_runner_error(error: RunnerError) -> TargetsError {
    runner_error(error, "SSH", "ssh")
}

pub(super) fn wsl_runner_error(error: RunnerError) -> TargetsError {
    runner_error(error, "WSL", "wsl.exe")
}

fn runner_error(
    error: RunnerError,
    transport: &'static str,
    program: &'static str,
) -> TargetsError {
    match error {
        RunnerError::Io { phase, source } => {
            let action = match phase {
                RunnerPhase::Start => "start",
                RunnerPhase::WriteStdin => "write",
                RunnerPhase::ReadStdout => "read stdout from",
                RunnerPhase::ReadStderr => "read stderr from",
                RunnerPhase::Wait => "wait for",
            };
            let context = match phase {
                RunnerPhase::WriteStdin => format!("Failed to write {program} stdin"),
                _ => format!("Failed to {action} {program}"),
            };
            TargetsError::io(context, source)
        }
        RunnerError::TimedOut { class, deadline } => TargetsError::ProcessTimedOut {
            transport,
            class: class.label(),
            timeout_ms: deadline.as_millis(),
        },
        RunnerError::Cancelled => TargetsError::ProcessCancelled(transport),
        RunnerError::OutputLimitExceeded { stream, limit } => {
            TargetsError::ProcessOutputLimitExceeded {
                transport,
                stream: stream.label(),
                limit,
            }
        }
        RunnerError::TerminationFailed { trigger, source } => {
            TargetsError::ProcessTerminationFailed {
                transport,
                trigger: trigger.label(),
                source,
            }
        }
    }
}

pub(super) async fn run_process(
    runner: &dyn CommandRunner,
    command: Command,
    stdin: Option<&[u8]>,
    policy: ProcessPolicy,
    cancellation: ProcessCancellation<'_>,
    map_error: fn(RunnerError) -> TargetsError,
) -> Result<std::process::Output, TargetsError> {
    let mut request = ProcessRequest::new(command, policy);
    if let Some(stdin) = stdin {
        request = request.with_stdin(stdin);
    }
    runner
        .run(request.with_cancellation(cancellation))
        .await
        .map_err(map_error)
}

impl ConnectedSshTarget {
    pub async fn run_probe_script(
        &self,
        script: &str,
        args: &[&str],
    ) -> Result<String, TargetsError> {
        let output = self
            .run_command_with_stdin(
                &remote_script_command(args),
                script.as_bytes(),
                ProcessPolicy::probe(),
                ProcessCancellation::Never,
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::RemoteStdoutNotUtf8)
    }

    pub async fn run_script(&self, script: &str, args: &[&str]) -> Result<String, TargetsError> {
        let output = self
            .run_command_with_stdin(
                &remote_script_command(args),
                script.as_bytes(),
                ProcessPolicy::standard(),
                ProcessCancellation::Never,
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::RemoteStdoutNotUtf8)
    }

    pub(crate) async fn run_script_cancellable(
        &self,
        script: &str,
        args: &[&str],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<String, TargetsError> {
        let output = self
            .run_command_with_stdin(
                &remote_script_command(args),
                script.as_bytes(),
                ProcessPolicy::bulk_transfer(),
                cancel.into(),
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::RemoteStdoutNotUtf8)
    }

    pub async fn run_command_with_stdin_bytes(
        &self,
        command: &str,
        stdin: &[u8],
    ) -> Result<Vec<u8>, TargetsError> {
        self.run_command_with_stdin(
            command,
            stdin,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
        )
        .await
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn run_command_with_stdin_bytes_cancellable(
        &self,
        command: &str,
        stdin: &[u8],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Vec<u8>, TargetsError> {
        self.run_command_with_stdin(
            command,
            stdin,
            ProcessPolicy::bulk_transfer(),
            cancel.into(),
        )
        .await
    }

    pub async fn run_command(&self, command: &str) -> Result<String, TargetsError> {
        self.run_command_with_control(
            command,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
        )
        .await
    }

    pub(super) async fn run_command_with_control(
        &self,
        command: &str,
        policy: ProcessPolicy,
        cancellation: ProcessCancellation<'_>,
    ) -> Result<String, TargetsError> {
        let mut process = self.base_command();
        process.arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            policy,
            cancellation,
            ssh_runner_error,
        )
        .await?;
        if output.status.success() {
            String::from_utf8(output.stdout).map_err(TargetsError::RemoteStdoutNotUtf8)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    pub(super) async fn run_command_with_stdin(
        &self,
        command: &str,
        stdin: &[u8],
        policy: ProcessPolicy,
        cancellation: ProcessCancellation<'_>,
    ) -> Result<Vec<u8>, TargetsError> {
        let mut process = self.base_command();
        process.arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            Some(stdin),
            policy,
            cancellation,
            ssh_runner_error,
        )
        .await?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    pub(super) async fn run_command_bytes(&self, command: &str) -> Result<Vec<u8>, TargetsError> {
        let mut process = self.base_command();
        process.arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
            ssh_runner_error,
        )
        .await?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }
}

impl ConnectedWslTarget {
    pub async fn run_probe_script(
        &self,
        script: &str,
        args: &[&str],
    ) -> Result<String, TargetsError> {
        let mut command = self.base_command();
        command.arg("sh").arg("-s").arg("--");
        for arg in args {
            command.arg(arg);
        }
        let output = self
            .run_command_process_with_stdin(
                command,
                script.as_bytes(),
                ProcessPolicy::probe(),
                ProcessCancellation::Never,
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::WslStdoutNotUtf8)
    }

    pub async fn run_script(&self, script: &str, args: &[&str]) -> Result<String, TargetsError> {
        let mut command = self.base_command();
        command.arg("sh").arg("-s").arg("--");
        for arg in args {
            command.arg(arg);
        }
        let output = self
            .run_command_process_with_stdin(
                command,
                script.as_bytes(),
                ProcessPolicy::standard(),
                ProcessCancellation::Never,
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::WslStdoutNotUtf8)
    }

    pub(crate) async fn run_script_cancellable(
        &self,
        script: &str,
        args: &[&str],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<String, TargetsError> {
        let mut command = self.base_command();
        command.arg("sh").arg("-s").arg("--");
        for arg in args {
            command.arg(arg);
        }
        let output = self
            .run_command_process_with_stdin(
                command,
                script.as_bytes(),
                ProcessPolicy::bulk_transfer(),
                cancel.into(),
            )
            .await?;
        String::from_utf8(output).map_err(TargetsError::WslStdoutNotUtf8)
    }

    pub async fn run_command_with_stdin_bytes(
        &self,
        command: &str,
        stdin: &[u8],
    ) -> Result<Vec<u8>, TargetsError> {
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        self.run_command_process_with_stdin(
            process,
            stdin,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
        )
        .await
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn run_command_with_stdin_bytes_cancellable(
        &self,
        command: &str,
        stdin: &[u8],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Vec<u8>, TargetsError> {
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        self.run_command_process_with_stdin(
            process,
            stdin,
            ProcessPolicy::bulk_transfer(),
            cancel.into(),
        )
        .await
    }

    pub async fn run_command(&self, command: &str) -> Result<String, TargetsError> {
        self.run_command_with_control(
            command,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
        )
        .await
    }

    pub(super) async fn run_command_with_control(
        &self,
        command: &str,
        policy: ProcessPolicy,
        cancellation: ProcessCancellation<'_>,
    ) -> Result<String, TargetsError> {
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            policy,
            cancellation,
            wsl_runner_error,
        )
        .await?;
        if output.status.success() {
            String::from_utf8(output.stdout).map_err(TargetsError::WslStdoutNotUtf8)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    pub(super) async fn run_command_process_with_stdin(
        &self,
        command: Command,
        stdin: &[u8],
        policy: ProcessPolicy,
        cancellation: ProcessCancellation<'_>,
    ) -> Result<Vec<u8>, TargetsError> {
        let output = run_process(
            self.runner.as_ref(),
            command,
            Some(stdin),
            policy,
            cancellation,
            wsl_runner_error,
        )
        .await?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }

    pub(super) async fn run_command_bytes(&self, command: &str) -> Result<Vec<u8>, TargetsError> {
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
            wsl_runner_error,
        )
        .await?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(self.remote_command_error(output.status, &output.stderr))
        }
    }
}
