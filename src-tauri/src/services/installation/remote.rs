//! Remote (SSH/WSL) execution half of skill install / uninstall. Drives the
//! `REMOTE_CENTRAL_INSTALL_SCRIPT` shell snippet on a POSIX remote-like
//! target in a single round trip. The business orchestration lives in
//! `install.rs`; this module only implements the Remote arms of the
//! [`super::transport::InstallTransport`] hooks.

use crate::db::repos::installations_repo;
use crate::db::repos::skills_repo;
use crate::db::{self, DbPool};

use crate::targets::{remote_join, ConnectedRemoteTarget};

#[cfg(test)]
use crate::db::SkillInstallation;
#[cfg(test)]
use crate::targets::RemotePathInfo;

use super::error::InstallationError;
use super::transport::{transport_error, Placement, ResolvedMethod, SourceContext};

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

pub(crate) async fn ensure_remote_centralized(
    connection: &ConnectedRemoteTarget,
    pool: &DbPool,
    skill_id: &str,
    canonical_dir: &str,
) -> Result<(), InstallationError> {
    let canonical_skill_md = remote_join(canonical_dir, "SKILL.md");
    if connection
        .exists(&canonical_skill_md)
        .await
        .map_err(transport_error)?
    {
        return Ok(());
    }

    let skill = skills_repo::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| InstallationError::SkillNotFound(skill_id.to_string()))?;
    let source_dir = crate::targets::remote_parent(&skill.file_path)
        .ok_or_else(|| InstallationError::InvalidSkillFilePath(skill_id.to_string()))?;
    let source_skill_md = remote_join(&source_dir, "SKILL.md");
    if !connection
        .exists(&source_skill_md)
        .await
        .map_err(transport_error)?
    {
        return Err(InstallationError::SkillSourceMissing(source_skill_md));
    }

    connection
        .copy_dir(&source_dir, canonical_dir)
        .await
        .map_err(transport_error)?;

    let mut updated = skill;
    updated.canonical_path = Some(canonical_dir.to_string());
    updated.is_central = true;
    updated.file_path = canonical_skill_md;
    skills_repo::upsert_skill(pool, &updated).await?;

    Ok(())
}

pub(crate) fn remote_skill_source_dir(
    skill: &db::Skill,
    skill_id: &str,
) -> Result<String, InstallationError> {
    crate::targets::remote_parent(&skill.file_path)
        .ok_or_else(|| InstallationError::InvalidSkillFilePath(skill_id.to_string()))
}

async fn mark_remote_skill_centralized(
    pool: &DbPool,
    mut skill: db::Skill,
    canonical_dir: &str,
    canonical_skill_md: &str,
) -> Result<(), InstallationError> {
    skill.canonical_path = Some(canonical_dir.to_string());
    skill.is_central = true;
    skill.file_path = canonical_skill_md.to_string();
    Ok(skills_repo::upsert_skill(pool, &skill).await?)
}

async fn run_remote_central_install_script(
    connection: &ConnectedRemoteTarget,
    source_dir: &str,
    canonical_dir: &str,
    target_path: &str,
    agent_dir: &str,
    method: &str,
    managed_copy: bool,
) -> Result<(), InstallationError> {
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
        .map_err(transport_error)
        .map(|_| ())
}

/// Placement Remote arm: one atomic script round trip that centralizes,
/// clears the install slot, and lays down the symlink or copy; then marks
/// the skill centralized in the DB.
pub(crate) async fn place_install_remote(
    pool: &DbPool,
    connection: &ConnectedRemoteTarget,
    source: SourceContext,
    agent: &db::Agent,
    central: &db::Agent,
    skill_id: &str,
    method: ResolvedMethod,
) -> Result<Placement, InstallationError> {
    let SourceContext::Remote { skill, source_dir } = source else {
        return Err(InstallationError::Remote(
            "remote install invoked without a remote source context".to_string(),
        ));
    };

    let canonical_dir = remote_join(&central.global_skills_dir, skill_id);
    let canonical_skill_md = remote_join(&canonical_dir, "SKILL.md");
    let target_path = remote_join(&agent.global_skills_dir, skill_id);

    let method = match method {
        ResolvedMethod::Symlink => "symlink",
        ResolvedMethod::Copy | ResolvedMethod::Auto => "copy",
    };
    let installations = installations_repo::get_skill_installations(pool, skill_id).await?;
    let installation = installations
        .iter()
        .find(|record| record.agent_id == agent.id);
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
    mark_remote_skill_centralized(pool, *skill, &canonical_dir, &canonical_skill_md).await?;

    if method == "symlink" {
        return Ok(Placement {
            installed_path: target_path,
            link_type: "symlink",
            symlink_target: Some(canonical_dir),
        });
    }

    Ok(Placement {
        installed_path: target_path,
        link_type: "copy",
        symlink_target: None,
    })
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
