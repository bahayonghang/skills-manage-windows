use std::io;
use std::process::Command;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
pub(super) static PREPARE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::null;
    use tokio::process::Child;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, JobObjectExtendedLimitInformation, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    // windows-sys gates CreateJobObjectW behind Win32_Security because its
    // optional parameter is SECURITY_ATTRIBUTES. We always pass null and keep
    // the approved feature set limited to the Job Object APIs actually used.
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> HANDLE;
    }

    pub(super) struct Guard {
        handle: isize,
    }

    impl Guard {
        pub(super) fn prepare(_command: &mut Command) -> io::Result<Self> {
            let handle = unsafe { CreateJobObjectW(null(), null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    (&raw const information).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if configured == 0 {
                let error = io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                return Err(error);
            }

            Ok(Self {
                handle: handle as isize,
            })
        }

        fn handle(&self) -> HANDLE {
            self.handle as HANDLE
        }

        pub(super) fn assign(&mut self, child: &Child) -> io::Result<()> {
            let process = child
                .raw_handle()
                .ok_or_else(|| io::Error::other("child exited before Job Object assignment"))?;
            if unsafe { AssignProcessToJobObject(self.handle(), process.cast()) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub(super) fn terminate(&mut self) -> io::Result<()> {
            if unsafe { TerminateJobObject(self.handle(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle());
            }
        }
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::process::CommandExt;
    use tokio::process::Child;

    const SIGKILL: i32 = 9;
    const ESRCH: i32 = 3;

    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }

    pub(super) struct Guard {
        process_group: Option<i32>,
    }

    impl Guard {
        pub(super) fn prepare(command: &mut Command) -> io::Result<Self> {
            command.process_group(0);
            Ok(Self {
                process_group: None,
            })
        }

        pub(super) fn assign(&mut self, child: &Child) -> io::Result<()> {
            let pid = child
                .id()
                .ok_or_else(|| io::Error::other("child exited before process-group assignment"))?;
            self.process_group = Some(pid as i32);
            Ok(())
        }

        pub(super) fn terminate(&mut self) -> io::Result<()> {
            let Some(process_group) = self.process_group else {
                return Ok(());
            };
            if unsafe { kill(-process_group, SIGKILL) } == 0 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ESRCH) {
                Ok(())
            } else {
                Err(error)
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.terminate();
        }
    }
}

pub(super) struct ProcessTreeGuard(platform::Guard);

impl ProcessTreeGuard {
    pub(super) fn prepare(command: &mut Command) -> io::Result<Self> {
        #[cfg(test)]
        PREPARE_CALLS.fetch_add(1, Ordering::SeqCst);
        #[cfg(windows)]
        super::hide_child_window(command);
        platform::Guard::prepare(command).map(Self)
    }

    pub(super) fn assign(&mut self, child: &tokio::process::Child) -> io::Result<()> {
        self.0.assign(child)
    }

    pub(super) fn terminate(&mut self) -> io::Result<()> {
        self.0.terminate()
    }
}

#[cfg(test)]
mod prepare_tests {
    use super::*;
    use std::time::Duration;

    use crate::targets::{CommandRunner, ProcessPolicy, ProcessRequest, ProcessRunner};
    #[cfg(windows)]
    use crate::targets::{CREATE_NO_WINDOW, LAST_HIDDEN_CHILD_CREATION_FLAGS};

    #[cfg(windows)]
    #[test]
    fn prepare_sets_create_no_window_on_command() {
        LAST_HIDDEN_CHILD_CREATION_FLAGS.store(0, Ordering::SeqCst);
        let mut command = Command::new("cmd");
        let _guard = ProcessTreeGuard::prepare(&mut command).expect("prepare");
        assert_eq!(
            LAST_HIDDEN_CHILD_CREATION_FLAGS.load(Ordering::SeqCst),
            CREATE_NO_WINDOW,
            "prepare must apply CREATE_NO_WINDOW via hide_child_window; Command Debug is {command:?}"
        );
    }

    #[tokio::test]
    async fn process_runner_run_goes_through_prepare() {
        PREPARE_CALLS.store(0, Ordering::SeqCst);
        let command = if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.args(["/C", "exit", "0"]);
            command
        } else {
            Command::new("true")
        };
        let request = ProcessRequest::new(
            command,
            ProcessPolicy::for_tests(Duration::from_secs(5), 64, 64),
        );
        let _ = ProcessRunner.run(request).await;
        assert!(
            PREPARE_CALLS.load(Ordering::SeqCst) >= 1,
            "ProcessRunner::run must call ProcessTreeGuard::prepare"
        );
    }
}
