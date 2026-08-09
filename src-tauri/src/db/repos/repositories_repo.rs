//! `skill_repositories` and `skill_repository_members` CRUD — Phase 2c.
//!
//! `LOCAL_UNKNOWN_REPOSITORY_ID` is the system-managed sink for skills whose
//! source can't be identified; it must never be deleted.

use std::collections::HashMap;

use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::db::repos::skills_repo::upsert_skill_in_transaction;
use crate::db::sqlite_batch::{sqlite_rows_per_batch, validate_text_ids_exist, TextIdTable};
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

pub async fn get_local_unknown_repository(pool: &DbPool) -> Result<SkillRepository, sqlx::Error> {
    get_skill_repository_by_id(pool, LOCAL_UNKNOWN_REPOSITORY_ID)
        .await?
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(
                "Local unknown repository metadata is not initialized".to_string(),
            )
        })
}

pub async fn get_skill_repository_by_id(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Option<SkillRepository>, sqlx::Error> {
    sqlx::query_as::<_, SkillRepository>("SELECT * FROM skill_repositories WHERE id = ?")
        .bind(repository_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_central_skill_ids_by_repository(
    pool: &DbPool,
    repository_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
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
}

pub async fn get_central_repository_members_by_repositories(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<Vec<SkillRepositoryMember>, sqlx::Error> {
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

    let rows = query.fetch_all(pool).await?;
    let mut members = Vec::with_capacity(rows.len());
    for row in rows {
        let repository = SkillRepository {
            id: row.try_get("repository_id")?,
            name: row.try_get("repository_name")?,
            source_type: row.try_get("repository_source_type")?,
            owner: row.try_get("repository_owner")?,
            repo: row.try_get("repository_repo")?,
            branch: row.try_get("repository_branch")?,
            url: row.try_get("repository_url")?,
            pinned: row.try_get("repository_pinned")?,
            is_unknown: row.try_get("repository_is_unknown")?,
            created_at: row.try_get("repository_created_at")?,
            updated_at: row.try_get("repository_updated_at")?,
            last_synced_at: row.try_get("repository_last_synced_at")?,
        };
        members.push(SkillRepositoryMember {
            skill_id: row.try_get("skill_id")?,
            source_path: row.try_get("source_path")?,
            repository,
        });
    }

    Ok(members)
}

pub async fn get_skill_repository_sync_skips(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<Vec<SkillRepositorySyncSkip>, sqlx::Error> {
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

    query.fetch_all(pool).await
}

pub async fn upsert_skill_repository_sync_skip(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
    skill_id: &str,
    skill_name: &str,
) -> Result<SkillRepositorySyncSkip, sqlx::Error> {
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
    .await?;

    sqlx::query_as::<_, SkillRepositorySyncSkip>(
        "SELECT *
         FROM skill_repository_sync_skips
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        sqlx::Error::InvalidArgument("Failed to retrieve repository sync skip".to_string())
    })
}

pub async fn delete_skill_repository_sync_skip(
    pool: &DbPool,
    repository_id: &str,
    source_path: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM skill_repository_sync_skips
         WHERE repository_id = ? AND source_path = ?",
    )
    .bind(repository_id)
    .bind(source_path)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn detach_skill_remote_source(pool: &DbPool, skill_id: &str) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM skill_update_states WHERE skill_id = ?")
        .bind(skill_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM skill_repository_members WHERE skill_id = ?")
        .bind(skill_id)
        .execute(&mut *transaction)
        .await?;
    prune_empty_skill_repositories_in_transaction(&mut transaction).await?;
    transaction.commit().await
}

pub async fn delete_empty_skill_repository(
    pool: &DbPool,
    repository_id: &str,
) -> Result<bool, sqlx::Error> {
    let repository = get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(format!("Repository '{}' not found", repository_id))
        })?;
    if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err(sqlx::Error::InvalidArgument(
            "The system unknown-source repository cannot be deleted".to_string(),
        ));
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
    .await?;

    Ok(result.rows_affected() > 0)
}

const PRUNE_EMPTY_SKILL_REPOSITORIES_SQL: &str = "DELETE FROM skill_repositories
         WHERE id <> ?
           AND is_unknown = 0
           AND NOT EXISTS (
             SELECT 1 FROM skill_repository_members
             WHERE repository_id = skill_repositories.id
           )";

pub(crate) async fn prune_empty_skill_repositories_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(PRUNE_EMPTY_SKILL_REPOSITORIES_SQL)
        .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
        .execute(&mut **transaction)
        .await?;

    Ok(result.rows_affected())
}

pub async fn prune_empty_skill_repositories(pool: &DbPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(PRUNE_EMPTY_SKILL_REPOSITORIES_SQL)
        .bind(LOCAL_UNKNOWN_REPOSITORY_ID)
        .execute(pool)
        .await?;

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
) -> Result<SkillRepository, sqlx::Error> {
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
    ?;

    get_skill_repository_by_id(pool, &normalized_id)
        .await?
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument("Failed to retrieve repository metadata".to_string())
        })
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
) -> Result<SkillRepository, sqlx::Error> {
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
///
/// `resolved_commit_sha` / `content_digest` carry per-skill provenance from an
/// immutable preview snapshot. Callers without a confirmed snapshot pass `None`,
/// which preserves the existing membership provenance (or leaves it NULL for a
/// new row, read as "provenance unknown").
#[allow(clippy::too_many_arguments)]
pub async fn upsert_skill_with_github_repository(
    pool: &DbPool,
    skill: &Skill,
    owner: &str,
    repo: &str,
    branch: &str,
    url: &str,
    source_path: &str,
    resolved_commit_sha: Option<&str>,
    content_digest: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    upsert_skill_with_github_repository_in_transaction(
        &mut transaction,
        skill,
        owner,
        repo,
        branch,
        url,
        source_path,
        resolved_commit_sha,
        content_digest,
    )
    .await?;
    transaction.commit().await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn upsert_skill_with_github_repository_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    skill: &Skill,
    owner: &str,
    repo: &str,
    branch: &str,
    url: &str,
    source_path: &str,
    resolved_commit_sha: Option<&str>,
    content_digest: Option<&str>,
) -> Result<(), sqlx::Error> {
    upsert_skill_in_transaction(transaction, skill).await?;

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
    .execute(&mut **transaction)
    .await
    ?;

    sqlx::query(
        "INSERT INTO skill_repository_members
         (skill_id, repository_id, source_path, resolved_commit_sha, content_digest, added_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_id) DO UPDATE SET
           repository_id = excluded.repository_id,
           source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
           resolved_commit_sha = COALESCE(excluded.resolved_commit_sha, skill_repository_members.resolved_commit_sha),
           content_digest = COALESCE(excluded.content_digest, skill_repository_members.content_digest),
           updated_at = excluded.updated_at",
    )
    .bind(&skill.id)
    .bind(&repository_id)
    .bind(source_path)
    .bind(resolved_commit_sha)
    .bind(content_digest)
    .bind(&now)
    .bind(&now)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Read the per-skill GitHub import provenance recorded at import time.
///
/// `None` values mean "provenance unknown" (pre-migration rows or imports that
/// did not run through a confirmed preview snapshot).
pub async fn get_skill_repository_provenance(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Option<(Option<String>, Option<String>)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT resolved_commit_sha, content_digest
         FROM skill_repository_members
         WHERE skill_id = ?",
    )
    .bind(skill_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| {
        (
            row.get::<Option<String>, _>("resolved_commit_sha"),
            row.get::<Option<String>, _>("content_digest"),
        )
    }))
}

pub async fn set_skill_repository_pinned(
    pool: &DbPool,
    repository_id: &str,
    pinned: bool,
) -> Result<SkillRepository, sqlx::Error> {
    let repository = get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(format!("Repository '{}' not found", repository_id))
        })?;
    if repository.id == LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err(sqlx::Error::InvalidArgument(
            "The system unknown-source repository cannot be pinned".to_string(),
        ));
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
    .await?;

    get_skill_repository_by_id(pool, repository_id)
        .await?
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument("Failed to retrieve repository metadata".to_string())
        })
}

pub async fn assign_skills_to_repository(
    pool: &DbPool,
    repository_id: &str,
    skill_ids: &[String],
    source_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let repository_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM skill_repositories WHERE id = ?)")
            .bind(repository_id)
            .fetch_one(&mut *transaction)
            .await?;
    if !repository_exists {
        return Err(sqlx::Error::InvalidArgument(format!(
            "Repository '{}' not found",
            repository_id
        )));
    }
    validate_text_ids_exist(&mut transaction, TextIdTable::Skills, "Skill", skill_ids).await?;

    let now = now_rfc3339();
    let rows_per_batch = sqlite_rows_per_batch(5)?;
    for chunk in skill_ids.chunks(rows_per_batch) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO skill_repository_members
             (skill_id, repository_id, source_path, added_at, updated_at) ",
        );
        builder.push_values(chunk, |mut row, skill_id| {
            row.push_bind(skill_id)
                .push_bind(repository_id)
                .push_bind(source_path)
                .push_bind(&now)
                .push_bind(&now);
        });
        builder.push(
            " ON CONFLICT(skill_id) DO UPDATE SET
               repository_id = excluded.repository_id,
               source_path = COALESCE(excluded.source_path, skill_repository_members.source_path),
               updated_at = excluded.updated_at",
        );
        builder.build().execute(&mut *transaction).await?;
    }

    transaction.commit().await
}

pub async fn get_skill_repository_assignment(
    pool: &DbPool,
    skill_id: &str,
) -> Result<SkillRepositoryAssignment, sqlx::Error> {
    let assigned = sqlx::query_as::<_, SkillRepository>(
        "SELECT r.* FROM skill_repositories r
         JOIN skill_repository_members m ON r.id = m.repository_id
         WHERE m.skill_id = ?",
    )
    .bind(skill_id)
    .fetch_optional(pool)
    .await?;

    if let Some(repository) = assigned {
        let source_path = sqlx::query_scalar::<_, Option<String>>(
            "SELECT source_path FROM skill_repository_members WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_optional(pool)
        .await?
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
) -> Result<HashMap<String, SkillRepositoryAssignment>, sqlx::Error> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut assignments = HashMap::with_capacity(skill_ids.len());
    for chunk in skill_ids.chunks(crate::db::sqlite_batch::SQLITE_IN_QUERY_BATCH_SIZE) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
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
        for skill_id in chunk {
            query = query.bind(skill_id);
        }

        for row in query.fetch_all(pool).await? {
            let skill_id: String = row.try_get("skill_id")?;
            let repository = SkillRepository {
                id: row.try_get("repository_id")?,
                name: row.try_get("repository_name")?,
                source_type: row.try_get("repository_source_type")?,
                owner: row.try_get("repository_owner")?,
                repo: row.try_get("repository_repo")?,
                branch: row.try_get("repository_branch")?,
                url: row.try_get("repository_url")?,
                pinned: row.try_get("repository_pinned")?,
                is_unknown: row.try_get("repository_is_unknown")?,
                created_at: row.try_get("repository_created_at")?,
                updated_at: row.try_get("repository_updated_at")?,
                last_synced_at: row.try_get("repository_last_synced_at")?,
            };
            assignments.insert(
                skill_id,
                SkillRepositoryAssignment {
                    is_source_unknown: repository.is_unknown,
                    repository,
                    source_path: row.try_get("source_path")?,
                },
            );
        }
    }

    Ok(assignments)
}

pub async fn get_skill_repositories_with_stats(
    pool: &DbPool,
) -> Result<Vec<SkillRepositoryWithStats>, sqlx::Error> {
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
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let repository = SkillRepository {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            source_type: row.try_get("source_type")?,
            owner: row.try_get("owner")?,
            repo: row.try_get("repo")?,
            branch: row.try_get("branch")?,
            url: row.try_get("url")?,
            pinned: row.try_get("pinned")?,
            is_unknown: row.try_get("is_unknown")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            last_synced_at: row.try_get("last_synced_at")?,
        };
        result.push(SkillRepositoryWithStats {
            unknown_skill_count: row.try_get("unknown_skill_count")?,
            repository,
            skill_count: row.try_get("skill_count")?,
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
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE skill_repositories SET last_synced_at = ? WHERE id = ?")
        .bind(timestamp)
        .bind(repository_id)
        .execute(pool)
        .await
        .map(|_| ())
}
