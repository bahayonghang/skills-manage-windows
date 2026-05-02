use crate::db::{self, DbPool};
use crate::targets::{connect_ssh_target, ActiveTarget};

pub async fn read_skill_content_for_target_impl(
    pool: &DbPool,
    active_target: ActiveTarget,
    skill_id: &str,
) -> Result<String, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    match active_target {
        ActiveTarget::Local => read_skill_file_content(&skill.file_path),
        ActiveTarget::Ssh(target) => {
            let connection = connect_ssh_target(&target).await?;
            let bytes = connection.read_file(&skill.file_path).await?;
            String::from_utf8(bytes).map_err(|e| {
                format!(
                    "Remote file '{}' is not valid UTF-8: {}",
                    skill.file_path, e
                )
            })
        }
    }
}

fn read_skill_file_content(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

pub async fn read_file_by_path_for_target_impl(
    active_target: ActiveTarget,
    path: &str,
) -> Result<String, String> {
    match active_target {
        ActiveTarget::Local => read_file_by_path_impl(path),
        ActiveTarget::Ssh(target) => {
            let connection = connect_ssh_target(&target).await?;
            let bytes = connection.read_file(path).await?;
            String::from_utf8(bytes)
                .map_err(|e| format!("Remote file '{}' is not valid UTF-8: {}", path, e))
        }
    }
}

pub(super) fn read_file_by_path_impl(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

pub fn open_in_file_manager_for_target_impl(
    active_target: ActiveTarget,
    path: &str,
) -> Result<(), String> {
    if matches!(active_target, ActiveTarget::Ssh(_)) {
        return Err("Remote paths cannot be opened in the local file manager. Copy the remote path instead.".to_string());
    }
    open_in_file_manager_checked_impl(path)
}

pub(super) fn open_in_file_manager_checked_impl(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    open_in_file_manager_impl(path)
}

fn open_in_file_manager_impl(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }

    Ok(())
}
