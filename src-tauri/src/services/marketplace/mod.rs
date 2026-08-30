//! Marketplace service: skill registry CRUD, GitHub registry sync, and remote
//! skill installation. The Tauri command shells in `commands::marketplace`
//! delegate to the `*_impl` functions exposed here.

use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::db::sqlite_batch::sqlite_rows_per_batch;
use crate::secrets::SecretStore;
use crate::services::{central_updates, github_import};
use crate::targets::{remote_join, ActiveTarget};

mod error;
mod skills_sh;

pub use error::MarketplaceError;
pub use skills_sh::{
    browse_skills_sh_directory_impl, install_from_skills_sh_impl,
    install_from_skills_sh_with_options_impl, read_skills_sh_file_impl, resolve_skills_sh_url_impl,
    search_skills_sh_impl, SkillsShFileEntry, SkillsShSkill,
};

#[cfg(test)]
pub(crate) use skills_sh::{
    resolve_skills_sh_candidate_from_snapshot, skills_sh_file_entries_from_snapshot,
    source_to_github_url,
};

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillRegistry {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub is_builtin: bool,
    pub is_enabled: bool,
    pub last_synced: Option<String>,
    pub last_attempted_sync: Option<String>,
    pub last_sync_status: String,
    pub last_sync_error: Option<String>,
    pub cache_updated_at: Option<String>,
    pub cache_expires_at: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketplaceSkill {
    pub id: String,
    pub registry_id: String,
    pub name: String,
    pub description: Option<String>,
    pub download_url: String,
    pub is_installed: bool,
    pub synced_at: String,
    pub cache_updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistrySyncStatus {
    Never,
    Success,
    Error,
}

impl RegistrySyncStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Success => "success",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCacheMetadata {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub cache_expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncRegistryOptions {
    pub force_refresh: bool,
}

#[derive(sqlx::FromRow)]
struct MarketplaceInstallSourceRow {
    registry_id: String,
    source_type: String,
    url: String,
    is_enabled: bool,
}

#[derive(sqlx::FromRow)]
struct CentralMarketplaceIdentityRow {
    skill_id: String,
    owner: Option<String>,
    repo: Option<String>,
    resolved_commit_sha: Option<String>,
    content_digest: Option<String>,
}

// ─── Registry Fetcher ────────────────────────────────────────────────────────

/// Fetch skills from a GitHub repository.
/// Reuses the same repository snapshot + manifest classification logic as
/// the GitHub import flow so Marketplace preview and import stay in sync.
async fn fetch_github_skills(
    auth_pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    url: &str,
    registry_id: &str,
) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
    let auth = github_import::github_direct_auth_from_secret_store(auth_pool, secrets).await?;
    let resolved = github_import::resolve_repo_source(url, auth.as_deref()).await?;
    let candidates = github_import::fetch_repo_skill_candidates_from_source(
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth.as_deref(),
    )
    .await?;
    marketplace_skills_from_candidates(registry_id, candidates)
}

pub(crate) fn marketplace_skills_from_candidates(
    registry_id: &str,
    candidates: Vec<github_import::RemoteSkillCandidate>,
) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut seen_ids = HashSet::new();
    let mut skills = Vec::new();

    for candidate in candidates {
        let id = marketplace_candidate_id(registry_id, &candidate);
        if !seen_ids.insert(id.clone()) {
            return Err(MarketplaceError::CandidateAmbiguous);
        }

        skills.push(MarketplaceSkill {
            id,
            registry_id: registry_id.to_string(),
            name: candidate.skill_name,
            description: candidate.description,
            download_url: candidate.download_url,
            is_installed: false,
            synced_at: now.clone(),
            cache_updated_at: Some(now.clone()),
        });
    }

    Ok(skills)
}

fn marketplace_candidate_id(
    registry_id: &str,
    candidate: &github_import::RemoteSkillCandidate,
) -> String {
    format!("{}::{}", registry_id, candidate.skill_id)
}

// ─── Registry CRUD ───────────────────────────────────────────────────────────

pub async fn list_registries_impl(
    pool: &crate::db::DbPool,
) -> Result<Vec<SkillRegistry>, MarketplaceError> {
    let rows = sqlx::query(
        "SELECT id, name, source_type, url, is_builtin, is_enabled, last_synced,
                last_attempted_sync, last_sync_status, last_sync_error,
                cache_updated_at, cache_expires_at, etag, last_modified, created_at
         FROM skill_registries ORDER BY is_builtin DESC, name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            use sqlx::Row;
            SkillRegistry {
                id: r.get("id"),
                name: r.get("name"),
                source_type: r.get("source_type"),
                url: r.get("url"),
                is_builtin: r.get("is_builtin"),
                is_enabled: r.get("is_enabled"),
                last_synced: r.get("last_synced"),
                last_attempted_sync: r.get("last_attempted_sync"),
                last_sync_status: r.get("last_sync_status"),
                last_sync_error: r.get("last_sync_error"),
                cache_updated_at: r.get("cache_updated_at"),
                cache_expires_at: r.get("cache_expires_at"),
                etag: r.get("etag"),
                last_modified: r.get("last_modified"),
                created_at: r.get("created_at"),
            }
        })
        .collect())
}

pub async fn add_registry_impl(
    pool: &crate::db::DbPool,
    name: String,
    source_type: String,
    url: String,
    cache_metadata: Option<RegistryCacheMetadata>,
) -> Result<SkillRegistry, MarketplaceError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let cache_metadata = cache_metadata.unwrap_or_default();

    sqlx::query(
        "INSERT INTO skill_registries
         (id, name, source_type, url, is_builtin, is_enabled, last_sync_status,
          cache_expires_at, etag, last_modified, created_at)
         VALUES (?, ?, ?, ?, 0, 1, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&source_type)
    .bind(&url)
    .bind(RegistrySyncStatus::Never.as_str())
    .bind(&cache_metadata.cache_expires_at)
    .bind(&cache_metadata.etag)
    .bind(&cache_metadata.last_modified)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(SkillRegistry {
        id,
        name,
        source_type,
        url,
        is_builtin: false,
        is_enabled: true,
        last_synced: None,
        last_attempted_sync: None,
        last_sync_status: RegistrySyncStatus::Never.as_str().to_string(),
        last_sync_error: None,
        cache_updated_at: None,
        cache_expires_at: cache_metadata.cache_expires_at,
        etag: cache_metadata.etag,
        last_modified: cache_metadata.last_modified,
        created_at: now,
    })
}

pub async fn remove_registry_impl(
    pool: &crate::db::DbPool,
    registry_id: String,
) -> Result<(), MarketplaceError> {
    let mut transaction = pool.begin().await?;
    // Don't allow removing built-in registries
    let row = sqlx::query("SELECT is_builtin FROM skill_registries WHERE id = ?")
        .bind(&registry_id)
        .fetch_optional(&mut *transaction)
        .await?;

    if let Some(r) = &row {
        use sqlx::Row;
        if r.get::<bool, _>("is_builtin") {
            return Err(MarketplaceError::BuiltinRegistryRemoval);
        }
    }

    // Delete cached skills first
    sqlx::query("DELETE FROM marketplace_skills WHERE registry_id = ?")
        .bind(&registry_id)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("DELETE FROM skill_registries WHERE id = ?")
        .bind(&registry_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn replace_registry_cache_snapshot(
    pool: &crate::db::DbPool,
    registry_id: &str,
    skills: &[MarketplaceSkill],
    installed_skill_ids: &HashSet<String>,
    attempt_time: &str,
    synced_at: &str,
) -> Result<(), MarketplaceError> {
    let result = replace_registry_cache_snapshot_in_transaction(
        pool,
        registry_id,
        skills,
        installed_skill_ids,
        attempt_time,
        synced_at,
    )
    .await;
    if let Err(error) = &result {
        record_registry_sync_error_best_effort(pool, registry_id, attempt_time, error).await;
    }
    result
}

async fn replace_registry_cache_snapshot_in_transaction(
    pool: &crate::db::DbPool,
    registry_id: &str,
    skills: &[MarketplaceSkill],
    installed_skill_ids: &HashSet<String>,
    attempt_time: &str,
    synced_at: &str,
) -> Result<(), MarketplaceError> {
    let mut transaction = pool.begin().await?;

    sqlx::query("DELETE FROM marketplace_skills WHERE registry_id = ?")
        .bind(registry_id)
        .execute(&mut *transaction)
        .await?;

    let rows_per_batch = sqlite_rows_per_batch(8)?;
    for chunk in skills.chunks(rows_per_batch) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT INTO marketplace_skills
             (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at) ",
        );
        builder.push_values(chunk, |mut row, skill| {
            row.push_bind(&skill.id)
                .push_bind(&skill.registry_id)
                .push_bind(&skill.name)
                .push_bind(&skill.description)
                .push_bind(&skill.download_url)
                .push_bind(installed_skill_ids.contains(&skill.id))
                .push_bind(&skill.synced_at)
                .push_bind(&skill.cache_updated_at);
        });
        builder.push(
            " ON CONFLICT(id) DO UPDATE SET
                registry_id = excluded.registry_id,
                name = excluded.name,
                description = excluded.description,
                download_url = excluded.download_url,
                is_installed = excluded.is_installed,
                synced_at = excluded.synced_at,
                cache_updated_at = excluded.cache_updated_at",
        );
        builder.build().execute(&mut *transaction).await?;
    }

    let result = sqlx::query(
        "UPDATE skill_registries
         SET last_synced = ?, last_attempted_sync = ?, last_sync_status = ?,
             last_sync_error = NULL, cache_updated_at = ?
         WHERE id = ?",
    )
    .bind(synced_at)
    .bind(attempt_time)
    .bind(RegistrySyncStatus::Success.as_str())
    .bind(synced_at)
    .bind(registry_id)
    .execute(&mut *transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(MarketplaceError::RegistryNotFound);
    }

    transaction.commit().await?;
    Ok(())
}

async fn record_registry_sync_error_best_effort(
    pool: &crate::db::DbPool,
    registry_id: &str,
    attempt_time: &str,
    error: &MarketplaceError,
) {
    if let Err(_marker_error) = sqlx::query(
        "UPDATE skill_registries
         SET last_attempted_sync = ?, last_sync_status = ?, last_sync_error = ?
         WHERE id = ?",
    )
    .bind(attempt_time)
    .bind(RegistrySyncStatus::Error.as_str())
    .bind(error.to_string())
    .bind(registry_id)
    .execute(pool)
    .await
    {
        tracing::warn!(
            registry_id,
            "failed to record marketplace registry sync error"
        );
    }
}

pub(crate) async fn sync_registry_impl(
    pool: &crate::db::DbPool,
    auth_pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    registry_id: String,
    options: SyncRegistryOptions,
) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
    // Get registry info
    let row = sqlx::query(
        "SELECT id, name, source_type, url, is_builtin, is_enabled, last_synced,
                last_attempted_sync, last_sync_status, last_sync_error,
                cache_updated_at, cache_expires_at, etag, last_modified, created_at
         FROM skill_registries WHERE id = ?",
    )
    .bind(&registry_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MarketplaceError::RegistryNotFound)?;

    let registry = {
        use sqlx::Row;
        SkillRegistry {
            id: row.get("id"),
            name: row.get("name"),
            source_type: row.get("source_type"),
            url: row.get("url"),
            is_builtin: row.get("is_builtin"),
            is_enabled: row.get("is_enabled"),
            last_synced: row.get("last_synced"),
            last_attempted_sync: row.get("last_attempted_sync"),
            last_sync_status: row.get("last_sync_status"),
            last_sync_error: row.get("last_sync_error"),
            cache_updated_at: row.get("cache_updated_at"),
            cache_expires_at: row.get("cache_expires_at"),
            etag: row.get("etag"),
            last_modified: row.get("last_modified"),
            created_at: row.get("created_at"),
        }
    };

    if !options.force_refresh && registry_has_cached_skills(pool, &registry.id).await? {
        return search_marketplace_skills_impl(pool, Some(registry_id), None).await;
    }

    let attempt_time = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE skill_registries
         SET last_attempted_sync = ?, last_sync_error = NULL
         WHERE id = ?",
    )
    .bind(&attempt_time)
    .bind(&registry.id)
    .execute(pool)
    .await?;

    // Fetch skills based on source type
    let skills = match registry.source_type.as_str() {
        "github" => {
            match fetch_github_skills(auth_pool, secrets, &registry.url, &registry.id).await {
                Ok(skills) => skills,
                Err(error) => {
                    sqlx::query(
                        "UPDATE skill_registries
                     SET last_attempted_sync = ?, last_sync_status = ?, last_sync_error = ?
                     WHERE id = ?",
                    )
                    .bind(&attempt_time)
                    .bind(RegistrySyncStatus::Error.as_str())
                    .bind(error.to_string())
                    .bind(&registry.id)
                    .execute(pool)
                    .await?;

                    if registry_has_cached_skills(pool, &registry.id).await? {
                        return search_marketplace_skills_impl(pool, Some(registry_id), None).await;
                    }

                    return Err(error);
                }
            }
        }
        _ => {
            return Err(MarketplaceError::UnsupportedSourceType(
                registry.source_type.clone(),
            ))
        }
    };

    let installed_identities = marketplace_installed_identities(pool).await?;
    let registry_repository_key = github_import::github_repository_key_from_source(&registry.url)?;

    let mut installed_skill_ids = HashSet::new();
    for skill in &skills {
        let is_installed = marketplace_skill_candidate_id(skill).is_some_and(|candidate_id| {
            installed_identities
                .get(candidate_id)
                .is_some_and(|repository_key| repository_key == &registry_repository_key)
        });
        if is_installed {
            installed_skill_ids.insert(skill.id.clone());
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    replace_registry_cache_snapshot(
        pool,
        &registry_id,
        &skills,
        &installed_skill_ids,
        &attempt_time,
        &now,
    )
    .await?;

    // Return the updated list
    search_marketplace_skills_impl(pool, Some(registry_id), None).await
}

pub(crate) async fn search_marketplace_skills_impl(
    pool: &crate::db::DbPool,
    registry_id: Option<String>,
    query: Option<String>,
) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
    let mut sql = String::from(
        r#"SELECT id, registry_id, name, description, download_url,
            is_installed, synced_at, cache_updated_at
         FROM marketplace_skills WHERE 1=1"#,
    );
    let mut bindings: Vec<String> = Vec::new();

    if let Some(ref rid) = registry_id {
        sql.push_str(" AND registry_id = ?");
        bindings.push(rid.clone());
    }
    if let Some(ref q) = query {
        if !q.trim().is_empty() {
            sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
            let pattern = format!("%{}%", q);
            bindings.push(pattern.clone());
            bindings.push(pattern);
        }
    }
    sql.push_str(" ORDER BY name");

    let mut q = sqlx::query(&sql);
    for b in &bindings {
        q = q.bind(b);
    }

    let rows = q.fetch_all(pool).await?;
    let mut skills = rows
        .iter()
        .map(row_to_marketplace_skill)
        .collect::<Vec<_>>();
    repair_marketplace_installed_state(pool, &mut skills).await?;
    Ok(skills)
}

pub(crate) async fn registry_has_cached_skills(
    pool: &crate::db::DbPool,
    registry_id: &str,
) -> Result<bool, MarketplaceError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM marketplace_skills WHERE registry_id = ?",
    )
    .bind(registry_id)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

fn row_to_marketplace_skill(row: &sqlx::sqlite::SqliteRow) -> MarketplaceSkill {
    use sqlx::Row;

    MarketplaceSkill {
        id: row.get("id"),
        registry_id: row.get("registry_id"),
        name: row.get("name"),
        description: row.get("description"),
        download_url: row.get("download_url"),
        is_installed: row.get::<i64, _>("is_installed") != 0,
        synced_at: row.get("synced_at"),
        cache_updated_at: row.get("cache_updated_at"),
    }
}

fn marketplace_skill_candidate_id(skill: &MarketplaceSkill) -> Option<&str> {
    skill
        .id
        .strip_prefix(&format!("{}::", skill.registry_id))
        .filter(|candidate_id| !candidate_id.is_empty())
}

async fn repair_marketplace_installed_state(
    pool: &crate::db::DbPool,
    skills: &mut [MarketplaceSkill],
) -> Result<(), MarketplaceError> {
    let installed_identities = marketplace_installed_identities(pool).await?;
    let registry_repository_keys = list_registries_impl(pool)
        .await?
        .into_iter()
        .filter(|registry| registry.source_type == "github")
        .filter_map(|registry| {
            github_import::github_repository_key_from_source(&registry.url)
                .ok()
                .map(|key| (registry.id, key))
        })
        .collect::<HashMap<_, _>>();
    for skill in skills {
        let installed = marketplace_skill_candidate_id(skill).is_some_and(|candidate_id| {
            let Some(registry_repository_key) = registry_repository_keys.get(&skill.registry_id)
            else {
                return false;
            };
            installed_identities
                .get(candidate_id)
                .is_some_and(|repository_key| repository_key == registry_repository_key)
        });
        if installed != skill.is_installed {
            skill.is_installed = installed;
            repair_marketplace_marker_best_effort(pool, &skill.id, installed).await;
        }
    }
    Ok(())
}

async fn marketplace_installed_identities(
    pool: &crate::db::DbPool,
) -> Result<HashMap<String, String>, MarketplaceError> {
    let rows = sqlx::query_as::<_, CentralMarketplaceIdentityRow>(
        "SELECT s.id AS skill_id, r.owner, r.repo,
                m.resolved_commit_sha, m.content_digest
         FROM skills s
         JOIN skill_repository_members m ON m.skill_id = s.id
         JOIN skill_repositories r ON r.id = m.repository_id
         WHERE s.is_central = 1 AND r.source_type = 'github'",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let owner = row.owner?.to_ascii_lowercase();
            let repo = row.repo?.to_ascii_lowercase();
            let commit = row.resolved_commit_sha?.trim().to_string();
            let digest = row.content_digest?.trim().to_string();
            (!commit.is_empty() && !digest.is_empty())
                .then(|| (row.skill_id, format!("{owner}/{repo}")))
        })
        .collect())
}

async fn repair_marketplace_marker_best_effort(
    pool: &crate::db::DbPool,
    skill_id: &str,
    installed: bool,
) {
    if sqlx::query("UPDATE marketplace_skills SET is_installed = ? WHERE id = ?")
        .bind(installed)
        .bind(skill_id)
        .execute(pool)
        .await
        .is_err()
    {
        tracing::warn!(
            "Marketplace installed-state cache repair failed after deriving live Central state"
        );
    }
}

// ─── Install ─────────────────────────────────────────────────────────────────

pub(crate) fn marketplace_candidate_for_id(
    registry_id: &str,
    requested_id: &str,
    candidates: &[github_import::RemoteSkillCandidate],
) -> Result<github_import::RemoteSkillCandidate, MarketplaceError> {
    marketplace_skills_from_candidates(registry_id, candidates.to_vec())?;
    let mut matches = candidates
        .iter()
        .filter(|candidate| marketplace_candidate_id(registry_id, candidate) == requested_id);
    let candidate = matches
        .next()
        .cloned()
        .ok_or(MarketplaceError::CandidateStale)?;
    if matches.next().is_some() {
        return Err(MarketplaceError::CandidateAmbiguous);
    }
    Ok(candidate)
}

pub async fn install_marketplace_skill_impl(
    pool: &crate::db::DbPool,
    auth_pool: &crate::db::DbPool,
    secrets: &dyn SecretStore,
    active_target: ActiveTarget,
    skill_id: String,
) -> Result<(), MarketplaceError> {
    let source = sqlx::query_as::<_, MarketplaceInstallSourceRow>(
        "SELECT ms.registry_id, sr.source_type, sr.url, sr.is_enabled
         FROM marketplace_skills ms
         JOIN skill_registries sr ON sr.id = ms.registry_id
         WHERE ms.id = ?",
    )
    .bind(&skill_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MarketplaceError::SkillNotFound)?;
    if !source.is_enabled {
        return Err(MarketplaceError::RegistryDisabled);
    }
    if source.source_type != "github" {
        return Err(MarketplaceError::UnsupportedSourceType(source.source_type));
    }

    let auth = github_import::github_direct_auth_from_secret_store(auth_pool, secrets).await?;
    let resolved = github_import::resolve_repo_source(&source.url, auth.as_deref()).await?;
    let pinned = github_import::acquire_pinned_repo_snapshot(resolved, auth.as_deref()).await?;
    install_marketplace_pinned_snapshot(pool, active_target, &skill_id, &source.registry_id, pinned)
        .await
}

async fn install_marketplace_pinned_snapshot(
    pool: &crate::db::DbPool,
    active_target: ActiveTarget,
    skill_id: &str,
    registry_id: &str,
    pinned: github_import::PinnedGitHubRepoSnapshot,
) -> Result<(), MarketplaceError> {
    let candidate = marketplace_candidate_for_id(registry_id, skill_id, &pinned.candidates)?;
    let content_digest = github_import::candidate_content_digest_from_snapshot(
        &pinned.snapshot,
        &candidate.source_path,
    )?;
    let central = crate::db::get_agent_by_id(pool, "central")
        .await?
        .ok_or(MarketplaceError::CentralAgentMissing)?;
    let target_directory = match &active_target {
        ActiveTarget::Local => PathBuf::from(&central.global_skills_dir).join(&candidate.skill_id),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            PathBuf::from(remote_join(&central.global_skills_dir, &candidate.skill_id))
        }
    };
    let file_path = match &active_target {
        ActiveTarget::Local => target_directory
            .join("SKILL.md")
            .to_string_lossy()
            .into_owned(),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            remote_join(&target_directory.to_string_lossy(), "SKILL.md")
        }
    };
    let repo = pinned.resolved.repo;
    let db_skill = crate::db::Skill {
        id: candidate.skill_id.clone(),
        uid: uuid::Uuid::new_v4().to_string(),
        name: candidate.skill_name.clone(),
        description: candidate.description.clone(),
        file_path,
        canonical_path: Some(target_directory.to_string_lossy().into_owned()),
        is_central: true,
        source: Some(format!("github:{}/{}", repo.owner, repo.repo)),
        content: None,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    central_updates::journaled_central_content_upsert(
        pool,
        &active_target,
        central_updates::JournaledCentralContentUpsert {
            skill: db_skill,
            repo,
            candidate,
            snapshot: &pinned.snapshot,
            target_dir: target_directory,
            resolved_commit_sha: Some(pinned.resolved_commit_sha),
            content_digest: Some(content_digest),
        },
    )
    .await?;

    repair_marketplace_marker_best_effort(pool, skill_id, true).await;

    Ok(())
}

#[cfg(test)]
mod tests;
