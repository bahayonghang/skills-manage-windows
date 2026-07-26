use super::*;
pub(super) struct SshProbe {
    pub(super) remote_home: String,
    pub(super) remote_os: String,
}

pub(super) const SSH_CONNECT_TIMEOUT_SECS: u64 = 10;
pub(super) const SSH_SERVER_ALIVE_INTERVAL_SECS: u64 = 15;
pub(super) const SSH_SERVER_ALIVE_COUNT_MAX: u64 = 3;

pub(super) fn askpass_password_from_env(
    marker: Option<OsString>,
    password: Option<OsString>,
) -> Option<String> {
    marker?;
    Some(password.unwrap_or_default().to_string_lossy().into_owned())
}

pub fn maybe_run_ssh_askpass_helper() -> bool {
    let Some(password) = askpass_password_from_env(
        env::var_os(SSH_ASKPASS_HELPER_ENV),
        env::var_os(SSH_PASSWORD_ENV),
    ) else {
        return false;
    };

    print!("{}", password);
    let _ = std::io::stdout().flush();
    true
}

pub(super) fn remote_probe_script() -> String {
    format!(
        r#"printf 'HOME\t%s\n' "$HOME"
printf 'OS\t%s\n' "$(uname -s 2>/dev/null || printf '%s' unknown)"
mkdir -p -- "$HOME/{central_skills}" && printf 'MKDIR_OK\n'"#,
        central_skills = crate::paths::CENTRAL_SKILLS_REL_FROM_HOME
    )
}

pub(super) async fn probe_ssh_target(
    target: &RemoteTargetConfig,
) -> Result<SshProbe, TargetsError> {
    let connection = open_ssh_target(target)?;
    let output = connection
        .run_probe_script(&remote_probe_script(), &[])
        .await?;
    let probe = parse_ssh_probe_output(&output)?;

    Ok(probe)
}

pub(super) async fn probe_wsl_target(target: &WslTargetConfig) -> Result<SshProbe, TargetsError> {
    let connection = open_wsl_target(target)?;
    let output = connection
        .run_probe_script(&remote_probe_script(), &[])
        .await?;
    parse_ssh_probe_output(&output)
}

pub(super) fn is_supported_remote_os(remote_os: &str) -> bool {
    matches!(remote_os, "Linux" | "Darwin")
}

pub fn remote_symlink_allowed(target: &RemoteTargetConfig) -> bool {
    remote_target_symlink_allowed(&target.remote_os, target.symlink_enabled)
}

pub fn wsl_symlink_allowed(target: &WslTargetConfig) -> bool {
    remote_target_symlink_allowed(&target.remote_os, target.symlink_enabled)
}

pub fn remote_target_symlink_allowed(remote_os: &str, symlink_enabled: bool) -> bool {
    symlink_enabled || is_supported_remote_os(remote_os)
}

#[derive(Debug, Clone)]
pub struct RemoteDirEntry {
    pub name: String,
    pub file_type: String,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePathInfo {
    pub file_type: String,
    pub symlink_target: Option<String>,
}

pub(super) fn ssh_program() -> std::ffi::OsString {
    #[cfg(windows)]
    {
        if let Ok(windir) = env::var("WINDIR") {
            let candidate = PathBuf::from(windir)
                .join("System32")
                .join("OpenSSH")
                .join("ssh.exe");
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    "ssh".into()
}

pub(super) fn wsl_program() -> std::ffi::OsString {
    #[cfg(windows)]
    {
        if let Ok(windir) = env::var("WINDIR") {
            let candidate = PathBuf::from(windir).join("System32").join("wsl.exe");
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    "wsl".into()
}

#[derive(Clone)]
pub struct ConnectedWslTarget {
    pub(super) target: WslTargetConfig,
    pub(super) runner: Arc<dyn CommandRunner>,
}

impl std::fmt::Debug for ConnectedWslTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectedWslTarget")
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}

pub fn open_wsl_target(target: &WslTargetConfig) -> Result<ConnectedWslTarget, TargetsError> {
    #[cfg(not(windows))]
    {
        let _ = target;
        Err(TargetsError::WslWindowsOnly)
    }

    #[cfg(windows)]
    {
        Ok(ConnectedWslTarget {
            target: target.clone(),
            runner: Arc::new(ProcessRunner),
        })
    }
}

impl ConnectedSshTarget {
    pub(super) fn base_command(&self) -> Command {
        let mut command = Command::new(ssh_program());
        #[cfg(windows)]
        {
            hide_child_window(&mut command);
        }
        command
            .arg("-p")
            .arg(self.target.port.to_string())
            .arg("-o")
            .arg(format!("ConnectTimeout={}", SSH_CONNECT_TIMEOUT_SECS))
            .arg("-o")
            .arg(format!(
                "ServerAliveInterval={}",
                SSH_SERVER_ALIVE_INTERVAL_SECS
            ))
            .arg("-o")
            .arg(format!(
                "ServerAliveCountMax={}",
                SSH_SERVER_ALIVE_COUNT_MAX
            ))
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new");

        match self.target.auth_method {
            SshAuthMethod::Key => {
                command
                    .arg("-i")
                    .arg(&self.target.key_path)
                    .arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("PreferredAuthentications=publickey");
            }
            SshAuthMethod::Password => {
                command
                    .arg("-o")
                    .arg("BatchMode=no")
                    .arg("-o")
                    .arg("PreferredAuthentications=password,keyboard-interactive")
                    .arg("-o")
                    .arg("PubkeyAuthentication=no")
                    .arg("-o")
                    .arg("NumberOfPasswordPrompts=1");
                if let (Some(helper), Some(password)) = (&self.askpass_helper, &self.password) {
                    command
                        .env("SSH_ASKPASS", &helper.path)
                        .env("SSH_ASKPASS_REQUIRE", "force")
                        .env("DISPLAY", "skillport")
                        .env(SSH_PASSWORD_ENV, password);
                    if helper.use_app_helper_env {
                        command.env(SSH_ASKPASS_HELPER_ENV, "1");
                    }
                }
            }
        }
        command.arg(format!("{}@{}", self.target.username, self.target.host));
        command
    }

    pub(super) fn remote_command_error(&self, status: ExitStatus, stderr: &[u8]) -> TargetsError {
        TargetsError::RemoteCommandFailed {
            status,
            detail: self.ssh_failure_detail(stderr),
        }
    }

    pub(super) fn ssh_failure_detail(&self, stderr: &[u8]) -> String {
        let detail = String::from_utf8_lossy(stderr);
        let detail = detail.trim();
        if let Some(message) = self.ssh_auth_failure_message(detail) {
            if detail.is_empty() {
                message
            } else {
                format!("{} Raw ssh error: {}", message, detail)
            }
        } else if detail.is_empty() {
            "ssh exited without stderr.".to_string()
        } else {
            detail.to_string()
        }
    }

    fn ssh_auth_failure_message(&self, stderr: &str) -> Option<String> {
        if !stderr.to_ascii_lowercase().contains("permission denied") {
            return None;
        }

        let destination = format!("{}@{}", self.target.username, self.target.host);
        let message = match self.target.auth_method {
            SshAuthMethod::Key => {
                let key_hint = self.target.key_path.trim();
                if key_hint.is_empty() {
                    format!(
                        "SSH authentication failed for {} using key authentication. Check that the target uses the correct username and port, and configure a private key path for this Windows machine.",
                        destination
                    )
                } else {
                    format!(
                        "SSH authentication failed for {} using key authentication with '{}'. Check that the private key path is correct on this Windows machine and that the matching public key is installed in the remote user's ~/.ssh/authorized_keys.",
                        destination, key_hint
                    )
                }
            }
            SshAuthMethod::Password => format!(
                "SSH authentication failed for {} using password authentication. Check that the password is correct and that the remote sshd allows password or keyboard-interactive login for this user.",
                destination
            ),
        };
        Some(message)
    }

    pub async fn ensure_dir(&self, path: &str) -> Result<(), TargetsError> {
        if self.exists(path).await? {
            return Ok(());
        }
        Err(TargetsError::RemotePathMissing(path.to_string()))
    }

    pub async fn exists(&self, path: &str) -> Result<bool, TargetsError> {
        let command = format!("test -e {}", shell_quote(path));
        let mut process = self.base_command();
        process.arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::probe(),
            ProcessCancellation::Never,
            ssh_runner_error,
        )
        .await?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(TargetsError::RemoteInspectFailed {
                path: path.to_string(),
                detail: self.ssh_failure_detail(&output.stderr),
            }),
        }
    }

    pub async fn inspect_path(&self, path: &str) -> Result<Option<RemotePathInfo>, TargetsError> {
        let command = format!(
            r#"p={path}; if [ -L "$p" ]; then printf 'symlink\t%s\n' "$(readlink "$p" || true)"; elif [ -d "$p" ]; then printf 'dir\t\n'; elif [ -f "$p" ]; then printf 'file\t\n'; elif [ -e "$p" ]; then printf 'other\t\n'; else exit 1; fi"#,
            path = shell_quote(path)
        );
        let mut process = self.base_command();
        process.arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::probe(),
            ProcessCancellation::Never,
            ssh_runner_error,
        )
        .await?;
        match output.status.code() {
            Some(0) => {
                let stdout =
                    String::from_utf8(output.stdout).map_err(TargetsError::RemoteStdoutNotUtf8)?;
                let mut parts = stdout.trim_end().splitn(2, '\t');
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Ok(Some(RemotePathInfo {
                    file_type,
                    symlink_target,
                }))
            }
            Some(1) => Ok(None),
            _ => Err(TargetsError::RemoteInspectFailed {
                path: path.to_string(),
                detail: self.ssh_failure_detail(&output.stderr),
            }),
        }
    }

    pub async fn mkdir_p(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("mkdir -p {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), TargetsError> {
        if let Some(parent) = remote_parent(path) {
            self.mkdir_p(&parent).await?;
        }
        let command = format!("cat > {}", shell_quote(path));
        self.run_command_with_stdin(
            &command,
            bytes,
            ProcessPolicy::standard(),
            ProcessCancellation::Never,
        )
        .await
        .map(|_| ())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, TargetsError> {
        self.run_command_bytes(&format!("cat {}", shell_quote(path)))
            .await
    }

    pub async fn copy_dir(&self, source: &str, target: &str) -> Result<(), TargetsError> {
        let command = format!(
            "mkdir -p {target} && cp -R {source}/. {target}/",
            source = shell_quote(source),
            target = shell_quote(target)
        );
        self.run_command_with_control(
            &command,
            ProcessPolicy::bulk_transfer(),
            ProcessCancellation::Never,
        )
        .await
        .map(|_| ())
    }

    pub async fn remove_tree(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("rm -rf -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("rm -f -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, TargetsError> {
        let command = format!(
            r#"d={dir}; for p in "$d"/* "$d"/.[!.]* "$d"/..?*; do [ -e "$p" ] || continue; name=${{p##*/}}; link=""; if [ -L "$p" ]; then kind="symlink"; link=$(readlink "$p" || true); elif [ -d "$p" ]; then kind="dir"; elif [ -f "$p" ]; then kind="file"; else kind="other"; fi; printf '%s\t%s\t%s\n' "$name" "$kind" "$link"; done"#,
            dir = shell_quote(path)
        );
        let output = self.run_command(&command).await?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let name = parts.next()?.to_string();
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Some(RemoteDirEntry {
                    name,
                    file_type,
                    symlink_target,
                })
            })
            .collect())
    }
}

impl ConnectedWslTarget {
    pub(super) fn base_command(&self) -> Command {
        let mut command = Command::new(wsl_program());
        #[cfg(windows)]
        {
            hide_child_window(&mut command);
        }
        command
            .arg("-d")
            .arg(&self.target.distribution)
            .arg("--exec");
        command
    }

    pub(super) fn remote_command_error(&self, status: ExitStatus, stderr: &[u8]) -> TargetsError {
        TargetsError::WslCommandFailed {
            status,
            detail: self.wsl_failure_detail(stderr),
        }
    }

    pub(super) fn wsl_failure_detail(&self, stderr: &[u8]) -> String {
        let detail = String::from_utf8_lossy(stderr);
        let detail = detail.trim();
        if detail.is_empty() {
            format!(
                "wsl.exe exited without stderr for distro '{}'. Verify the distro exists with `wsl.exe -l -q` and that it can run `sh`.",
                self.target.distribution
            )
        } else {
            detail.to_string()
        }
    }

    pub async fn ensure_dir(&self, path: &str) -> Result<(), TargetsError> {
        if self.exists(path).await? {
            return Ok(());
        }
        Err(TargetsError::WslPathMissing(path.to_string()))
    }

    pub async fn exists(&self, path: &str) -> Result<bool, TargetsError> {
        let command = format!("test -e {}", shell_quote(path));
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::probe(),
            ProcessCancellation::Never,
            wsl_runner_error,
        )
        .await?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(TargetsError::WslInspectFailed {
                path: path.to_string(),
                detail: self.wsl_failure_detail(&output.stderr),
            }),
        }
    }

    pub async fn inspect_path(&self, path: &str) -> Result<Option<RemotePathInfo>, TargetsError> {
        let command = format!(
            r#"p={path}; if [ -L "$p" ]; then printf 'symlink\t%s\n' "$(readlink "$p" || true)"; elif [ -d "$p" ]; then printf 'dir\t\n'; elif [ -f "$p" ]; then printf 'file\t\n'; elif [ -e "$p" ]; then printf 'other\t\n'; else exit 1; fi"#,
            path = shell_quote(path)
        );
        let mut process = self.base_command();
        process.arg("sh").arg("-lc").arg(command);
        let output = run_process(
            self.runner.as_ref(),
            process,
            None,
            ProcessPolicy::probe(),
            ProcessCancellation::Never,
            wsl_runner_error,
        )
        .await?;
        match output.status.code() {
            Some(0) => {
                let stdout =
                    String::from_utf8(output.stdout).map_err(TargetsError::WslStdoutNotUtf8)?;
                let mut parts = stdout.trim_end().splitn(2, '\t');
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Ok(Some(RemotePathInfo {
                    file_type,
                    symlink_target,
                }))
            }
            Some(1) => Ok(None),
            _ => Err(TargetsError::WslInspectFailed {
                path: path.to_string(),
                detail: self.wsl_failure_detail(&output.stderr),
            }),
        }
    }

    pub async fn mkdir_p(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("mkdir -p {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), TargetsError> {
        if let Some(parent) = remote_parent(path) {
            self.mkdir_p(&parent).await?;
        }
        let command = format!("cat > {}", shell_quote(path));
        self.run_command_with_stdin_bytes(&command, bytes)
            .await
            .map(|_| ())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, TargetsError> {
        self.run_command_bytes(&format!("cat {}", shell_quote(path)))
            .await
    }

    pub async fn copy_dir(&self, source: &str, target: &str) -> Result<(), TargetsError> {
        let command = format!(
            "mkdir -p {target} && cp -R {source}/. {target}/",
            source = shell_quote(source),
            target = shell_quote(target)
        );
        self.run_command_with_control(
            &command,
            ProcessPolicy::bulk_transfer(),
            ProcessCancellation::Never,
        )
        .await
        .map(|_| ())
    }

    pub async fn remove_tree(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("rm -rf -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), TargetsError> {
        self.run_command(&format!("rm -f -- {}", shell_quote(path)))
            .await
            .map(|_| ())
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, TargetsError> {
        let command = format!(
            r#"d={dir}; for p in "$d"/* "$d"/.[!.]* "$d"/..?*; do [ -e "$p" ] || continue; name=${{p##*/}}; link=""; if [ -L "$p" ]; then kind="symlink"; link=$(readlink "$p" || true); elif [ -d "$p" ]; then kind="dir"; elif [ -f "$p" ]; then kind="file"; else kind="other"; fi; printf '%s\t%s\t%s\n' "$name" "$kind" "$link"; done"#,
            dir = shell_quote(path)
        );
        let output = self.run_command(&command).await?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let name = parts.next()?.to_string();
                let file_type = parts.next().unwrap_or("other").to_string();
                let symlink_target = parts
                    .next()
                    .map(str::to_string)
                    .filter(|value| !value.is_empty());
                Some(RemoteDirEntry {
                    name,
                    file_type,
                    symlink_target,
                })
            })
            .collect())
    }
}

pub(super) fn remote_script_command(args: &[&str]) -> String {
    let mut command = "sh -s --".to_string();
    for arg in args {
        command.push(' ');
        command.push_str(&shell_quote(arg));
    }
    command
}

pub use crate::paths::remote_join;

pub fn remote_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    let index = trimmed.rfind('/')?;
    if index == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..index].to_string())
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn remote_file_type_is_dir(debug_value: &str) -> bool {
    matches!(debug_value, "dir" | "Directory") || debug_value.contains("Directory")
}

pub(super) fn parse_ssh_probe_output(output: &str) -> Result<SshProbe, TargetsError> {
    let mut remote_home = None;
    let mut remote_os = None;
    let mut mkdir_ok = false;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("HOME\t") {
            let trimmed = value.trim();
            if trimmed.starts_with('/') {
                remote_home = Some(trimmed.to_string());
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("OS\t") {
            remote_os = Some(value.trim().to_string());
            continue;
        }
        if line.trim() == "MKDIR_OK" {
            mkdir_ok = true;
        }
    }

    let remote_home = remote_home.ok_or(TargetsError::ProbeHomeMissing)?;
    if !mkdir_ok {
        return Err(TargetsError::ProbeMkdirUnconfirmed);
    }

    Ok(SshProbe {
        remote_home,
        remote_os: remote_os.unwrap_or_else(|| "unknown".to_string()),
    })
}
