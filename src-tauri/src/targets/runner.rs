//! Injectable process runner seam for SSH / WSL command execution.
//!
//! `base_command()` builders stay pure (already covered by arg-order tests);
//! everything past them — spawning, stdin piping, output capture — funnels
//! through [`CommandRunner`] so remote execution paths can be unit-tested
//! with a fake runner instead of a live `ssh` / `wsl.exe` process.

use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Which phase of process execution failed. Callers map each phase to the
/// historical error message ("Failed to start ssh" / "Failed to write ssh
/// stdin" / "Failed to wait for ssh", and the wsl.exe equivalents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerPhase {
    Start,
    WriteStdin,
    Wait,
}

#[derive(Debug)]
pub(crate) struct RunnerError {
    pub(crate) phase: RunnerPhase,
    pub(crate) source: std::io::Error,
}

pub(crate) trait CommandRunner: Send + Sync {
    /// `stdin == None` → `.stdin(null).output()`;
    /// `stdin == Some` → `spawn()` + `write_all` + `wait_with_output()`.
    fn run(&self, command: Command, stdin: Option<&[u8]>) -> Result<Output, RunnerError>;
}

/// Production runner: byte-for-byte the two execution shapes that previously
/// lived inline at each call site in `exec.rs`.
pub(crate) struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, mut command: Command, stdin: Option<&[u8]>) -> Result<Output, RunnerError> {
        match stdin {
            None => command
                .stdin(Stdio::null())
                .output()
                .map_err(|e| RunnerError {
                    phase: RunnerPhase::Start,
                    source: e,
                }),
            Some(bytes) => {
                let mut child = command
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| RunnerError {
                        phase: RunnerPhase::Start,
                        source: e,
                    })?;

                if let Some(mut child_stdin) = child.stdin.take() {
                    child_stdin.write_all(bytes).map_err(|e| RunnerError {
                        phase: RunnerPhase::WriteStdin,
                        source: e,
                    })?;
                }

                child.wait_with_output().map_err(|e| RunnerError {
                    phase: RunnerPhase::Wait,
                    source: e,
                })
            }
        }
    }
}
