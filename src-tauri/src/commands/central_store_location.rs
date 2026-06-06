use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::State;

use crate::db::DbPool;
use crate::services::installation::{copy_dir_all, create_symlink, symlink_target_path};
use crate::services::scanner::scan_all_skills_impl;
use crate::targets::ActiveTarget;
use crate::AppState;

const CENTRAL_AGENT_ID: &str = "central";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationPreviewRequest {
    pub target_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationApplyRequest {
    pub target_path: String,
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationPreview {
    pub source_path: String,
    pub target_path: String,
    pub skills_to_copy: usize,
    pub skills_to_overwrite: usize,
    pub target_only_skills: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationSymlinkFailure {
    pub table: String,
    pub skill_id: String,
    pub owner_id: String,
    pub installed_path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationChangeResult {
    pub source_path: String,
    pub target_path: String,
    pub copied: usize,
    pub overwritten: usize,
    pub target_only_imported: usize,
    pub symlink_rebuild_failed: usize,
    pub symlink_failures: Vec<CentralStoreLocationSymlinkFailure>,
    pub completed_at: String,
}

#[tauri::command]
pub async fn preview_central_store_location_change(
    state: State<'_, AppState>,
    request: CentralStoreLocationPreviewRequest,
) -> Result<CentralStoreLocationPreview, String> {
    ensure_local_target(&state).await?;
    let pool = state.active_db().await?;
    preview_central_store_location_change_impl(&pool, &request.target_path).await
}

#[tauri::command]
pub async fn apply_central_store_location_change(
    state: State<'_, AppState>,
    request: CentralStoreLocationApplyRequest,
) -> Result<CentralStoreLocationChangeResult, String> {
    ensure_local_target(&state).await?;
    let pool = state.active_db().await?;
    apply_central_store_location_change_impl(
        &pool,
        &request.target_path,
        request.overwrite_existing,
    )
    .await
}

async fn ensure_local_target(state: &AppState) -> Result<(), String> {
    match state.active_target().await? {
        ActiveTarget::Local => Ok(()),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            Err("central_store_location_unsupported_target".to_string())
        }
    }
}

pub async fn preview_central_store_location_change_impl(
    pool: &DbPool,
    target_path: &str,
) -> Result<CentralStoreLocationPreview, String> {
    let (source_root, target_root) = validated_roots(pool, target_path).await?;
    let source_ids = skill_dir_ids(&source_root)?;
    let target_ids = skill_dir_ids(&target_root)?;

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
) -> Result<CentralStoreLocationChangeResult, String> {
    if !overwrite_existing {
        return Err("central_store_location_requires_overwrite".to_string());
    }

    let preview = preview_central_store_location_change_impl(pool, target_path).await?;
    let source_root = PathBuf::from(&preview.source_path);
    let target_root = PathBuf::from(&preview.target_path);
    std::fs::create_dir_all(&target_root).map_err(|e| {
        format!(
            "Failed to create central store target '{}': {}",
            target_root.display(),
            e
        )
    })?;

    let mut copied = 0usize;
    let mut overwritten = 0usize;
    if source_root.exists() {
        for entry in std::fs::read_dir(&source_root).map_err(|e| {
            format!(
                "Failed to read central store '{}': {}",
                source_root.display(),
                e
            )
        })? {
            let entry = entry.map_err(|e| format!("Failed to read central skill entry: {}", e))?;
            let source_skill_dir = entry.path();
            if !source_skill_dir.join("SKILL.md").exists() {
                continue;
            }
            let target_skill_dir = target_root.join(entry.file_name());
            let existed =
                target_skill_dir.exists() || std::fs::symlink_metadata(&target_skill_dir).is_ok();
            if existed {
                remove_existing_path(&target_skill_dir)?;
            }
            copy_dir_all(&source_skill_dir, &target_skill_dir)?;
            if existed {
                overwritten += 1;
            } else {
                copied += 1;
            }
        }
    }

    update_central_root(pool, &source_root, &target_root).await?;
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

async fn validated_roots(pool: &DbPool, target_path: &str) -> Result<(PathBuf, PathBuf), String> {
    let target_path = target_path.trim();
    if target_path.is_empty() {
        return Err("central_store_location_empty_path".to_string());
    }

    let central = crate::db::get_agent_by_id(pool, CENTRAL_AGENT_ID)
        .await?
        .ok_or_else(|| "Central agent not found".to_string())?;
    let source_root = normalize_local_root(Path::new(&central.global_skills_dir))?;
    let target_root = normalize_local_root(&crate::paths::expand_home_path(target_path))?;

    if crate::paths::paths_equivalent(&source_root, &target_root) {
        return Err("central_store_location_same_path".to_string());
    }
    if is_nested_path(&source_root, &target_root) || is_nested_path(&target_root, &source_root) {
        return Err("central_store_location_nested_path".to_string());
    }

    Ok((source_root, target_root))
}

fn normalize_local_root(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("central_store_location_empty_path".to_string());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to resolve current directory: {}", e))?
            .join(path)
    };
    Ok(absolute.canonicalize().unwrap_or(absolute))
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

fn skill_dir_ids(root: &Path) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    if !root.exists() {
        return Ok(ids);
    }
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("Failed to read central store '{}': {}", root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read central skill entry: {}", e))?;
        let path = entry.path();
        if path.join("SKILL.md").exists() {
            ids.insert(entry.file_name().to_string_lossy().to_lowercase());
        }
    }
    Ok(ids)
}

async fn update_central_root(
    pool: &DbPool,
    old_root: &Path,
    new_root: &Path,
) -> Result<(), String> {
    let old_root = stored_path_string(old_root);
    let new_root = stored_path_string(new_root);
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
        .bind(&new_root)
        .bind(CENTRAL_AGENT_ID)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE scan_directories SET path = ? WHERE path = ? AND is_builtin = 1")
        .bind(&new_root)
        .bind(&old_root)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE skills
         SET file_path = REPLACE(file_path, ?, ?),
             canonical_path = CASE
               WHEN canonical_path IS NULL THEN canonical_path
               ELSE REPLACE(canonical_path, ?, ?)
             END
         WHERE is_central = 1",
    )
    .bind(&old_root)
    .bind(&new_root)
    .bind(&old_root)
    .bind(&new_root)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE skill_installations
         SET installed_path = REPLACE(installed_path, ?, ?),
             symlink_target = CASE
               WHEN symlink_target IS NULL THEN symlink_target
               ELSE REPLACE(symlink_target, ?, ?)
             END
         WHERE agent_id = ? AND link_type = 'native'",
    )
    .bind(&old_root)
    .bind(&new_root)
    .bind(&old_root)
    .bind(&new_root)
    .bind(CENTRAL_AGENT_ID)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
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
) -> Result<Vec<CentralStoreLocationSymlinkFailure>, String> {
    let mut rows = Vec::new();
    for row in sqlx::query(
        "SELECT skill_id, agent_id AS owner_id, installed_path, symlink_target
         FROM skill_installations
         WHERE link_type = 'symlink' AND symlink_target IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?
    {
        rows.push(SymlinkRow {
            table: "skill_installations",
            skill_id: row.try_get("skill_id").map_err(|e| e.to_string())?,
            owner_id: row.try_get("owner_id").map_err(|e| e.to_string())?,
            installed_path: row.try_get("installed_path").map_err(|e| e.to_string())?,
            symlink_target: row.try_get("symlink_target").ok(),
        });
    }
    for row in sqlx::query(
        "SELECT skill_id, project_id || ':' || agent_id AS owner_id, installed_path, symlink_target
         FROM project_skill_installations
         WHERE link_type = 'symlink' AND symlink_target IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?
    {
        rows.push(SymlinkRow {
            table: "project_skill_installations",
            skill_id: row.try_get("skill_id").map_err(|e| e.to_string())?,
            owner_id: row.try_get("owner_id").map_err(|e| e.to_string())?,
            installed_path: row.try_get("installed_path").map_err(|e| e.to_string())?,
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
            .map_err(|e| e.to_string())?;
        let new_target = new_root.join(relative);
        let new_link_value = link_path
            .parent()
            .map(|parent| symlink_target_path(parent, &new_target))
            .unwrap_or_else(|| new_target.clone());

        if let Err(error) = replace_symlink(&link_path, &new_link_value) {
            failures.push(CentralStoreLocationSymlinkFailure {
                table: row.table.to_string(),
                skill_id: row.skill_id,
                owner_id: row.owner_id,
                installed_path: row.installed_path,
                error,
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

fn replace_symlink(link_path: &Path, target: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(link_path)
        .map_err(|e| format!("Failed to inspect symlink '{}': {}", link_path.display(), e))?;
    if !meta.file_type().is_symlink() {
        return Err(format!("'{}' is not a symlink", link_path.display()));
    }
    remove_existing_path(link_path)?;
    create_symlink(target, link_path)
}

fn remove_existing_path(path: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| format!("Failed to inspect '{}': {}", path.display(), e))?;
    if meta.file_type().is_symlink() {
        #[cfg(windows)]
        {
            return std::fs::remove_dir(path)
                .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e));
        }
        #[cfg(not(windows))]
        {
            return std::fs::remove_file(path)
                .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e));
        }
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory '{}': {}", path.display(), e))
    } else {
        std::fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file '{}': {}", path.display(), e))
    }
}

async fn update_symlink_row(pool: &DbPool, row: &SymlinkRow, target: &Path) -> Result<(), String> {
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
        .map_err(|e| e.to_string()),
        "project_skill_installations" => {
            let (project_id, agent_id) = row
                .owner_id
                .split_once(':')
                .ok_or_else(|| "Invalid project symlink owner id".to_string())?;
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
            .map_err(|e| e.to_string())
        }
        _ => Ok(()),
    }
}

fn stored_path_string(path: &Path) -> String {
    crate::paths::normalize_stored_path(&crate::paths::path_to_string(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, Skill, SkillInstallation};
    use tempfile::TempDir;

    async fn setup() -> DbPool {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    async fn set_central_root(pool: &DbPool, root: &Path) {
        let path = root.to_string_lossy().into_owned();
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(&path)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT OR IGNORE INTO scan_directories (path, label, is_active, is_builtin, added_at)
             VALUES (?, 'Central Skills', 1, 1, ?)",
        )
        .bind(path)
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .unwrap();
    }

    fn write_skill(root: &Path, id: &str, marker: &str) {
        let dir = root.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {id}\ndescription: {marker}\n---\n\n# {id}\n"),
        )
        .unwrap();
    }

    fn skill_row(root: &Path, id: &str) -> Skill {
        let dir = root.join(id);
        Skill {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            file_path: dir.join("SKILL.md").to_string_lossy().into_owned(),
            canonical_path: Some(dir.to_string_lossy().into_owned()),
            is_central: true,
            source: Some("native".to_string()),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        }
    }

    #[tokio::test]
    async fn central_store_location_preview_counts_copy_overwrite_and_target_only() {
        let pool = setup().await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("old");
        let target = temp.path().join("new");
        write_skill(&source, "copy-me", "source");
        write_skill(&source, "overwrite-me", "source");
        write_skill(&target, "overwrite-me", "target");
        write_skill(&target, "target-only", "target");
        set_central_root(&pool, &source).await;

        let preview = preview_central_store_location_change_impl(&pool, &target.to_string_lossy())
            .await
            .unwrap();

        assert_eq!(preview.skills_to_copy, 1);
        assert_eq!(preview.skills_to_overwrite, 1);
        assert_eq!(preview.target_only_skills, 1);
    }

    #[tokio::test]
    async fn central_store_location_apply_overwrites_preserves_old_and_imports_target_only() {
        let pool = setup().await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("old");
        let target = temp.path().join("new");
        write_skill(&source, "same", "from-source");
        write_skill(&source, "source-only", "from-source");
        write_skill(&target, "same", "from-target");
        write_skill(&target, "target-only", "from-target");
        set_central_root(&pool, &source).await;
        db::upsert_skill(&pool, &skill_row(&source, "same"))
            .await
            .unwrap();

        let result =
            apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
                .await
                .unwrap();

        assert_eq!(result.copied, 1);
        assert_eq!(result.overwritten, 1);
        assert_eq!(result.target_only_imported, 1);
        assert!(source.join("same").join("SKILL.md").exists());
        let overwritten = std::fs::read_to_string(target.join("same").join("SKILL.md")).unwrap();
        assert!(overwritten.contains("from-source"));
        assert!(target.join("target-only").join("SKILL.md").exists());

        let central = db::get_agent_by_id(&pool, "central")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(central.global_skills_dir, stored_path_string(&target));
        let skills = db::get_central_skills(&pool).await.unwrap();
        assert!(skills.iter().any(|skill| skill.id == "target-only"));
    }

    #[tokio::test]
    async fn central_store_location_rejects_nested_and_same_paths() {
        let pool = setup().await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("old");
        std::fs::create_dir_all(&source).unwrap();
        set_central_root(&pool, &source).await;

        let same = preview_central_store_location_change_impl(&pool, &source.to_string_lossy())
            .await
            .unwrap_err();
        assert_eq!(same, "central_store_location_same_path");

        let nested = source.join("child");
        let nested_err =
            preview_central_store_location_change_impl(&pool, &nested.to_string_lossy())
                .await
                .unwrap_err();
        assert_eq!(nested_err, "central_store_location_nested_path");
    }

    #[tokio::test]
    async fn central_store_location_updates_existing_native_installation_paths() {
        let pool = setup().await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("old");
        let target = temp.path().join("new");
        write_skill(&source, "native-skill", "source");
        set_central_root(&pool, &source).await;
        db::upsert_skill(&pool, &skill_row(&source, "native-skill"))
            .await
            .unwrap();
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "native-skill".to_string(),
                agent_id: "central".to_string(),
                installed_path: source.join("native-skill").to_string_lossy().into_owned(),
                link_type: "native".to_string(),
                symlink_target: None,
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();

        apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
            .await
            .unwrap();

        let rows = db::get_skill_installations(&pool, "native-skill")
            .await
            .unwrap();
        assert!(rows.iter().any(|row| {
            row.agent_id == "central"
                && crate::paths::paths_equivalent(
                    Path::new(&row.installed_path),
                    &target.join("native-skill"),
                )
        }));
    }
}
