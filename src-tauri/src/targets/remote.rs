use super::askpass::{open_ssh_target, ConnectedSshTarget};
use super::error::TargetsError;
use super::exec::{
    open_wsl_target, remote_symlink_allowed, wsl_symlink_allowed, ConnectedWslTarget,
    RemoteDirEntry, RemotePathInfo,
};
use super::model::ActiveTarget;

pub async fn connect_remote_target(
    active_target: &ActiveTarget,
) -> Result<ConnectedRemoteTarget, TargetsError> {
    match active_target {
        ActiveTarget::Local => Err(TargetsError::LocalTargetNotRemote),
        ActiveTarget::Ssh(target) => open_ssh_target(target).map(ConnectedRemoteTarget::Ssh),
        ActiveTarget::Wsl(target) => open_wsl_target(target).map(ConnectedRemoteTarget::Wsl),
    }
}

pub enum ConnectedRemoteTarget {
    Ssh(ConnectedSshTarget),
    Wsl(ConnectedWslTarget),
}

impl ConnectedRemoteTarget {
    pub fn active_target(&self) -> ActiveTarget {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                ActiveTarget::Ssh(Box::new(connection.target.clone()))
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                ActiveTarget::Wsl(Box::new(connection.target.clone()))
            }
        }
    }

    pub fn target_id(&self) -> &str {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.target.id.as_str(),
            ConnectedRemoteTarget::Wsl(connection) => connection.target.id.as_str(),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.target.label.as_str(),
            ConnectedRemoteTarget::Wsl(connection) => connection.target.label.as_str(),
        }
    }

    pub fn remote_home(&self) -> &str {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.target.remote_home.as_str(),
            ConnectedRemoteTarget::Wsl(connection) => connection.target.remote_home.as_str(),
        }
    }

    pub fn remote_os(&self) -> &str {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.target.remote_os.as_str(),
            ConnectedRemoteTarget::Wsl(connection) => connection.target.remote_os.as_str(),
        }
    }

    pub fn symlink_allowed(&self) -> bool {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => remote_symlink_allowed(&connection.target),
            ConnectedRemoteTarget::Wsl(connection) => wsl_symlink_allowed(&connection.target),
        }
    }

    pub async fn run_script(&self, script: &str, args: &[&str]) -> Result<String, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.run_script(script, args).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.run_script(script, args).await,
        }
    }

    pub(crate) async fn run_script_cancellable(
        &self,
        script: &str,
        args: &[&str],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<String, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                connection
                    .run_script_cancellable(script, args, cancel)
                    .await
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                connection
                    .run_script_cancellable(script, args, cancel)
                    .await
            }
        }
    }

    pub async fn run_command(&self, command: &str) -> Result<String, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.run_command(command).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.run_command(command).await,
        }
    }

    pub async fn run_command_with_stdin_bytes(
        &self,
        command: &str,
        stdin: &[u8],
    ) -> Result<Vec<u8>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                connection
                    .run_command_with_stdin_bytes(command, stdin)
                    .await
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                connection
                    .run_command_with_stdin_bytes(command, stdin)
                    .await
            }
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn run_command_with_stdin_bytes_cancellable(
        &self,
        command: &str,
        stdin: &[u8],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Vec<u8>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                connection
                    .run_command_with_stdin_bytes_cancellable(command, stdin, cancel)
                    .await
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                connection
                    .run_command_with_stdin_bytes_cancellable(command, stdin, cancel)
                    .await
            }
        }
    }

    pub async fn ensure_dir(&self, path: &str) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.ensure_dir(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.ensure_dir(path).await,
        }
    }

    pub async fn exists(&self, path: &str) -> Result<bool, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.exists(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.exists(path).await,
        }
    }

    pub async fn inspect_path(&self, path: &str) -> Result<Option<RemotePathInfo>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.inspect_path(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.inspect_path(path).await,
        }
    }

    pub async fn mkdir_p(&self, path: &str) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.mkdir_p(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.mkdir_p(path).await,
        }
    }

    pub async fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.write_file(path, bytes).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.write_file(path, bytes).await,
        }
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.read_file(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.read_file(path).await,
        }
    }

    pub async fn read_file_bounded(
        &self,
        path: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                connection.read_file_bounded(path, max_bytes).await
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                connection.read_file_bounded(path, max_bytes).await
            }
        }
    }

    pub async fn copy_dir(&self, source: &str, target: &str) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.copy_dir(source, target).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.copy_dir(source, target).await,
        }
    }

    pub async fn remove_tree(&self, path: &str) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.remove_tree(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.remove_tree(path).await,
        }
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.remove_file(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.remove_file(path).await,
        }
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, TargetsError> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.list_dir(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.list_dir(path).await,
        }
    }
}
