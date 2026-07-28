use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{self, Collection, DbPool, Skill};
use crate::services::installation::{install_skill, InstallTransport};
use crate::targets::ActiveTarget;
use crate::AppState;

use super::linker::{BatchInstallResult, FailedInstall};

mod export_import;

pub use export_import::{export_collection_impl, import_collection_impl, CollectionExport};

// ─── Types ────────────────────────────────────────────────────────────────────

/// A Collection with its member skills included.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// All skills that are members of this collection.
    pub skills: Vec<Skill>,
}

// ─── Core Implementations (testable without Tauri State) ─────────────────────

/// Create a new collection and return it.
pub async fn create_collection_impl(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
) -> Result<Collection, String> {
    if name.trim().is_empty() {
        return Err("Collection name cannot be empty".to_string());
    }
    db::create_collection(pool, name, description)
        .await
        .map_err(|e| e.to_string())
}

/// Return all collections.
pub async fn get_collections_impl(pool: &DbPool) -> Result<Vec<Collection>, String> {
    db::get_all_collections(pool)
        .await
        .map_err(|e| e.to_string())
}

async fn get_collection_or_err(pool: &DbPool, collection_id: &str) -> Result<Collection, String> {
    db::get_collection_by_id(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Collection '{}' not found", collection_id))
}

/// Return a collection with its member skills.
pub async fn get_collection_detail_impl(
    pool: &DbPool,
    collection_id: &str,
) -> Result<CollectionDetail, String> {
    let collection = get_collection_or_err(pool, collection_id).await?;

    let skills = db::get_collection_skills(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CollectionDetail {
        id: collection.id,
        name: collection.name,
        description: collection.description,
        created_at: collection.created_at,
        updated_at: collection.updated_at,
        skills,
    })
}

/// Add a skill to a collection (idempotent).
pub async fn add_skill_to_collection_impl(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    get_collection_or_err(pool, collection_id).await?;

    db::add_skill_to_collection(pool, collection_id, skill_id)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a skill from a collection.
pub async fn remove_skill_from_collection_impl(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    get_collection_or_err(pool, collection_id).await?;

    db::remove_skill_from_collection(pool, collection_id, skill_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a collection and all its skill memberships.
pub async fn delete_collection_impl(pool: &DbPool, collection_id: &str) -> Result<(), String> {
    get_collection_or_err(pool, collection_id).await?;

    db::delete_collection(pool, collection_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a collection's name and optional description.
pub async fn update_collection_impl(
    pool: &DbPool,
    collection_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<Collection, String> {
    if name.trim().is_empty() {
        return Err("Collection name cannot be empty".to_string());
    }

    get_collection_or_err(pool, collection_id).await?;

    db::update_collection(pool, collection_id, name, description)
        .await
        .map_err(|e| e.to_string())?;

    db::get_collection_by_id(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Collection '{}' not found after update", collection_id))
}

/// Install all skills in a collection to the given agents (symlink method).
///
/// Each (skill, agent) pair is attempted independently. Failures are collected
/// in the `failed` list rather than aborting the whole batch.
pub async fn batch_install_collection_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    collection_id: &str,
    agent_ids: &[String],
) -> Result<BatchInstallResult, String> {
    get_collection_or_err(pool, collection_id).await?;

    let skills = db::get_collection_skills(pool, collection_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    match InstallTransport::for_target(active_target).await {
        Ok(transport) => {
            // Historical semantics: local collection installs symlink; remote
            // targets have always installed collection skills as copies.
            let method = if transport.is_remote() {
                "copy"
            } else {
                "symlink"
            };
            for skill in &skills {
                for agent_id in agent_ids {
                    match install_skill(pool, &transport, &skill.id, agent_id, method).await {
                        Ok(_) => succeeded.push(format!("{}:{}", skill.id, agent_id)),
                        Err(e) => failed.push(FailedInstall {
                            agent_id: format!("{}:{}", skill.id, agent_id),
                            error: e.to_string(),
                        }),
                    }
                }
            }
        }
        Err(error) => {
            let error = error.to_string();
            for skill in &skills {
                for agent_id in agent_ids {
                    failed.push(FailedInstall {
                        agent_id: format!("{}:{}", skill.id, agent_id),
                        error: error.clone(),
                    });
                }
            }
        }
    }

    Ok(BatchInstallResult {
        succeeded,
        skipped: Vec::new(),
        failed,
    })
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

/// Tauri command: create a new collection.
#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> crate::ipc_error::IpcResult<Collection> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        create_collection_impl(&pool, &name, description.as_deref()).await
    })
}

/// Tauri command: return all collections.
#[tauri::command]
pub async fn get_collections(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<Collection>> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        get_collections_impl(&pool).await
    })
}

/// Tauri command: return a collection with its member skills.
#[tauri::command]
pub async fn get_collection_detail(
    state: State<'_, AppState>,
    collection_id: String,
) -> crate::ipc_error::IpcResult<CollectionDetail> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        get_collection_detail_impl(&pool, &collection_id).await
    })
}

/// Tauri command: add a skill to a collection.
#[tauri::command]
pub async fn add_skill_to_collection(
    state: State<'_, AppState>,
    collection_id: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        add_skill_to_collection_impl(&pool, &collection_id, &skill_id).await
    })
}

/// Tauri command: remove a skill from a collection.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_skill_from_collection(
    state: State<'_, AppState>,
    collection_id: String,
    skill_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        remove_skill_from_collection_impl(&pool, &collection_id, &skill_id).await
    })
}

/// Tauri command: delete a collection.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        delete_collection_impl(&pool, &collection_id).await
    })
}

/// Tauri command: update a collection's name and description.
#[tauri::command]
pub async fn update_collection(
    state: State<'_, AppState>,
    collection_id: String,
    name: String,
    description: Option<String>,
) -> crate::ipc_error::IpcResult<Collection> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        update_collection_impl(&pool, &collection_id, &name, description.as_deref()).await
    })
}

/// Tauri command: install all skills in a collection to the given agents.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn batch_install_collection(
    state: State<'_, AppState>,
    collection_id: String,
    agent_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<BatchInstallResult> {
    crate::ipc_boundary_async!({
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let pool = request_context.db().clone();
        batch_install_collection_impl(&pool, &active_target, &collection_id, &agent_ids).await
    })
}

/// Tauri command: export a collection to a JSON string.
#[tauri::command]
pub async fn export_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        export_collection_impl(&pool, &collection_id).await
    })
}

/// Tauri command: import a collection from a JSON string.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn import_collection(
    state: State<'_, AppState>,
    json: String,
) -> crate::ipc_error::IpcResult<Collection> {
    crate::ipc_boundary_async!({
        let pool = state.active_db().await?;
        import_collection_impl(&pool, &json).await
    })
}

#[cfg(test)]
mod tests;
