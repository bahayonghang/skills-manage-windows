//! `projects` 和 `project_skill_installations` 两张表的 CRUD。

use sqlx::Row;

use crate::db::types::{DbPool, Project, ProjectSkillInstallation};

// ─── projects ────────────────────────────────────────────────────────────────

/// 插入新项目。调用方需自行规范化 path、计算 id。
pub async fn insert_project(pool: &DbPool, project: &Project) -> Result<(), String> {
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
    .map_err(|e| e.to_string())
}

pub async fn get_project_by_id(pool: &DbPool, id: &str) -> Result<Option<Project>, String> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_project_by_path(pool: &DbPool, path: &str) -> Result<Option<Project>, String> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// pinned 在前；同 pin 状态下 last_scanned_at 倒序，未扫描的排最后。
pub async fn list_projects(pool: &DbPool) -> Result<Vec<Project>, String> {
    sqlx::query_as::<_, Project>(
        "SELECT * FROM projects
         ORDER BY pinned DESC,
                  (last_scanned_at IS NULL) ASC,
                  last_scanned_at DESC,
                  added_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_project_name(pool: &DbPool, id: &str, name: &str) -> Result<(), String> {
    sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn update_project_pinned(pool: &DbPool, id: &str, pinned: bool) -> Result<(), String> {
    sqlx::query("UPDATE projects SET pinned = ? WHERE id = ?")
        .bind(pinned)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn update_project_last_scanned(
    pool: &DbPool,
    id: &str,
    last_scanned_at: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE projects SET last_scanned_at = ? WHERE id = ?")
        .bind(last_scanned_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// 删项目。`project_skill_installations` 通过 `ON DELETE CASCADE` 自动清理。
/// 注意：SQLite 默认不开启外键，需要执行前 `PRAGMA foreign_keys = ON`，
/// 或调用方先显式删 psi。这里走显式删除避免依赖 PRAGMA 状态。
pub async fn delete_project(pool: &DbPool, id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM project_skill_installations WHERE project_id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ─── project_skill_installations ─────────────────────────────────────────────

/// 插入或更新 psi 行。冲突时刷新 installed_path / link_type / symlink_target，
/// 保留原始 `created_at`。
pub async fn upsert_project_skill_installation(
    pool: &DbPool,
    psi: &ProjectSkillInstallation,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO project_skill_installations
         (project_id, skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(project_id, skill_id, agent_id) DO UPDATE SET
             installed_path = excluded.installed_path,
             link_type      = excluded.link_type,
             symlink_target = excluded.symlink_target",
    )
    .bind(&psi.project_id)
    .bind(&psi.skill_id)
    .bind(&psi.agent_id)
    .bind(&psi.installed_path)
    .bind(&psi.link_type)
    .bind(&psi.symlink_target)
    .bind(&psi.created_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn list_project_skill_installations(
    pool: &DbPool,
    project_id: &str,
) -> Result<Vec<ProjectSkillInstallation>, String> {
    sqlx::query_as::<_, ProjectSkillInstallation>(
        "SELECT * FROM project_skill_installations WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_project_skill_installation(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
) -> Result<Option<ProjectSkillInstallation>, String> {
    sqlx::query_as::<_, ProjectSkillInstallation>(
        "SELECT * FROM project_skill_installations
         WHERE project_id = ? AND skill_id = ? AND agent_id = ?",
    )
    .bind(project_id)
    .bind(skill_id)
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn delete_project_skill_installation(
    pool: &DbPool,
    project_id: &str,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), String> {
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
    .map_err(|e| e.to_string())
}

/// 扫描后用：删除 psi 中本项目下、且不在 `kept_keys` 集合里的孤儿行。
/// `kept_keys` 元素是 `(skill_id, agent_id)` 元组的字符串拼接，用 `\x1f` 隔离避免冲突。
pub async fn delete_stale_project_skill_installations(
    pool: &DbPool,
    project_id: &str,
    kept_pairs: &[(String, String)],
) -> Result<(), String> {
    if kept_pairs.is_empty() {
        return sqlx::query(
            "DELETE FROM project_skill_installations WHERE project_id = ?",
        )
        .bind(project_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string());
    }

    // 拉全量行，应用侧筛掉 kept 的，逐行删除。psi 表预期单项目下行数有限（<几千），
    // 这种简单方案足够，避免拼 `NOT IN (?,?,?...)` 的双列变体。
    let rows = sqlx::query("SELECT skill_id, agent_id FROM project_skill_installations WHERE project_id = ?")
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let kept: std::collections::HashSet<(String, String)> = kept_pairs.iter().cloned().collect();
    for row in rows {
        let skill_id: String = row.try_get("skill_id").map_err(|e| e.to_string())?;
        let agent_id: String = row.try_get("agent_id").map_err(|e| e.to_string())?;
        if kept.contains(&(skill_id.clone(), agent_id.clone())) {
            continue;
        }
        delete_project_skill_installation(pool, project_id, &skill_id, &agent_id).await?;
    }
    Ok(())
}
