//! `projects` 和 `project_skill_installations` 两张表的 CRUD。

use std::collections::{HashMap, HashSet};

use sqlx::Row;

use crate::db::sqlite_batch::SQLITE_IN_QUERY_BATCH_SIZE;
use crate::db::types::{DbPool, Project, ProjectSkillInstallation};

/// One distinct project destination for a Central skill, plus `pinned` for sort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedProjectRow {
    pub skill_id: String,
    pub project_id: String,
    pub path: String,
    pub pinned: bool,
}

// ─── projects ────────────────────────────────────────────────────────────────

/// 插入新项目。调用方需自行规范化 path、计算 id。
pub async fn insert_project(pool: &DbPool, project: &Project) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO projects (id, path, name, pinned, added_at, last_scanned_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&project.id)
    .bind(&project.path)
    .bind(&project.name)
    .bind(project.pinned)
    .bind(&project.added_at)
    .bind(&project.last_scanned_at)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn get_project_by_id(pool: &DbPool, id: &str) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_project_by_path(
    pool: &DbPool,
    path: &str,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await
}

/// pinned 在前；同 pin 状态下 last_scanned_at 倒序，未扫描的排最后。
pub async fn list_projects(pool: &DbPool) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "SELECT * FROM projects
         ORDER BY pinned DESC,
                  (last_scanned_at IS NULL) ASC,
                  last_scanned_at DESC,
                  added_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn update_project_name(pool: &DbPool, id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_project_pinned(
    pool: &DbPool,
    id: &str,
    pinned: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET pinned = ? WHERE id = ?")
        .bind(pinned)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_project_path(pool: &DbPool, id: &str, path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET path = ? WHERE id = ?")
        .bind(path)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_project_last_scanned(
    pool: &DbPool,
    id: &str,
    last_scanned_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET last_scanned_at = ? WHERE id = ?")
        .bind(last_scanned_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// 删项目。`project_skill_installations` 通过 `ON DELETE CASCADE` 自动清理。
/// Production and test pools enable and verify foreign keys on every connection.
pub async fn delete_project(pool: &DbPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

// ─── project_skill_installations ─────────────────────────────────────────────

/// 插入或更新 psi 行。冲突时刷新 installed_path / link_type / symlink_target，
/// 保留原始 `created_at`。
pub async fn upsert_project_skill_installation(
    pool: &DbPool,
    psi: &ProjectSkillInstallation,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO project_skill_installations
         (project_id, skill_id, name, description, file_path, source_origin,
          agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(project_id, skill_id, agent_id) DO UPDATE SET
             installed_path = excluded.installed_path,
             name           = excluded.name,
             description    = excluded.description,
             file_path      = excluded.file_path,
             source_origin  = CASE
                 WHEN project_skill_installations.source_origin = 'central'
                      AND (
                          excluded.source_origin = 'central'
                          OR project_skill_installations.installed_path = excluded.installed_path
                      )
                 THEN 'central'
                 WHEN excluded.source_origin = 'central'
                 THEN 'central'
                 ELSE 'project'
             END,
             link_type      = excluded.link_type,
             symlink_target = excluded.symlink_target",
    )
    .bind(&psi.project_id)
    .bind(&psi.skill_id)
    .bind(&psi.name)
    .bind(&psi.description)
    .bind(&psi.file_path)
    .bind(&psi.source_origin)
    .bind(&psi.agent_id)
    .bind(&psi.installed_path)
    .bind(&psi.link_type)
    .bind(&psi.symlink_target)
    .bind(&psi.created_at)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Persist a full project scan in one transaction: upsert all observed rows,
/// delete stale rows for the project, and refresh `last_scanned_at`.
pub async fn persist_project_skill_scan(
    pool: &DbPool,
    project_id: &str,
    rows: &[ProjectSkillInstallation],
    last_scanned_at: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "CREATE TEMP TABLE IF NOT EXISTS project_scan_keep (
            project_id TEXT NOT NULL,
            skill_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            PRIMARY KEY (project_id, skill_id, agent_id)
         )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM project_scan_keep")
        .execute(&mut *tx)
        .await?;

    for psi in rows {
        sqlx::query(
            "INSERT INTO project_skill_installations
             (project_id, skill_id, name, description, file_path, source_origin,
              agent_id, installed_path, link_type, symlink_target, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(project_id, skill_id, agent_id) DO UPDATE SET
                 installed_path = excluded.installed_path,
                 name           = excluded.name,
                 description    = excluded.description,
                 file_path      = excluded.file_path,
                 source_origin  = CASE
                     WHEN project_skill_installations.source_origin = 'central'
                          AND (
                              excluded.source_origin = 'central'
                              OR project_skill_installations.installed_path = excluded.installed_path
                          )
                     THEN 'central'
                     WHEN excluded.source_origin = 'central'
                     THEN 'central'
                     ELSE 'project'
                 END,
                 link_type      = excluded.link_type,
                 symlink_target = excluded.symlink_target",
        )
        .bind(&psi.project_id)
        .bind(&psi.skill_id)
        .bind(&psi.name)
        .bind(&psi.description)
        .bind(&psi.file_path)
        .bind(&psi.source_origin)
        .bind(&psi.agent_id)
        .bind(&psi.installed_path)
        .bind(&psi.link_type)
        .bind(&psi.symlink_target)
        .bind(&psi.created_at)
        .execute(&mut *tx)
        .await
        ?;

        sqlx::query(
            "INSERT OR IGNORE INTO project_scan_keep (project_id, skill_id, agent_id)
             VALUES (?, ?, ?)",
        )
        .bind(&psi.project_id)
        .bind(&psi.skill_id)
        .bind(&psi.agent_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "DELETE FROM project_skill_installations
         WHERE project_id = ?
           AND NOT EXISTS (
             SELECT 1 FROM project_scan_keep keep
             WHERE keep.project_id = project_skill_installations.project_id
               AND keep.skill_id = project_skill_installations.skill_id
               AND keep.agent_id = project_skill_installations.agent_id
           )",
    )
    .bind(project_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE projects SET last_scanned_at = ? WHERE id = ?")
        .bind(last_scanned_at)
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}

pub async fn list_project_skill_installations(
    pool: &DbPool,
    project_id: &str,
) -> Result<Vec<ProjectSkillInstallation>, sqlx::Error> {
    sqlx::query_as::<_, ProjectSkillInstallation>(
        "SELECT * FROM project_skill_installations WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

pub async fn get_project_skill_installation(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
) -> Result<Option<ProjectSkillInstallation>, sqlx::Error> {
    sqlx::query_as::<_, ProjectSkillInstallation>(
        "SELECT * FROM project_skill_installations
         WHERE project_id = ? AND skill_id = ? AND agent_id = ?",
    )
    .bind(project_id)
    .bind(skill_id)
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_project_skill_installation(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM project_skill_installations
         WHERE project_id = ? AND skill_id = ? AND agent_id = ?",
    )
    .bind(project_id)
    .bind(skill_id)
    .bind(agent_id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// 扫描后用：删除 psi 中本项目下、且不在 `kept_keys` 集合里的孤儿行。
/// `kept_keys` 元素是 `(skill_id, agent_id)` 元组的字符串拼接，用 `\x1f` 隔离避免冲突。
pub async fn delete_stale_project_skill_installations(
    pool: &DbPool,
    project_id: &str,
    kept_pairs: &[(String, String)],
) -> Result<(), sqlx::Error> {
    if kept_pairs.is_empty() {
        return sqlx::query("DELETE FROM project_skill_installations WHERE project_id = ?")
            .bind(project_id)
            .execute(pool)
            .await
            .map(|_| ());
    }

    // 拉全量行，应用侧筛掉 kept 的，逐行删除。psi 表预期单项目下行数有限（<几千），
    // 这种简单方案足够，避免拼 `NOT IN (?,?,?...)` 的双列变体。
    let rows = sqlx::query(
        "SELECT skill_id, agent_id FROM project_skill_installations WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let kept: HashSet<(String, String)> = kept_pairs.iter().cloned().collect();
    for row in rows {
        let skill_id: String = row.try_get("skill_id")?;
        let agent_id: String = row.try_get("agent_id")?;
        if kept.contains(&(skill_id.clone(), agent_id.clone())) {
            continue;
        }
        delete_project_skill_installation(pool, project_id, &skill_id, &agent_id).await?;
    }
    Ok(())
}

/// Batch-read distinct project destinations for Central skill cards.
///
/// Empty `skill_ids` returns an empty map and issues no query. Dynamic `IN`
/// lists are chunked at [`SQLITE_IN_QUERY_BATCH_SIZE`]. Each skill's list is
/// unique by `project_id` and sorted pinned DESC, path basename
/// case-insensitive, then `project_id`.
pub async fn list_linked_projects_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, Vec<LinkedProjectRow>>, sqlx::Error> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut grouped: HashMap<String, Vec<LinkedProjectRow>> = HashMap::new();
    for chunk in skill_ids.chunks(SQLITE_IN_QUERY_BATCH_SIZE) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT psi.skill_id AS skill_id,
                    p.id AS project_id,
                    p.path AS path,
                    p.pinned AS pinned
               FROM project_skill_installations psi
               JOIN projects p ON p.id = psi.project_id
              WHERE psi.skill_id IN ({})",
            placeholders
        );
        let mut query = sqlx::query(&sql);
        for id in chunk {
            query = query.bind(id);
        }
        for row in query.fetch_all(pool).await? {
            let item = LinkedProjectRow {
                skill_id: row.try_get("skill_id")?,
                project_id: row.try_get("project_id")?,
                path: row.try_get("path")?,
                pinned: row.try_get("pinned")?,
            };
            let entries = grouped.entry(item.skill_id.clone()).or_default();
            if entries
                .iter()
                .any(|existing| existing.project_id == item.project_id)
            {
                continue;
            }
            entries.push(item);
        }
    }

    for entries in grouped.values_mut() {
        entries.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| {
                    path_basename_sort_key(&left.path).cmp(&path_basename_sort_key(&right.path))
                })
                .then_with(|| left.project_id.cmp(&right.project_id))
        });
    }

    Ok(grouped)
}

fn path_basename_sort_key(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    trimmed
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_lowercase()
}
