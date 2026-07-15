//! Marketplace service: skill registry CRUD, GitHub registry sync, and remote
//! skill installation. The Tauri command shells in `commands::marketplace`
//! delegate to the `*_impl` functions exposed here.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::paths;
use crate::secrets::SecretStore;
use crate::services::github_import;
use crate::targets::{connect_remote_target, remote_join, ActiveTarget};

mod error;
mod skills_sh;

pub use error::MarketplaceError;
pub use skills_sh::{
    browse_skills_sh_directory_impl, install_from_skills_sh_impl,
    install_from_skills_sh_with_options_impl, read_skills_sh_file_impl,
    resolve_skills_sh_url_impl, search_skills_sh_impl, SkillsShFileEntry, SkillsShSkill,
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
struct MarketplaceSkillRow {
    name: String,
    download_url: String,
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
    Ok(marketplace_skills_from_candidates(registry_id, candidates))
}

pub(crate) fn marketplace_skills_from_candidates(
    registry_id: &str,
    candidates: Vec<github_import::RemoteSkillCandidate>,
) -> Vec<MarketplaceSkill> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut seen_names = HashSet::new();
    let mut skills = Vec::new();

    for candidate in candidates {
        if !seen_names.insert(candidate.skill_name.clone()) {
            continue;
        }

        skills.push(MarketplaceSkill {
            id: format!("{}::{}", registry_id, candidate.skill_id),
            registry_id: registry_id.to_string(),
            name: candidate.skill_name,
            description: candidate.description,
            download_url: candidate.download_url,
            is_installed: false,
            synced_at: now.clone(),
            cache_updated_at: Some(now.clone()),
        });
    }

    skills
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
    // Don't allow removing built-in registries
    let row = sqlx::query("SELECT is_builtin FROM skill_registries WHERE id = ?")
        .bind(&registry_id)
        .fetch_optional(pool)
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
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM skill_registries WHERE id = ?")
        .bind(&registry_id)
        .execute(pool)
        .await?;

    Ok(())
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

    let installed_central_names: HashSet<String> = crate::db::get_central_skills(pool)
        .await?
        .into_iter()
        .flat_map(|skill| [skill.id, skill.name])
        .collect();

    // Upsert skills into marketplace_skills
    for skill in &skills {
        let is_installed = installed_central_names.contains(&skill.name);

        sqlx::query(
            "INSERT INTO marketplace_skills (id, registry_id, name, description, download_url, is_installed, synced_at, cache_updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                download_url = excluded.download_url,
                is_installed = excluded.is_installed,
                synced_at = excluded.synced_at,
                cache_updated_at = excluded.cache_updated_at",
        )
        .bind(&skill.id)
        .bind(&skill.registry_id)
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(&skill.download_url)
        .bind(is_installed)
        .bind(&skill.synced_at)
        .bind(&skill.cache_updated_at)
        .execute(pool)
        .await?;
    }

    // Update last_synced
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE skill_registries
         SET last_synced = ?, last_attempted_sync = ?, last_sync_status = ?, last_sync_error = NULL, cache_updated_at = ?
         WHERE id = ?",
    )
        .bind(&now)
        .bind(&attempt_time)
        .bind(RegistrySyncStatus::Success.as_str())
        .bind(&now)
        .bind(&registry_id)
        .execute(pool)
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
    Ok(rows.iter().map(row_to_marketplace_skill).collect())
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

// ─── Install ─────────────────────────────────────────────────────────────────

pub(crate) fn central_skill_dir_for_name(central_dir: &Path, skill_name: &str) -> PathBuf {
    central_dir.join(skill_name)
}

#[cfg(test)]
pub(crate) fn is_skill_installed_in_central(central_dir: &Path, skill_name: &str) -> bool {
    central_skill_dir_for_name(central_dir, skill_name)
        .join("SKILL.md")
        .exists()
}

pub async fn install_marketplace_skill_impl(
    pool: &crate::db::DbPool,
    active_target: ActiveTarget,
    skill_id: String,
) -> Result<(), MarketplaceError> {
    // Get skill info
    let skill = sqlx::query_as::<_, MarketplaceSkillRow>(
        "SELECT id, registry_id, name, description, download_url, is_installed, synced_at
         FROM marketplace_skills WHERE id = ?",
    )
    .bind(&skill_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MarketplaceError::SkillNotFound)?;

    // Download SKILL.md content
    let client = reqwest::Client::builder()
        .user_agent(crate::commands::APP_USER_AGENT)
        .build()
        .map_err(|e| MarketplaceError::Http(e.to_string()))?;

    let resp = client
        .get(&skill.download_url)
        .send()
        .await
        .map_err(|e| MarketplaceError::Http(format!("Download failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(MarketplaceError::Http(format!(
            "Download returned {}",
            resp.status()
        )));
    }

    let content = resp
        .text()
        .await
        .map_err(|e| MarketplaceError::Http(format!("Failed to read response: {}", e)))?;

    match &active_target {
        ActiveTarget::Local => {
            let skill_dir = central_skill_dir_for_name(&paths::central_skills_dir(), &skill.name);
            std::fs::create_dir_all(&skill_dir)
                .map_err(|e| MarketplaceError::io("Failed to create directory", e))?;

            let skill_md_path = skill_dir.join("SKILL.md");
            std::fs::write(&skill_md_path, &content)
                .map_err(|e| MarketplaceError::io("Failed to write SKILL.md", e))?;
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            let central = crate::db::get_agent_by_id(pool, "central")
                .await?
                .ok_or(MarketplaceError::CentralAgentMissing)?;
            let skill_dir = remote_join(&central.global_skills_dir, &skill.name);
            let skill_md_path = remote_join(&skill_dir, "SKILL.md");
            let connection = connect_remote_target(&active_target)
                .await
                .map_err(|e| MarketplaceError::Remote(e.to_string()))?;
            connection
                .mkdir_p(&skill_dir)
                .await
                .map_err(|e| MarketplaceError::Remote(e.to_string()))?;
            connection
                .write_file(&skill_md_path, content.as_bytes())
                .await
                .map_err(|e| MarketplaceError::Remote(e.to_string()))?;
        }
    }

    // Mark as installed in DB
    sqlx::query("UPDATE marketplace_skills SET is_installed = 1 WHERE id = ?")
        .bind(&skill_id)
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests;
