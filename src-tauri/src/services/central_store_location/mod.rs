//! Central store location migration service.
//!
//! Owns preview / apply of relocating the central skills directory: source
//! and target validation, skill directory copying, DB path rewrites, and
//! rebuilding symlinks that pointed into the old root. Tauri IPC shells live
//! in `crate::commands::central_store_location`.

mod error;

#[cfg(test)]
mod tests;

pub use error::CentralStoreLocationError;

use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::DbPool;
use crate::fs_util::run_blocking_fs_with;
use crate::services::installation::{copy_dir_all, create_symlink, symlink_target_path};
use crate::services::scanner::scan_all_skills_impl;
use crate::targets::ActiveTarget;

const CENTRAL_AGENT_ID: &str = "central";

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationPreview {
    pub source_path: String,
    pub target_path: String,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub skills_to_copy: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub skills_to_overwrite: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub target_only_skills: usize,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationSymlinkFailure {
    pub table: String,
    pub skill_id: String,
    pub owner_id: String,
    pub installed_path: String,
    pub error: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationChangeResult {
    pub source_path: String,
    pub target_path: String,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub copied: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub overwritten: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub target_only_imported: usize,
    #[cfg_attr(feature = "ipc-codegen", specta(type = specta_typescript::Number))]
    pub symlink_rebuild_failed: usize,
    pub symlink_failures: Vec<CentralStoreLocationSymlinkFailure>,
    pub completed_at: String,
}

/// Store relocation only operates on the local target; remote targets keep
/// their own central directory untouched.
pub fn ensure_local_target(target: &ActiveTarget) -> Result<(), CentralStoreLocationError> {
    match target {
        ActiveTarget::Local => Ok(()),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            Err(CentralStoreLocationError::UnsupportedTarget)
        }
    }
}

pub async fn preview_central_store_location_change_impl(
    pool: &DbPool,
    target_path: &str,
) -> Result<CentralStoreLocationPreview, CentralStoreLocationError> {
    let (source_root, target_root) = validated_roots(pool, target_path).await?;
    let source_root_for_scan = source_root.clone();
    let target_root_for_scan = target_root.clone();
    let (source_ids, target_ids) = run_blocking_fs_with(
        "central store preview scan",
        move || {
            Ok((
                skill_dir_ids(&source_root_for_scan)?,
                skill_dir_ids(&target_root_for_scan)?,
            ))
        },
        CentralStoreLocationError::task_join,
    )
    .await?;

    let skills_to_overwrite = source_ids.intersection(&target_ids).count();
    let skills_to_copy = source_ids.len().saturating_sub(skills_to_overwrite);
    let target_only_skills = target_ids.difference(&source_ids).count();

    Ok(CentralStoreLocationPreview {
        source_path: stored_path_string(&source_root),
        target_path: stored_path_string(&target_root),
        skills_to_copy,
        skills_to_overwrite,
        target_only_skills,
    })
}

pub async fn apply_central_store_location_change_impl(
    pool: &DbPool,
    target_path: &str,
    overwrite_existing: bool,
) -> Result<CentralStoreLocationChangeResult, CentralStoreLocationError> {
    if !overwrite_existing {
        return Err(CentralStoreLocationError::RequiresOverwrite);
    }

    preview_central_store_location_change_impl(pool, target_path).await?;
    let _mutation_guard = crate::services::central_mutation::acquire_central_mutation_guard(
        "central store relocation",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    let preview = preview_central_store_location_change_impl(pool, target_path).await?;
    let source_root = PathBuf::from(&preview.source_path);
    let target_root = PathBuf::from(&preview.target_path);

    let source_root_for_copy = source_root.clone();
    let target_root_for_copy = target_root.clone();
    let (copied, overwritten, created_skill_dirs) = run_blocking_fs_with(
        "central store relocation copy",
        move || {
            std::fs::create_dir_all(&target_root_for_copy).map_err(|e| {
                CentralStoreLocationError::io(
                    format!(
                        "Failed to create central store target '{}'",
                        target_root_for_copy.display()
                    ),
                    e,
                )
            })?;

            let mut copied = 0usize;
            let mut overwritten = 0usize;
            let mut created_skill_dirs = Vec::new();
            if source_root_for_copy.exists() {
                for entry in std::fs::read_dir(&source_root_for_copy).map_err(|e| {
                    CentralStoreLocationError::io(
                        format!(
                            "Failed to read central store '{}'",
                            source_root_for_copy.display()
                        ),
                        e,
                    )
                })? {
                    let entry = entry.map_err(|e| {
                        CentralStoreLocationError::io("Failed to read central skill entry", e)
                    })?;
                    let source_skill_dir = entry.path();
                    if !source_skill_dir.join("SKILL.md").exists() {
                        continue;
                    }
                    let target_skill_dir = target_root_for_copy.join(entry.file_name());
                    let existed = target_skill_dir.exists()
                        || std::fs::symlink_metadata(&target_skill_dir).is_ok();
                    if existed {
                        remove_existing_path(&target_skill_dir)?;
                    }
                    copy_dir_all(&source_skill_dir, &target_skill_dir)?;
                    if existed {
                        overwritten += 1;
                    } else {
                        copied += 1;
                        created_skill_dirs.push(target_skill_dir);
                    }
                }
            }
            Ok((copied, overwritten, created_skill_dirs))
        },
        CentralStoreLocationError::task_join,
    )
    .await?;

    if let Err(error) = update_central_root(pool, &source_root, &target_root).await {
        compensate_created_skill_dirs(created_skill_dirs).await?;
        return Err(error);
    }
    let symlink_failures =
        rebuild_symlinks_pointing_to_old_root(pool, &source_root, &target_root).await?;
    scan_all_skills_impl(pool).await?;

    Ok(CentralStoreLocationChangeResult {
        source_path: preview.source_path,
        target_path: preview.target_path,
        copied,
        overwritten,
        target_only_imported: preview.target_only_skills,
        symlink_rebuild_failed: symlink_failures.len(),
        symlink_failures,
        completed_at: Utc::now().to_rfc3339(),
    })
}

async fn validated_roots(
    pool: &DbPool,
    target_path: &str,
) -> Result<(PathBuf, PathBuf), CentralStoreLocationError> {
    let target_path = target_path.trim();
    if target_path.is_empty() {
        return Err(CentralStoreLocationError::EmptyPath);
    }

    let central = crate::db::get_agent_by_id(pool, CENTRAL_AGENT_ID)
        .await?
        .ok_or(CentralStoreLocationError::CentralAgentNotFound)?;
    let source_root = normalize_local_root(Path::new(&central.global_skills_dir))?;
    let target_root = normalize_local_root(&crate::paths::expand_home_path(target_path))?;

    if crate::paths::paths_equivalent(&source_root, &target_root) {
        return Err(CentralStoreLocationError::SamePath);
    }
    if is_nested_path(&source_root, &target_root) || is_nested_path(&target_root, &source_root) {
        return Err(CentralStoreLocationError::NestedPath);
    }

    Ok((source_root, target_root))
}

fn normalize_local_root(path: &Path) -> Result<PathBuf, CentralStoreLocationError> {
    if path.as_os_str().is_empty() {
        return Err(CentralStoreLocationError::EmptyPath);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| CentralStoreLocationError::io("Failed to resolve current directory", e))?
            .join(path)
    };
    Ok(crate::paths::canonicalize_path_with_missing(&absolute))
}

fn is_nested_path(parent: &Path, child: &Path) -> bool {
    let parent = equivalence_components(parent);
    let child = equivalence_components(child);
    child.len() > parent.len() && child.starts_with(&parent)
}

fn equivalence_components(path: &Path) -> Vec<String> {
    let value = crate::paths::normalize_stored_path(&path.to_string_lossy());
    #[cfg(windows)]
    let value = value.to_ascii_lowercase();
    value
        .split('/')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn skill_dir_ids(root: &Path) -> Result<HashSet<String>, CentralStoreLocationError> {
    let mut ids = HashSet::new();
    if !root.exists() {
        return Ok(ids);
    }
    let entries = std::fs::read_dir(root).map_err(|e| {
        CentralStoreLocationError::io(
            format!("Failed to read central store '{}'", root.display()),
            e,
        )
    })?;
    for entry in entries {
        let entry = entry
            .map_err(|e| CentralStoreLocationError::io("Failed to read central skill entry", e))?;
        let path = entry.path();
        if path.join("SKILL.md").exists() {
            ids.insert(entry.file_name().to_string_lossy().to_lowercase());
        }
    }
    Ok(ids)
}

fn sql_like_child_prefix(root: &str) -> String {
    let escaped = root
        .replace('#', "##")
        .replace('%', "#%")
        .replace('_', "#_");
    format!("{escaped}/%")
}

fn stored_path_prefix_len(root: &str) -> i64 {
    i64::try_from(root.len() + 1).expect("stored path length fits i64")
}

async fn compensate_created_skill_dirs(
    created_skill_dirs: Vec<PathBuf>,
) -> Result<(), CentralStoreLocationError> {
    if created_skill_dirs.is_empty() {
        return Ok(());
    }
    run_blocking_fs_with(
        "central store relocation compensate created dirs",
        move || {
            for dir in created_skill_dirs {
                if std::fs::symlink_metadata(&dir).is_ok() {
                    remove_existing_path(&dir)?;
                }
            }
            Ok(())
        },
        CentralStoreLocationError::task_join,
    )
    .await
}

async fn update_central_root(
    pool: &DbPool,
    old_root: &Path,
    new_root: &Path,
) -> Result<(), CentralStoreLocationError> {
    let old_root = stored_path_string(old_root);
    let new_root = stored_path_string(new_root);
    let like_child = sql_like_child_prefix(&old_root);
    let prefix_len = stored_path_prefix_len(&old_root);

    let mut transaction = pool.begin().await?;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
        .bind(&new_root)
        .bind(CENTRAL_AGENT_ID)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE scan_directories SET path = ? WHERE path = ? AND is_builtin = 1")
        .bind(&new_root)
        .bind(&old_root)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE skills
         SET file_path = CASE
               WHEN file_path = ? THEN ?
               WHEN file_path LIKE ? ESCAPE '#' THEN ? || substr(file_path, ?)
               ELSE file_path
             END,
             canonical_path = CASE
               WHEN canonical_path IS NULL THEN canonical_path
               WHEN canonical_path = ? THEN ?
               WHEN canonical_path LIKE ? ESCAPE '#' THEN ? || substr(canonical_path, ?)
               ELSE canonical_path
             END
         WHERE is_central = 1",
    )
    .bind(&old_root)
    .bind(&new_root)
    .bind(&like_child)
    .bind(&new_root)
    .bind(prefix_len)
    .bind(&old_root)
    .bind(&new_root)
    .bind(&like_child)
    .bind(&new_root)
    .bind(prefix_len)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE skill_installations
         SET installed_path = CASE
               WHEN installed_path = ? THEN ?
               WHEN installed_path LIKE ? ESCAPE '#' THEN ? || substr(installed_path, ?)
               ELSE installed_path
             END,
             symlink_target = CASE
               WHEN symlink_target IS NULL THEN symlink_target
               WHEN symlink_target = ? THEN ?
               WHEN symlink_target LIKE ? ESCAPE '#' THEN ? || substr(symlink_target, ?)
               ELSE symlink_target
             END
         WHERE agent_id = ? AND link_type = 'native'",
    )
    .bind(&old_root)
    .bind(&new_root)
    .bind(&like_child)
    .bind(&new_root)
    .bind(prefix_len)
    .bind(&old_root)
    .bind(&new_root)
    .bind(&like_child)
    .bind(&new_root)
    .bind(prefix_len)
    .bind(CENTRAL_AGENT_ID)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

#[derive(Debug)]
struct SymlinkRow {
    table: &'static str,
    skill_id: String,
    owner_id: String,
    installed_path: String,
    symlink_target: Option<String>,
}

async fn rebuild_symlinks_pointing_to_old_root(
    pool: &DbPool,
    old_root: &Path,
    new_root: &Path,
) -> Result<Vec<CentralStoreLocationSymlinkFailure>, CentralStoreLocationError> {
    let mut rows = Vec::new();
    for row in sqlx::query(
        "SELECT skill_id, agent_id AS owner_id, installed_path, symlink_target
         FROM skill_installations
         WHERE link_type = 'symlink' AND symlink_target IS NOT NULL",
    )
    .fetch_all(pool)
    .await?
    {
        rows.push(SymlinkRow {
            table: "skill_installations",
            skill_id: row.try_get("skill_id")?,
            owner_id: row.try_get("owner_id")?,
            installed_path: row.try_get("installed_path")?,
            symlink_target: row.try_get("symlink_target").ok(),
        });
    }
    for row in sqlx::query(
        "SELECT skill_id, project_id || ':' || agent_id AS owner_id, installed_path, symlink_target
         FROM project_skill_installations
         WHERE link_type = 'symlink' AND symlink_target IS NOT NULL",
    )
    .fetch_all(pool)
    .await?
    {
        rows.push(SymlinkRow {
            table: "project_skill_installations",
            skill_id: row.try_get("skill_id")?,
            owner_id: row.try_get("owner_id")?,
            installed_path: row.try_get("installed_path")?,
            symlink_target: row.try_get("symlink_target").ok(),
        });
    }

    let mut failures = Vec::new();
    for row in rows {
        let link_path = PathBuf::from(&row.installed_path);
        let Some(old_target) = row
            .symlink_target
            .as_deref()
            .map(PathBuf::from)
            .map(|target| resolve_symlink_target(&link_path, &target))
        else {
            continue;
        };
        if !is_child_or_same(old_root, &old_target) {
            continue;
        }
        let relative = old_target
            .strip_prefix(old_root)
            .map_err(|e| CentralStoreLocationError::PathPrefix(e.to_string()))?;
        let new_target = new_root.join(relative);
        let new_link_value = link_path
            .parent()
            .map(|parent| symlink_target_path(parent, &new_target))
            .unwrap_or_else(|| new_target.clone());

        let link_path_for_replace = link_path.clone();
        let new_link_value_for_replace = new_link_value.clone();
        let replace_result = run_blocking_fs_with(
            "central store symlink rebuild",
            move || replace_symlink(&link_path_for_replace, &new_link_value_for_replace),
            CentralStoreLocationError::task_join,
        )
        .await;
        if let Err(error) = replace_result {
            failures.push(CentralStoreLocationSymlinkFailure {
                table: row.table.to_string(),
                skill_id: row.skill_id,
                owner_id: row.owner_id,
                installed_path: row.installed_path,
                error: error.to_string(),
            });
            continue;
        }

        update_symlink_row(pool, &row, &new_link_value).await?;
    }

    Ok(failures)
}

fn resolve_symlink_target(link_path: &Path, raw_target: &Path) -> PathBuf {
    if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        link_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(raw_target)
    }
}

fn is_child_or_same(parent: &Path, child: &Path) -> bool {
    crate::paths::paths_equivalent(parent, child) || is_nested_path(parent, child)
}

fn replace_symlink(link_path: &Path, target: &Path) -> Result<(), CentralStoreLocationError> {
    let meta = std::fs::symlink_metadata(link_path).map_err(|e| {
        CentralStoreLocationError::io(
            format!("Failed to inspect symlink '{}'", link_path.display()),
            e,
        )
    })?;
    if !meta.file_type().is_symlink() {
        return Err(CentralStoreLocationError::NotASymlink(
            link_path.display().to_string(),
        ));
    }
    remove_existing_path(link_path)?;
    create_symlink(target, link_path)?;
    Ok(())
}

fn remove_existing_path(path: &Path) -> Result<(), CentralStoreLocationError> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| {
        CentralStoreLocationError::io(format!("Failed to inspect '{}'", path.display()), e)
    })?;
    if meta.file_type().is_symlink() {
        #[cfg(windows)]
        {
            return std::fs::remove_dir(path).map_err(|e| {
                CentralStoreLocationError::io(
                    format!("Failed to remove symlink '{}'", path.display()),
                    e,
                )
            });
        }
        #[cfg(not(windows))]
        {
            return std::fs::remove_file(path).map_err(|e| {
                CentralStoreLocationError::io(
                    format!("Failed to remove symlink '{}'", path.display()),
                    e,
                )
            });
        }
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| {
            CentralStoreLocationError::io(
                format!("Failed to remove directory '{}'", path.display()),
                e,
            )
        })
    } else {
        std::fs::remove_file(path).map_err(|e| {
            CentralStoreLocationError::io(format!("Failed to remove file '{}'", path.display()), e)
        })
    }
}

async fn update_symlink_row(
    pool: &DbPool,
    row: &SymlinkRow,
    target: &Path,
) -> Result<(), CentralStoreLocationError> {
    let target = stored_path_string(target);
    match row.table {
        "skill_installations" => sqlx::query(
            "UPDATE skill_installations
                 SET symlink_target = ?
                 WHERE skill_id = ? AND agent_id = ?",
        )
        .bind(target)
        .bind(&row.skill_id)
        .bind(&row.owner_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(CentralStoreLocationError::from),
        "project_skill_installations" => {
            let (project_id, agent_id) = row
                .owner_id
                .split_once(':')
                .ok_or(CentralStoreLocationError::InvalidSymlinkOwner)?;
            sqlx::query(
                "UPDATE project_skill_installations
                 SET symlink_target = ?
                 WHERE skill_id = ? AND project_id = ? AND agent_id = ?",
            )
            .bind(target)
            .bind(&row.skill_id)
            .bind(project_id)
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(CentralStoreLocationError::from)
        }
        _ => Ok(()),
    }
}

fn stored_path_string(path: &Path) -> String {
    crate::paths::normalize_stored_path(&crate::paths::path_to_string(path))
}
