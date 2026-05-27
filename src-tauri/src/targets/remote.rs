use super::askpass::{connect_ssh_target, ConnectedSshTarget};
use super::exec::{
    connect_wsl_target, remote_symlink_allowed, wsl_symlink_allowed, ConnectedWslTarget,
    RemoteDirEntry, RemotePathInfo,
};
use super::model::ActiveTarget;

pub async fn connect_remote_target(
    active_target: &ActiveTarget,
) -> Result<ConnectedRemoteTarget, String> {
    match active_target {
        ActiveTarget::Local => Err("Local target is not a remote target.".to_string()),
        ActiveTarget::Ssh(target) => connect_ssh_target(target)
            .await
            .map(ConnectedRemoteTarget::Ssh),
        ActiveTarget::Wsl(target) => connect_wsl_target(target)
            .await
            .map(ConnectedRemoteTarget::Wsl),
    }
}

pub enum ConnectedRemoteTarget {
    Ssh(ConnectedSshTarget),
    Wsl(ConnectedWslTarget),
}

impl ConnectedRemoteTarget {
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

    pub async fn run_script(&self, script: &str, args: &[&str]) -> Result<String, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.run_script(script, args).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.run_script(script, args).await,
        }
    }

    pub async fn run_command(&self, command: &str) -> Result<String, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.run_command(command).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.run_command(command).await,
        }
    }

    pub fn run_command_with_stdin_bytes(
        &self,
        command: &str,
        stdin: &[u8],
    ) -> Result<Vec<u8>, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => {
                connection.run_command_with_stdin_bytes(command, stdin)
            }
            ConnectedRemoteTarget::Wsl(connection) => {
                connection.run_command_with_stdin_bytes(command, stdin)
            }
        }
    }

    pub async fn ensure_dir(&self, path: &str) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.ensure_dir(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.ensure_dir(path).await,
        }
    }

    pub async fn exists(&self, path: &str) -> Result<bool, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.exists(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.exists(path).await,
        }
    }

    pub async fn inspect_path(&self, path: &str) -> Result<Option<RemotePathInfo>, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.inspect_path(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.inspect_path(path).await,
        }
    }

    pub async fn mkdir_p(&self, path: &str) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.mkdir_p(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.mkdir_p(path).await,
        }
    }

    pub async fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.write_file(path, bytes).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.write_file(path, bytes).await,
        }
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.read_file(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.read_file(path).await,
        }
    }

    pub async fn copy_dir(&self, source: &str, target: &str) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.copy_dir(source, target).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.copy_dir(source, target).await,
        }
    }

    pub async fn remove_tree(&self, path: &str) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.remove_tree(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.remove_tree(path).await,
        }
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.remove_file(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.remove_file(path).await,
        }
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirEntry>, String> {
        match self {
            ConnectedRemoteTarget::Ssh(connection) => connection.list_dir(path).await,
            ConnectedRemoteTarget::Wsl(connection) => connection.list_dir(path).await,
        }
    }
}
