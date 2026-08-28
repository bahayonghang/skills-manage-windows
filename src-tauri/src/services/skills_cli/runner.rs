//! Local process execution seam for the official Skills CLI.
//!
//! Production runs `node <npx-cli.js> --yes --package=skills -- …`
//! through the shared `targets` supervisor, which provides bounded output,
//! deadlines, cancellation polling, and Windows Job Object / Unix process
//! group teardown. Tests inject [`FakeRunner`]-style doubles.

use std::path::PathBuf;
use std::process::Output as StdOutput;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use async_trait::async_trait;

use crate::targets::{
    CommandRunner, ProcessCancellation, ProcessClass, ProcessPolicy, ProcessRequest, ProcessRunner,
};

use super::error::SkillsCliError;

/// One supervised CLI invocation.
pub(crate) struct RunnerRequest<'a> {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub policy: ProcessPolicy,
    pub cancel: Option<&'a AtomicBool>,
}

/// Captured result of one invocation. Raw streams stay here; callers must
/// parse or drop them — they never reach IPC payloads or logs.
pub(crate) struct CliOutput {
    pub status_success: bool,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    /// Captured so callers can log byte lengths; contents never enter IPC
    /// payloads, tracing fields, or operation logs.
    pub stderr: Vec<u8>,
}

impl CliOutput {
    fn from_std(output: StdOutput) -> Self {
        Self {
            status_success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

#[async_trait]
pub(crate) trait SkillsCliRunner: Send + Sync {
    async fn run(&self, request: RunnerRequest<'_>) -> Result<CliOutput, SkillsCliError>;
}

pub(crate) fn map_runner_error(error: crate::targets::RunnerError) -> SkillsCliError {
    use crate::targets::{RunnerError, RunnerPhase};
    match error {
        RunnerError::Io {
            phase: RunnerPhase::Start,
            source,
        } => {
            tracing::warn!(
                phase = "start",
                io_kind = ?source.kind(),
                "Skills CLI process failed to start"
            );
            SkillsCliError::CliUnavailable
        }
        RunnerError::TimedOut { deadline, .. } => SkillsCliError::Timeout(deadline),
        RunnerError::Cancelled => SkillsCliError::Cancelled,
        RunnerError::OutputLimitExceeded { stream, .. } => SkillsCliError::OutputLimitExceeded {
            stream: stream.label(),
        },
        RunnerError::Io { source, .. } | RunnerError::TerminationFailed { source, .. } => {
            SkillsCliError::Io {
                context: "supervise Skills CLI process",
                source,
            }
        }
    }
}

/// Production runner backed by the shared process supervisor.
pub(crate) struct NodeProcessRunner;

#[async_trait]
impl SkillsCliRunner for NodeProcessRunner {
    async fn run(&self, request: RunnerRequest<'_>) -> Result<CliOutput, SkillsCliError> {
        let mut command = std::process::Command::new(&request.program);
        command.args(&request.args);
        command.env("CI", "1");
        command.env("npm_config_yes", "true");
        command.env("npm_config_update_notifier", "false");
        command.env("npm_config_fund", "false");
        let process_request = ProcessRequest::new(command, request.policy)
            .with_cancellation(ProcessCancellation::from(request.cancel));
        let output: StdOutput = ProcessRunner
            .run(process_request)
            .await
            .map_err(map_runner_error)?;
        Ok(CliOutput::from_std(output))
    }
}

/// Standard policy (120 s / 8 MiB stdout) with the CLI stderr cap held at
/// 1 MiB.
pub(crate) fn standard_policy() -> ProcessPolicy {
    ProcessPolicy {
        deadline: Duration::from_secs(120),
        stdout_limit: 8 * 1024 * 1024,
        stderr_limit: 1024 * 1024,
        class: ProcessClass::Standard,
    }
}

/// BulkTransfer policy (15 min / 32 MiB stdout) with the CLI stderr cap held
/// at 1 MiB.
pub(crate) fn bulk_transfer_policy() -> ProcessPolicy {
    ProcessPolicy {
        deadline: Duration::from_secs(15 * 60),
        stdout_limit: 32 * 1024 * 1024,
        stderr_limit: 1024 * 1024,
        class: ProcessClass::BulkTransfer,
    }
}
