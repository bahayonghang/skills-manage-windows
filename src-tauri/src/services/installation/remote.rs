//! Remote (SSH) install/uninstall orchestration. Drives the
//! `REMOTE_CENTRAL_INSTALL_SCRIPT` shell snippet on the remote host through
//! a `ConnectedSshTarget`.

use crate::db::{self, DbPool, SkillInstallation};
use crate::targets::{
    connect_ssh_target, remote_join, remote_symlink_allowed, ConnectedSshTarget, RemoteTargetConfig,
};

#[cfg(test)]
use crate::targets::RemotePathInfo;

use super::types::InstallResult;

/// POSIX shell snippet executed on the remote host to centralize a skill,
/// clear the install slot, and lay down the symlink or copy.
pub(crate) const REMOTE_CENTRAL_INSTALL_SCRIPT: &str = r#"
set -eu

canonical_dir=$1
source_dir=$2
target_path=$3
agent_dir=$4
method=$5
managed_copy=$6

canonical_skill_md="$canonical_dir/SKILL.md"
source_skill_md="$source_dir/SKILL.md"

if [ ! -e "$canonical_skill_md" ]; then
  if [ ! -e "$source_skill_md" ]; then
    printf 'Skill source not found at %s\n' "$source_skill_md" >&2
    exit 42
  fi
  mkdir -p "$canonical_dir"
  cp -R "$source_dir/." "$canonical_dir/"
fi

if [ -L "$target_path" ]; then
  rm -f -- "$target_path"
elif [ -e "$target_path" ]; then
  if [ -d "$target_path" ] && [ "$method" = "symlink" ] && [ "$managed_copy" = "1" ]; then
    rm -rf -- "$target_path"
  else
    if [ -d "$target_path" ]; then
      entry_type=directory
    elif [ -f "$target_path" ]; then
      entry_type=file
    else
      entry_type=entry
    fi
    printf 'A remote %s already exists at '\''%s'\''. Uninstall the existing entry or delete it before installing with %s.\n' "$entry_type" "$target_path" "$method" >&2
    exit 43
  fi
fi

mkdir -p "$agent_dir"

if [ "$method" = "symlink" ]; then
  ln -s "$canonical_dir" "$target_path"
else
  mkdir -p "$target_path"
  cp -R "$canonical_dir/." "$target_path/"
fi
"#;

async fn record_remote_installation(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    installed_path: &str,
    link_type: &str,
    symlink_target: Option<String>,
) -> Result<InstallResult, String> {
    let installation = SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: installed_path.to_string(),
        link_type: link_type.to_string(),
        symlink_target,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_skill_installation(pool, &installation).await?;

    Ok(InstallResult {
        symlink_path: installed_path.to_string(),
    })
}

async fn ensure_remote_centralized(
    connection: &ConnectedSshTarget,
    pool: &DbPool,
    skill_id: &str,
    canonical_dir: &str,
) -> Result<(), String> {
    let canonical_skill_md = remote_join(canonical_dir, "SKILL.md");
    if connection.exists(&canonical_skill_md).await? {
        return Ok(());
    }

    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found in database", skill_id))?;
    let source_dir = crate::targets::remote_parent(&skill.file_path)
        .ok_or_else(|| format!("Invalid file_path for skill '{}'", skill_id))?;
    let source_skill_md = remote_join(&source_dir, "SKILL.md");
    if !connection.exists(&source_skill_md).await? {
        return Err(format!("Skill source not found at '{}'", source_skill_md));
    }

    connection.copy_dir(&source_dir, canonical_dir).await?;

    let mut updated = skill;
    updated.canonical_path = Some(canonical_dir.to_string());
    updated.is_central = true;
    updated.file_path = canonical_skill_md;
    db::upsert_skill(pool, &updated).await?;

    Ok(())
}

fn remote_skill_source_dir(skill: &db::Skill, skill_id: &str) -> Result<String, String> {
    crate::targets::remote_parent(&skill.file_path)
        .ok_or_else(|| format!("Invalid file_path for skill '{}'", skill_id))
}

async fn mark_remote_skill_centralized(
    pool: &DbPool,
    mut skill: db::Skill,
    canonical_dir: &str,
    canonical_skill_md: &str,
) -> Result<(), String> {
    skill.canonical_path = Some(canonical_dir.to_string());
    skill.is_central = true;
    skill.file_path = canonical_skill_md.to_string();
    db::upsert_skill(pool, &skill).await
}

async fn run_remote_central_install_script(
    connection: &ConnectedSshTarget,
    source_dir: &str,
    canonical_dir: &str,
    target_path: &str,
    agent_dir: &str,
    method: &str,
    managed_copy: bool,
) -> Result<(), String> {
    let managed_copy = if managed_copy { "1" } else { "0" };
    connection
        .run_script(
            REMOTE_CENTRAL_INSTALL_SCRIPT,
            &[
                canonical_dir,
                source_dir,
                target_path,
                agent_dir,
                method,
                managed_copy,
            ],
        )
        .await
        .map(|_| ())
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoteExistingInstallAction {
    UseEmptyPath,
    RemoveSymlink,
    RemoveManagedCopy,
    Reject(String),
}

#[cfg(test)]
pub(crate) fn classify_remote_existing_install_target(
    target_path: &str,
    method: &str,
    path_info: Option<&RemotePathInfo>,
    installation: Option<&SkillInstallation>,
) -> RemoteExistingInstallAction {
    let Some(path_info) = path_info else {
        return RemoteExistingInstallAction::UseEmptyPath;
    };

    if path_info.file_type == "symlink" {
        return RemoteExistingInstallAction::RemoveSymlink;
    }

    if path_info.file_type == "dir"
        && method == "symlink"
        && installation.is_some_and(|record| {
            record.link_type == "copy" && record.installed_path == target_path
        })
    {
        return RemoteExistingInstallAction::RemoveManagedCopy;
    }

    let entry_type = match path_info.file_type.as_str() {
        "dir" => "directory",
        "file" => "file",
        "symlink" => "symlink",
        _ => "entry",
    };
    RemoteExistingInstallAction::Reject(format!(
        "A remote {} already exists at '{}'. Uninstall the existing entry or delete it before installing with {}.",
        entry_type, target_path, method
    ))
}

pub async fn install_skill_to_agent_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<InstallResult, String> {
    let connection = connect_ssh_target(target).await?;
    install_skill_to_agent_ssh_with_connection(
        pool,
        &connection,
        target,
        skill_id,
        agent_id,
        method,
    )
    .await
}

pub(crate) async fn install_skill_to_agent_ssh_with_connection(
    pool: &DbPool,
    connection: &ConnectedSshTarget,
    target: &RemoteTargetConfig,
    skill_id: &str,
    agent_id: &str,
    method: &str,
) -> Result<InstallResult, String> {
    if agent_id == "central" {
        return Err("Cannot install a skill to the central agent itself".to_string());
    }

    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let canonical_dir = remote_join(&central.global_skills_dir, skill_id);
    let canonical_skill_md = remote_join(&canonical_dir, "SKILL.md");
    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found in database", skill_id))?;
    let source_dir = remote_skill_source_dir(&skill, skill_id)?;

    if agent.global_skills_dir == central.global_skills_dir {
        ensure_remote_centralized(connection, pool, skill_id, &canonical_dir).await?;

        return record_remote_installation(
            pool,
            skill_id,
            agent_id,
            &canonical_dir,
            "native",
            None,
        )
        .await;
    }

    let target_path = remote_join(&agent.global_skills_dir, skill_id);
    let method = if method == "symlink" {
        "symlink"
    } else {
        "copy"
    };
    if method == "symlink" && !remote_symlink_allowed(target) {
        return Err("Remote symlink install is disabled for this target.".to_string());
    }
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let installation = installations
        .iter()
        .find(|record| record.agent_id == agent_id);
    let managed_copy = installation
        .is_some_and(|record| record.link_type == "copy" && record.installed_path == target_path);

    run_remote_central_install_script(
        connection,
        &source_dir,
        &canonical_dir,
        &target_path,
        &agent.global_skills_dir,
        method,
        managed_copy,
    )
    .await?;
    mark_remote_skill_centralized(pool, skill, &canonical_dir, &canonical_skill_md).await?;

    if method == "symlink" {
        return record_remote_installation(
            pool,
            skill_id,
            agent_id,
            &target_path,
            "symlink",
            Some(canonical_dir),
        )
        .await;
    }

    record_remote_installation(pool, skill_id, agent_id, &target_path, "copy", None).await
}

pub async fn uninstall_skill_from_agent_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    let agent = db::get_agent_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;

    if agent_id == "central" || agent.global_skills_dir == central.global_skills_dir {
        return Err(format!(
            "{} shares the Central Skills directory and cannot be uninstalled independently.",
            agent.display_name
        ));
    }

    let install_path = remote_join(&agent.global_skills_dir, skill_id);
    let connection = connect_ssh_target(target).await?;
    connection.remove_tree(&install_path).await?;
    db::delete_skill_installation(pool, skill_id, agent_id).await
}
