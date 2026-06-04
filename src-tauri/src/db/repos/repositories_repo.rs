//! `skill_repositories` and `skill_repository_members` CRUD — Phase 2c.
//!
//! `LOCAL_UNKNOWN_REPOSITORY_ID` is the system-managed sink for skills whose
//! source can't be identified; it must never be deleted.

use std::collections::HashMap;

use sqlx::Row;
use uuid::Uuid;

use crate::db::repos::skills_repo::upsert_skill_in_transaction;
use crate::db::types::{
    DbPool, Skill, SkillRepository, SkillRepositoryAssignment, SkillRepositoryMember,
    SkillRepositorySyncSkip, SkillRepositoryWithStats, LOCAL_UNKNOWN_REPOSITORY_ID,
};
use crate::db::util::now_rfc3339;

/// Lowercase + slug-safe normalization shared with `tags_repo` for ID derivation.
pub(crate) fn normalize_repository_component(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn github_repository_id(owner: &str, repo: &str, branch: &str) -> String {
    format!(
        "github:{}-{}-{}",
        normalize_repository_component(owner),
        normalize_repository_component(repo),
        normalize_repository_component(branch)
    )
}

pub async fn get_local_unknown_repository(pool: &DbPool) -> Result<SkillRepository, String> {
    get_skill_repository_by_id(pool, LOCAL_UNKNOWN_REPOSITORY_ID)
        .await?
        .ok_or_else(|| "Local unknown repository metadata is not initialized".to_string())
}

pub async fn get_skill_repository_by_id(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<SkillRepository>, String> {
    sqlx::query_as::<_, SkillRepository>("SELECT * FROM skill_repositories WHERE id = ?")
        .bind(repository_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_central_skill_ids_by_repository(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Vec<String>, String> {
    sqlx::query_scalar::<_, String>(
        "SELECT s.id
         FROM skill_repository_members m
         JOIN skills s ON s.id = m.skill_id
         WHERE m.repository_id = ? AND s.is_central = 1
         ORDER BY s.name, s.id",
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_central_repository_members_by_repositories(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<Vec<SkillRepositoryMember>, String> {
    if repository_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repository_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT
            m.skill_id AS skill_id,
            m.source_path AS source_path,
            r.id AS repository_id,
            r.name AS repository_name,
            r.source_type AS repository_source_type,
            r.owner AS repository_owner,
            r.repo AS repository_repo,
            r.branch AS repository_branch,
            r.url AS repository_url,
            r.pinned AS repository_pinned,
            r.is_unknown AS repository_is_unknown,
            r.created_at AS repository_created_at,
            r.updated_at AS repository_updated_at,
            r.last_synced_at AS repository_last_synced_at
         FROM skill_repository_members m
         JOIN skill_repositories r ON r.id = m.repository_id
         JOIN skills s ON s.id = m.skill_id
         WHERE m.repository_id IN ({})
           AND s.is_central = 1
           AND r.id <> ?
           AND r.is_unknown = 0
         ORDER BY r.name, s.name, s.id",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for repository_id in repository_ids {
        query = query.bind(repository_id);
    }
    query = query.bind(LOCAL_UNKNOWN_REPOSITORY_ID);

    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let mut members = Vec::with_capacity(rows.len());
    for row in rows {
        let repository = SkillRepository {
            id: row.try_get("repository_id").map_err(|e| e.to_string())?,
            name: row.try_get("repository_name").map_err(|e| e.to_string())?,
            source_type: row
                .try_get("repository_source_type")
                .map_err(|e| e.to_string())?,
            owner: row.try_get("repository_owner").map_err(|e| e.to_string())?,
            repo: row.try_get("repository_repo").map_err(|e| e.to_string())?,
            branch: row
                .try_get("repository_branch")
                .map_err(|e| e.to_string())?,
            url: row.try_get("repository_url").map_err(|e| e.to_string())?,
            pinned: row
                .try_get("repository_pinned")
                .map_err(|e| e.to_string())?,
            is_unknown: row
                .try_get("repository_is_unknown")
                .map_err(|e| e.to_string())?,
            created_at: row
                .try_get("repository_created_at")
                .map_err(|e| e.to_string())?,
            updated_at: row
                .try_get("repository_updated_at")
                .map_err(|e| e.to_string())?,
            last_synced_at: row
                .try_get("repository_last_synced_at")
                .map_err(|e| e.to_string())?,
        };
        members.push(SkillRepositoryMember {
            skill_id: row.try_get("skill_id").map_err(|e| e.to_string())?,
            source_path: row.try_get("source_path").map_err(|e| e.to_string())?,
            repository,
        });
    }

    Ok(members)
}

pub async fn get_skill_repository_sync_skips(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<Vec<SkillRepositorySyncSkip>, String> {
    if repository_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repository_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT *
         FROM skill_repository_sync_skips
         WHERE repository_id IN ({})
         ORDER BY repository_id, source_path",
        placeholders
    );
    let mut query = sqlx::query_as::<_, SkillRepositorySyncSkip>(&sql);
    for repository_id in repository_ids {
        query = query.bind(repository_id);
    }

    query.fetch_all(pool).await.map_err(|e| e.to_string())
}

pub async fn upsert_skill_repository_sync_skip(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
    skill_id: &str,
    skill_name: &str,
) -> Result<SkillRepositorySyncSkip, String> {
    let now = now_rfc3339();
    sqlx::query(
        "INSERT INTO skill_repository_sync_skips
         (repository_id, source_path, skill_id, skill_name, created_at, updated_at, last_seen_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(repository_id, source_path) DO UPDATE SET
           skill_id = excluded.skill_id,
           skill_name = excluded.skill_name,
           updated_at = excluded.updated_at,
           last_seen_at = excluded.last_seen_at",
    )
    .bind(repository_id)
    .bind(source_path)
    .bind(skill_id)
    .bind(skill_name)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, SkillRepositorySyncSkip>(
        "SELECT *
         FROM skill_repository_sync_skips
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Failed to retrieve repository sync skip".to_string())
}

pub async fn delete_skill_repository_sync_skip(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
) -> Result<bool, String> {
    let result = sqlx::query(
        "DELETE FROM skill_repository_sync_skips
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.rows_affected() > 0)
}

pub async fn detach_skill_remote_source(pool: &DbPool, skill_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_update_states WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_repository_members WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    prune_empty_skill_repositories(pool).await?;
    Ok(())
}

pub async fn delete_empty_skill_repository(
    pool: &DbPool,
    repository_id: &str,
) -> Result<bool, String> {
    let repository = get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| format!("Repository '{}' not found", repository_id))?;
    if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err("The system unknown-source repository cannot be deleted".to_string());
    }

    let result = sqlx::query(
        "DELETE FROM skill_repositories
         WHERE id = ?
           AND id <> ?
           AND is_unknown = 0
           AND NOT EXISTS (
             SELECT 1 FROM skill_repository_members
             WHERE repository_id = skill_repositories.id
           )",
    )
    .bind(repository_id)
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.rows_affected() > 0)
}

pub async fn prune_empty_skill_repositories(pool: &DbPool) -> Result<u64, String> {
    let result = sqlx::query(
        "DELETE FROM skill_repositories
         WHERE id <> ?
           AND is_unknown = 0
           AND NOT EXISTS (
             SELECT 1 FROM skill_repository_members
             WHERE repository_id = skill_repositories.id
           )",
    )
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.rows_affected())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_skill_repository(
    pool: &DbPool,
    id: Option<&str>,
    name: &str,
    source_type: &str,
    owner: Option<&str>,
    repo: Option<&str>,
    branch: Option<&str>,
    url: Option<&str>,
    is_unknown: bool,
) -> Result<SkillRepository, String> {
    let normalized_id = id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, pinned, is_unknown, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           source_type = excluded.source_type,
           owner = excluded.owner,
           repo = excluded.repo,
           branch = excluded.branch,
           url = excluded.url,
           pinned = skill_repositories.pinned,
           is_unknown = excluded.is_unknown,
           updated_at = excluded.updated_at",
    )
    .bind(&normalized_id)
    .bind(name)
    .bind(source_type)
    .bind(owner)
    .bind(repo)
    .bind(branch)
    .bind(url)
    .bind(is_unknown)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_skill_repository_by_id(pool, &normalized_id)
        .await?
        .ok_or_else(|| "Failed to retrieve repository metadata".to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn assign_github_repository_to_skill(
    pool: &DbPool,
    owner: &str,
    repo: &str,
    branch: &str,
    url: &str,
    skill_id: &str,
    source_path: &str,
) -> Result<SkillRepository, String> {
    let repository_id = github_repository_id(owner, repo, branch);
    let name = format!("{owner}/{repo}");
    let repository = create_or_update_skill_repository(
        pool,
        Some(&repository_id),
        &name,
        "github",
        Some(owner),
        Some(repo),
        Some(branch),
        Some(url),
        false,
    )
    .await?;
    assign_skills_to_repository(
        pool,
        &repository.id,
        &[skill_id.to_string()],
        Some(source_path),
    )
    .await?;
    Ok(repository)
}

/// Atomically persist an imported GitHub skill row and its repository assignment.
///
/// GitHub imports write files before touching the database. Keeping the skill
/// upsert and repository membership in one transaction prevents a half-state
/// where the Central skill row exists without source metadata (or vice versa)
/// if the second write fails.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_skill_with_github_repository(
    pool: &DbPool,
    skill: &Skill,
    owner: &str,
    repo: &str,
    branch: &str,
    url: &str,
    source_path: &str,
) -> Result<(), String> {
    let mut transaction = pool.begin().await.map_err(|e| e.to_string())?;

    upsert_skill_in_transaction(&mut transaction, skill).await?;

    let repository_id = github_repository_id(owner, repo, branch);
    let repository_name = format!("{owner}/{repo}");
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO skill_repositories
         (id, name, source_type, owner, repo, branch, url, pinned, is_unknown, created_at, updated_at)
         VALUES (?, ?, 'github', ?, ?, ?, ?, 0, 0, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           source_type = excluded.source_type,
           owner = excluded.owner,
           repo = excluded.repo,
           branch = excluded.branch,
           url = excluded.url,
           pinned = skill_repositories.pinned,
           is_unknown = excluded.is_unknown,
           updated_at = excluded.updated_at",
    )
    .bind(&repository_id)
    .bind(&repository_name)
    .bind(owner)
    .bind(repo)
    .bind(branch)
    .bind(url)
    .bind(&now)
    .bind(&now)
    .execute(&mut *transaction)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO skill_repository_members
         (skill_id, repository_id, source_path, added_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(skill_id) DO UPDATE SET
           repository_id = excluded.repository_id,
           source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
           updated_at = excluded.updated_at",
    )
    .bind(&skill.id)
    .bind(&repository_id)
    .bind(source_path)
    .bind(&now)
    .bind(&now)
    .execute(&mut *transaction)
    .await
    .map_err(|e| e.to_string())?;

    transaction.commit().await.map_err(|e| e.to_string())
}

pub async fn set_skill_repository_pinned(
    pool: &DbPool,
    repository_id: &str,
    pinned: bool,
) -> Result<SkillRepository, String> {
    let repository = get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| format!("Repository '{}' not found", repository_id))?;
    if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err("The system unknown-source repository cannot be pinned".to_string());
    }

    let now = now_rfc3339();
    sqlx::query(
        "UPDATE skill_repositories
         SET pinned = ?, updated_at = ?
         WHERE id = ? AND id <> ? AND is_unknown = 0",
    )
    .bind(pinned)
    .bind(&now)
    .bind(repository_id)
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| "Failed to retrieve repository metadata".to_string())
}

pub async fn assign_skills_to_repository(
    pool: &DbPool,
    repository_id: &str,
    skill_ids: &[String],
    source_path: Option<&str>,
) -> Result<(), String> {
    let existing = get_skill_repository_by_id(pool, repository_id).await?;
    if existing.is_none() {
        return Err(format!("Repository '{}' not found", repository_id));
    }

    let now = now_rfc3339();
    for skill_id in skill_ids {
        sqlx::query(
            "INSERT INTO skill_repository_members
             (skill_id, repository_id, source_path, added_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(skill_id) DO UPDATE SET
               repository_id = excluded.repository_id,
               source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
               updated_at = excluded.updated_at",
        )
        .bind(skill_id)
        .bind(repository_id)
        .bind(source_path)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn get_skill_repository_assignment(
    pool: &DbPool,
    skill_id: &str,
) -> Result<SkillRepositoryAssignment, String> {
    let assigned = sqlx::query_as::<_, SkillRepository>(
        "SELECT r.* FROM skill_repositories r
         JOIN skill_repository_members m ON r.id = m.repository_id
         WHERE m.skill_id = ?",
    )
    .bind(skill_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(repository) = assigned {
        let source_path = sqlx::query_scalar::<_, Option<String>>(
            "SELECT source_path FROM skill_repository_members WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .flatten();
        return Ok(SkillRepositoryAssignment {
            is_source_unknown: repository.is_unknown,
            repository,
            source_path,
        });
    }

    Ok(SkillRepositoryAssignment {
        repository: get_local_unknown_repository(pool).await?,
        source_path: None,
        is_source_unknown: true,
    })
}

pub async fn get_skill_repository_assignments_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, SkillRepositoryAssignment>, String> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT
            m.skill_id AS skill_id,
            m.source_path AS source_path,
            r.id AS repository_id,
            r.name AS repository_name,
            r.source_type AS repository_source_type,
            r.owner AS repository_owner,
            r.repo AS repository_repo,
            r.branch AS repository_branch,
            r.url AS repository_url,
            r.pinned AS repository_pinned,
            r.is_unknown AS repository_is_unknown,
            r.created_at AS repository_created_at,
            r.updated_at AS repository_updated_at,
            r.last_synced_at AS repository_last_synced_at
         FROM skill_repository_members m
         JOIN skill_repositories r ON r.id = m.repository_id
         WHERE m.skill_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let mut assignments = HashMap::with_capacity(rows.len());
    for row in rows {
        let skill_id: String = row.try_get("skill_id").map_err(|e| e.to_string())?;
        let repository = SkillRepository {
            id: row.try_get("repository_id").map_err(|e| e.to_string())?,
            name: row.try_get("repository_name").map_err(|e| e.to_string())?,
            source_type: row
                .try_get("repository_source_type")
                .map_err(|e| e.to_string())?,
            owner: row.try_get("repository_owner").map_err(|e| e.to_string())?,
            repo: row.try_get("repository_repo").map_err(|e| e.to_string())?,
            branch: row
                .try_get("repository_branch")
                .map_err(|e| e.to_string())?,
            url: row.try_get("repository_url").map_err(|e| e.to_string())?,
            pinned: row
                .try_get("repository_pinned")
                .map_err(|e| e.to_string())?,
            is_unknown: row
                .try_get("repository_is_unknown")
                .map_err(|e| e.to_string())?,
            created_at: row
                .try_get("repository_created_at")
                .map_err(|e| e.to_string())?,
            updated_at: row
                .try_get("repository_updated_at")
                .map_err(|e| e.to_string())?,
            last_synced_at: row
                .try_get("repository_last_synced_at")
                .map_err(|e| e.to_string())?,
        };
        assignments.insert(
            skill_id,
            SkillRepositoryAssignment {
                is_source_unknown: repository.is_unknown,
                repository,
                source_path: row.try_get("source_path").map_err(|e| e.to_string())?,
            },
        );
    }

    Ok(assignments)
}

pub async fn get_skill_repositories_with_stats(
    pool: &DbPool,
) -> Result<Vec<SkillRepositoryWithStats>, String> {
    let rows = sqlx::query(
        "SELECT
            r.id, r.name, r.source_type, r.owner, r.repo, r.branch, r.url,
            r.pinned, r.is_unknown, r.created_at, r.updated_at, r.last_synced_at,
            CASE
              WHEN r.id = ? THEN (
                SELECT COUNT(*)
                FROM skills s_unknown
                LEFT JOIN skill_repository_members m_unknown
                  ON s_unknown.id = m_unknown.skill_id
                WHERE s_unknown.is_central = 1 AND m_unknown.skill_id IS NULL
              )
              ELSE COUNT(s.id)
            END AS skill_count,
            CASE
              WHEN r.id = ? THEN (
                SELECT COUNT(*)
                FROM skills s_unknown
                LEFT JOIN skill_repository_members m_unknown
                  ON s_unknown.id = m_unknown.skill_id
                WHERE s_unknown.is_central = 1 AND m_unknown.skill_id IS NULL
              )
              ELSE 0
            END AS unknown_skill_count
         FROM skill_repositories r
         LEFT JOIN skill_repository_members m ON r.id = m.repository_id
         LEFT JOIN skills s ON s.id = m.skill_id AND s.is_central = 1
         GROUP BY r.id
         ORDER BY r.is_unknown DESC, r.pinned DESC, r.name",
    )
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let repository = SkillRepository {
            id: row.try_get("id").map_err(|e| e.to_string())?,
            name: row.try_get("name").map_err(|e| e.to_string())?,
            source_type: row.try_get("source_type").map_err(|e| e.to_string())?,
            owner: row.try_get("owner").map_err(|e| e.to_string())?,
            repo: row.try_get("repo").map_err(|e| e.to_string())?,
            branch: row.try_get("branch").map_err(|e| e.to_string())?,
            url: row.try_get("url").map_err(|e| e.to_string())?,
            pinned: row.try_get("pinned").map_err(|e| e.to_string())?,
            is_unknown: row.try_get("is_unknown").map_err(|e| e.to_string())?,
            created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            updated_at: row.try_get("updated_at").map_err(|e| e.to_string())?,
            last_synced_at: row.try_get("last_synced_at").map_err(|e| e.to_string())?,
        };
        result.push(SkillRepositoryWithStats {
            unknown_skill_count: row
                .try_get("unknown_skill_count")
                .map_err(|e| e.to_string())?,
            repository,
            skill_count: row.try_get("skill_count").map_err(|e| e.to_string())?,
        });
    }

    Ok(result)
}

/// 写 `skill_repositories.last_synced_at` —— Phase P2 引入。
///
/// 仅由 inventory refresh 流程调用，所以无需走 upsert / unknown 守卫，直接 UPDATE。
/// 未命中（repo 已被删除）静默忽略。
pub async fn set_repository_last_synced_at(
    pool: &DbPool,
    repository_id: &str,
    timestamp: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE skill_repositories SET last_synced_at = ? WHERE id = ?")
        .bind(timestamp)
        .bind(repository_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
