//! Injectable, asynchronous process supervision for SSH / WSL execution.

use super::process_tree::ProcessTreeGuard;
use async_trait::async_trait;
use std::future::pending;
use std::io;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

const ATOMIC_CANCEL_POLL: Duration = Duration::from_millis(50);
const REAP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessClass {
    Probe,
    Standard,
    BulkTransfer,
}

impl ProcessClass {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Standard => "standard",
            Self::BulkTransfer => "bulk_transfer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessPolicy {
    pub(crate) class: ProcessClass,
    pub(crate) deadline: Duration,
    pub(crate) stdout_limit: usize,
    pub(crate) stderr_limit: usize,
}

impl ProcessPolicy {
    const MIB: usize = 1024 * 1024;

    pub(crate) const fn probe() -> Self {
        Self {
            class: ProcessClass::Probe,
            deadline: Duration::from_secs(30),
            stdout_limit: Self::MIB,
            stderr_limit: Self::MIB,
        }
    }

    pub(crate) const fn standard() -> Self {
        Self {
            class: ProcessClass::Standard,
            deadline: Duration::from_secs(120),
            stdout_limit: 8 * Self::MIB,
            stderr_limit: 8 * Self::MIB,
        }
    }

    pub(crate) const fn bulk_transfer() -> Self {
        Self {
            class: ProcessClass::BulkTransfer,
            deadline: Duration::from_secs(15 * 60),
            stdout_limit: 32 * Self::MIB,
            stderr_limit: 32 * Self::MIB,
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_tests(
        deadline: Duration,
        stdout_limit: usize,
        stderr_limit: usize,
    ) -> Self {
        Self {
            class: ProcessClass::Standard,
            deadline,
            stdout_limit,
            stderr_limit,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) enum ProcessCancellation<'a> {
    #[default]
    Never,
    Atomic(&'a AtomicBool),
}

impl ProcessCancellation<'_> {
    async fn cancelled(&self) {
        match self {
            Self::Never => pending().await,
            Self::Atomic(flag) => loop {
                if flag.load(Ordering::SeqCst) {
                    return;
                }
                tokio::time::sleep(ATOMIC_CANCEL_POLL).await;
            },
        }
    }
}

impl<'a> From<Option<&'a AtomicBool>> for ProcessCancellation<'a> {
    fn from(flag: Option<&'a AtomicBool>) -> Self {
        flag.map_or(Self::Never, Self::Atomic)
    }
}

pub(crate) struct ProcessRequest<'a> {
    pub(crate) command: Command,
    pub(crate) stdin: Option<Vec<u8>>,
    pub(crate) policy: ProcessPolicy,
    pub(crate) cancellation: ProcessCancellation<'a>,
}

impl ProcessRequest<'_> {
    pub(crate) fn new(command: Command, policy: ProcessPolicy) -> Self {
        Self {
            command,
            stdin: None,
            policy,
            cancellation: ProcessCancellation::Never,
        }
    }

    pub(crate) fn with_stdin(mut self, stdin: &[u8]) -> Self {
        self.stdin = Some(stdin.to_vec());
        self
    }

    pub(crate) fn with_cancellation<'a>(
        self,
        cancellation: ProcessCancellation<'a>,
    ) -> ProcessRequest<'a> {
        ProcessRequest {
            command: self.command,
            stdin: self.stdin,
            policy: self.policy,
            cancellation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerPhase {
    Start,
    WriteStdin,
    ReadStdout,
    ReadStderr,
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerStream {
    Stdout,
    Stderr,
}

impl RunnerStream {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminationTrigger {
    Cancelled,
    TimedOut,
    OutputLimit,
    Io,
}

impl TerminationTrigger {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Cancelled => "cancellation",
            Self::TimedOut => "timeout",
            Self::OutputLimit => "output limit",
            Self::Io => "I/O failure",
        }
    }
}

#[derive(Debug)]
pub(crate) enum RunnerError {
    Io {
        phase: RunnerPhase,
        source: io::Error,
    },
    TimedOut {
        class: ProcessClass,
        deadline: Duration,
    },
    Cancelled,
    OutputLimitExceeded {
        stream: RunnerStream,
        limit: usize,
    },
    TerminationFailed {
        trigger: TerminationTrigger,
        source: io::Error,
    },
}

impl RunnerError {
    fn io(phase: RunnerPhase, source: io::Error) -> Self {
        Self::Io { phase, source }
    }

    fn termination_trigger(&self) -> TerminationTrigger {
        match self {
            Self::Cancelled => TerminationTrigger::Cancelled,
            Self::TimedOut { .. } => TerminationTrigger::TimedOut,
            Self::OutputLimitExceeded { .. } => TerminationTrigger::OutputLimit,
            Self::Io { .. } | Self::TerminationFailed { .. } => TerminationTrigger::Io,
        }
    }
}

#[async_trait]
pub(crate) trait CommandRunner: Send + Sync {
    async fn run(&self, request: ProcessRequest<'_>) -> Result<Output, RunnerError>;
}

pub(crate) struct ProcessRunner;

#[async_trait]
impl CommandRunner for ProcessRunner {
    async fn run(&self, mut request: ProcessRequest<'_>) -> Result<Output, RunnerError> {
        let mut tree = ProcessTreeGuard::prepare(&mut request.command)
            .map_err(|error| RunnerError::io(RunnerPhase::Start, error))?;
        request.command.stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        request
            .command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut command = tokio::process::Command::from(request.command);
        command.kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|error| RunnerError::io(RunnerPhase::Start, error))?;
        if let Err(error) = tree.assign(&child) {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RunnerError::io(RunnerPhase::Start, error));
        }

        let child_stdin = child.stdin.take();
        let child_stdout = child.stdout.take().ok_or_else(|| {
            RunnerError::io(
                RunnerPhase::ReadStdout,
                io::Error::other("stdout pipe unavailable"),
            )
        })?;
        let child_stderr = child.stderr.take().ok_or_else(|| {
            RunnerError::io(
                RunnerPhase::ReadStderr,
                io::Error::other("stderr pipe unavailable"),
            )
        })?;

        let completion = async {
            let write_stdin = async move {
                if let (Some(mut pipe), Some(bytes)) = (child_stdin, request.stdin) {
                    pipe.write_all(&bytes)
                        .await
                        .map_err(|error| RunnerError::io(RunnerPhase::WriteStdin, error))?;
                    pipe.shutdown()
                        .await
                        .map_err(|error| RunnerError::io(RunnerPhase::WriteStdin, error))?;
                }
                Ok(())
            };
            let read_stdout = read_bounded(
                child_stdout,
                RunnerStream::Stdout,
                request.policy.stdout_limit,
            );
            let read_stderr = read_bounded(
                child_stderr,
                RunnerStream::Stderr,
                request.policy.stderr_limit,
            );
            let wait = async {
                child
                    .wait()
                    .await
                    .map_err(|error| RunnerError::io(RunnerPhase::Wait, error))
            };
            tokio::try_join!(write_stdin, read_stdout, read_stderr, wait)
        };

        enum Outcome {
            Completed(Result<((), Vec<u8>, Vec<u8>, ExitStatus), RunnerError>),
            Cancelled,
            TimedOut,
        }

        let outcome = {
            tokio::pin!(completion);
            tokio::select! {
                biased;
                result = &mut completion => Outcome::Completed(result),
                _ = request.cancellation.cancelled() => Outcome::Cancelled,
                _ = tokio::time::sleep(request.policy.deadline) => Outcome::TimedOut,
            }
        };

        let primary_error = match outcome {
            Outcome::Completed(Ok(((), stdout, stderr, status))) => {
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Outcome::Completed(Err(error)) => error,
            Outcome::Cancelled => RunnerError::Cancelled,
            Outcome::TimedOut => RunnerError::TimedOut {
                class: request.policy.class,
                deadline: request.policy.deadline,
            },
        };

        let trigger = primary_error.termination_trigger();
        if let Err(error) = terminate_and_reap(&mut child, &mut tree).await {
            return Err(RunnerError::TerminationFailed {
                trigger,
                source: error,
            });
        }
        Err(primary_error)
    }
}

async fn read_bounded<R>(
    mut reader: R,
    stream: RunnerStream,
    limit: usize,
) -> Result<Vec<u8>, RunnerError>
where
    R: AsyncRead + Unpin,
{
    let mut output = Vec::with_capacity(limit.min(64 * 1024));
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut chunk)
            .await
            .map_err(|error| RunnerError::io(read_phase(stream), error))?;
        if read == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(read) > limit {
            return Err(RunnerError::OutputLimitExceeded { stream, limit });
        }
        output.extend_from_slice(&chunk[..read]);
    }
}

const fn read_phase(stream: RunnerStream) -> RunnerPhase {
    match stream {
        RunnerStream::Stdout => RunnerPhase::ReadStdout,
        RunnerStream::Stderr => RunnerPhase::ReadStderr,
    }
}

async fn terminate_and_reap(
    child: &mut tokio::process::Child,
    tree: &mut ProcessTreeGuard,
) -> io::Result<()> {
    if let Err(tree_error) = tree.terminate() {
        let _ = child.start_kill();
        let _ = tokio::time::timeout(REAP_TIMEOUT, child.wait()).await;
        return Err(tree_error);
    }

    match tokio::time::timeout(REAP_TIMEOUT, child.wait()).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "timed out while reaping terminated child process",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Instant;

    const FIXTURE_MODE_ENV: &str = "SKILLPORT_PROCESS_FIXTURE_MODE";
    const FIXTURE_READY_ENV: &str = "SKILLPORT_PROCESS_FIXTURE_READY";
    const FIXTURE_LEAK_ENV: &str = "SKILLPORT_PROCESS_FIXTURE_LEAK";
    const FIXTURE_TEST: &str = "targets::runner::tests::supervised_process_fixture";

    #[cfg(unix)]
    fn close_fixture_stdin() {
        use std::os::fd::{FromRawFd, OwnedFd};

        // SAFETY: the fixture is spawned with a dedicated piped stdin and does
        // not access the descriptor again after this helper closes it.
        drop(unsafe { OwnedFd::from_raw_fd(0) });
    }

    #[cfg(windows)]
    fn close_fixture_stdin() {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::CloseHandle;

        let handle = std::io::stdin().as_raw_handle();
        // SAFETY: the fixture is spawned with a dedicated piped stdin and does
        // not access the handle again after this helper closes it.
        assert_ne!(unsafe { CloseHandle(handle) }, 0, "close fixture stdin");
    }

    fn fixture_command(mode: &str, ready: Option<&Path>, leak: Option<&Path>) -> Command {
        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        command
            .arg("--exact")
            .arg(FIXTURE_TEST)
            .arg("--ignored")
            .arg("--nocapture")
            .env(FIXTURE_MODE_ENV, mode);
        if let Some(path) = ready {
            command.env(FIXTURE_READY_ENV, path);
        }
        if let Some(path) = leak {
            command.env(FIXTURE_LEAK_ENV, path);
        }
        command
    }

    fn test_request(command: Command, deadline: Duration, limit: usize) -> ProcessRequest<'static> {
        ProcessRequest::new(command, ProcessPolicy::for_tests(deadline, limit, limit))
    }

    async fn wait_for_path(path: &Path) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("fixture ready marker");
    }

    #[test]
    #[ignore = "spawned by ProcessRunner tests as a controlled child process"]
    fn supervised_process_fixture() {
        let mode = std::env::var(FIXTURE_MODE_ENV).expect("fixture mode");
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "large_stdout" => {
                // Write well past the test limit in chunks so a full pipe buffer
                // cannot hide the overflow behind a single short write.
                let chunk = vec![b'x'; 8 * 1024];
                for _ in 0..8 {
                    if std::io::stdout().write_all(&chunk).is_err() {
                        break;
                    }
                }
                let _ = std::io::stdout().flush();
            }
            "large_stderr" => {
                let chunk = vec![b'x'; 8 * 1024];
                for _ in 0..8 {
                    if std::io::stderr().write_all(&chunk).is_err() {
                        break;
                    }
                }
                let _ = std::io::stderr().flush();
            }
            "close_stdin" => {
                close_fixture_stdin();
                // Stay alive long enough for the parent to observe the closed
                // pipe instead of racing a fast child exit.
                std::thread::sleep(Duration::from_secs(2));
            }
            "barrier" => {
                let ready = PathBuf::from(std::env::var_os(FIXTURE_READY_ENV).expect("ready"));
                let peer = PathBuf::from(std::env::var_os(FIXTURE_LEAK_ENV).expect("peer"));
                std::fs::write(ready, b"ready").expect("write ready marker");
                let deadline = Instant::now() + Duration::from_secs(3);
                while !peer.exists() && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(10));
                }
                assert!(peer.exists(), "peer process did not start concurrently");
            }
            "tree_parent" => {
                let ready = PathBuf::from(std::env::var_os(FIXTURE_READY_ENV).expect("ready"));
                let leak = PathBuf::from(std::env::var_os(FIXTURE_LEAK_ENV).expect("leak"));
                let mut child = fixture_command("tree_child", None, Some(&leak))
                    .spawn()
                    .expect("spawn tree child");
                std::fs::write(ready, b"ready").expect("write ready marker");
                child.wait().expect("wait for tree child");
            }
            "tree_child" => {
                let leak = PathBuf::from(std::env::var_os(FIXTURE_LEAK_ENV).expect("leak"));
                std::thread::sleep(Duration::from_millis(750));
                std::fs::write(leak, b"leaked").expect("write leak marker");
                std::thread::sleep(Duration::from_secs(30));
            }
            other => panic!("unknown fixture mode: {other}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn timeout_is_bounded_without_blocking_the_runtime() {
        let runner = ProcessRunner;
        let started = Instant::now();
        let request = test_request(
            fixture_command("sleep", None, None),
            Duration::from_millis(150),
            1024,
        );
        let independent_tick = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            started.elapsed()
        };

        let (result, tick_elapsed) = tokio::join!(runner.run(request), independent_tick);

        assert!(matches!(result, Err(RunnerError::TimedOut { .. })));
        assert!(tick_elapsed < Duration::from_secs(1));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn atomic_cancellation_terminates_a_running_process() {
        let runner = ProcessRunner;
        let cancel = AtomicBool::new(false);
        let request = test_request(
            fixture_command("sleep", None, None),
            Duration::from_secs(10),
            1024,
        )
        .with_cancellation(ProcessCancellation::Atomic(&cancel));
        let request_cancel = async {
            tokio::time::sleep(Duration::from_millis(75)).await;
            cancel.store(true, Ordering::SeqCst);
        };

        let (result, ()) = tokio::join!(runner.run(request), request_cancel);

        assert!(matches!(result, Err(RunnerError::Cancelled)));
    }

    #[tokio::test]
    async fn concurrent_supervisors_do_not_serialize_targets() {
        let temp = tempfile::tempdir().unwrap();
        let left_ready = temp.path().join("left-ready");
        let right_ready = temp.path().join("right-ready");
        let left = ProcessRunner.run(test_request(
            fixture_command("barrier", Some(&left_ready), Some(&right_ready)),
            Duration::from_secs(5),
            1024,
        ));
        let right = ProcessRunner.run(test_request(
            fixture_command("barrier", Some(&right_ready), Some(&left_ready)),
            Duration::from_secs(5),
            1024,
        ));

        let (left, right) = tokio::join!(left, right);

        assert!(left.expect("left supervisor").status.success());
        assert!(right.expect("right supervisor").status.success());
    }

    #[tokio::test]
    async fn stdout_overflow_terminates_the_process() {
        let result = ProcessRunner
            .run(test_request(
                fixture_command("large_stdout", None, None),
                Duration::from_secs(3),
                1024,
            ))
            .await;

        assert!(
            matches!(
                result,
                Err(RunnerError::OutputLimitExceeded {
                    stream: RunnerStream::Stdout,
                    limit: 1024
                })
            ),
            "unexpected stdout overflow result: {result:?}"
        );
    }

    #[tokio::test]
    async fn stderr_overflow_terminates_the_process() {
        let result = ProcessRunner
            .run(test_request(
                fixture_command("large_stderr", None, None),
                Duration::from_secs(5),
                1024,
            ))
            .await;

        assert!(
            matches!(
                result,
                Err(RunnerError::OutputLimitExceeded {
                    stream: RunnerStream::Stderr,
                    limit: 1024
                })
            ),
            "unexpected stderr overflow result: {result:?}"
        );
    }

    #[tokio::test]
    async fn closed_stdin_is_classified_as_write_failure() {
        let request = test_request(
            fixture_command("close_stdin", None, None),
            Duration::from_secs(5),
            1024,
        )
        .with_stdin(&vec![b'x'; 8 * 1024 * 1024]);

        let result = ProcessRunner.run(request).await;

        assert!(
            matches!(
                result,
                Err(RunnerError::Io {
                    phase: RunnerPhase::WriteStdin,
                    ..
                })
            ),
            "unexpected closed-stdin result: {result:?}"
        );
    }

    #[tokio::test]
    async fn cancellation_kills_descendants() {
        let temp = tempfile::tempdir().unwrap();
        let ready = temp.path().join("ready");
        let leak = temp.path().join("leak");
        let cancel = AtomicBool::new(false);
        let request = test_request(
            fixture_command("tree_parent", Some(&ready), Some(&leak)),
            Duration::from_secs(10),
            1024,
        )
        .with_cancellation(ProcessCancellation::Atomic(&cancel));
        let cancel_when_ready = async {
            wait_for_path(&ready).await;
            cancel.store(true, Ordering::SeqCst);
        };

        let (result, ()) = tokio::join!(ProcessRunner.run(request), cancel_when_ready);
        assert!(matches!(result, Err(RunnerError::Cancelled)));
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(
            !leak.exists(),
            "descendant survived process-tree cancellation"
        );
    }

    #[tokio::test]
    async fn dropping_supervisor_future_kills_descendants() {
        let temp = tempfile::tempdir().unwrap();
        let ready = temp.path().join("ready");
        let leak = temp.path().join("leak");
        let runner = Arc::new(ProcessRunner);
        let request = test_request(
            fixture_command("tree_parent", Some(&ready), Some(&leak)),
            Duration::from_secs(10),
            1024,
        );
        let task_runner = Arc::clone(&runner);
        let task = tokio::spawn(async move { task_runner.run(request).await });

        wait_for_path(&ready).await;
        task.abort();
        let _ = task.await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        assert!(!leak.exists(), "descendant survived supervisor future drop");
    }
}
