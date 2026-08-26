use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{self, Collection, DbPool, Skill};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
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
    crate::ipc_boundary_async!("create_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("create_collection")
            .expect("create_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("create_collection must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |collection: &Collection| {
                SafeOperationResult::succeeded("Collection created.").identifier(
                    SafeDetailKey::Identifier,
                    SafeIdentifier::new(&collection.id),
                )
            },
            || async move {
                create_collection_impl(&pool, &name, description.as_deref())
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// Tauri command: return all collections.
#[tauri::command]
pub async fn get_collections(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<Collection>> {
    crate::ipc_boundary_async!("get_collections", {
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
    crate::ipc_boundary_async!("get_collection_detail", {
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
    crate::ipc_boundary_async!("add_skill_to_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("add_skill_to_collection")
            .expect("add_skill_to_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("add_skill_to_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| {
                SafeOperationResult::succeeded("Skill added to collection.")
                    .count(SafeDetailKey::AffectedCount, 1)
            },
            || async move {
                add_skill_to_collection_impl(&pool, &collection_id, &skill_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
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
    crate::ipc_boundary_async!("remove_skill_from_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("remove_skill_from_collection")
            .expect("remove_skill_from_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("remove_skill_from_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| {
                SafeOperationResult::succeeded("Skill removed from collection.")
                    .count(SafeDetailKey::AffectedCount, 1)
            },
            || async move {
                remove_skill_from_collection_impl(&pool, &collection_id, &skill_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// Tauri command: delete a collection.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("delete_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("delete_collection")
            .expect("delete_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("delete_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Collection deleted."),
            || async move {
                delete_collection_impl(&pool, &collection_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
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
    crate::ipc_boundary_async!("update_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("update_collection")
            .expect("update_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("update_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Collection updated."),
            || async move {
                update_collection_impl(&pool, &collection_id, &name, description.as_deref())
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
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
    crate::ipc_boundary_async!("batch_install_collection", {
        let request_context = state.resolve_target_context().await?;
        let active_target = request_context.target().clone();
        let audit_target = match &active_target {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("batch_install_collection")
            .expect("batch_install_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("batch_install_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |result: &BatchInstallResult| {
                let succeeded = result.succeeded.len() as u64;
                let failed = result.failed.len() as u64;
                let skipped = result.skipped.len() as u64;
                let requested_count = succeeded + failed + skipped;
                let summary = if failed == 0 {
                    SafeOperationResult::succeeded("Collection installed.")
                } else {
                    SafeOperationResult::partial("Collection install partially completed.")
                };
                summary
                    .count(SafeDetailKey::RequestedCount, requested_count)
                    .count(SafeDetailKey::SucceededCount, succeeded)
                    .count(SafeDetailKey::FailedCount, failed)
                    .count(SafeDetailKey::SkippedCount, skipped)
            },
            || async move {
                batch_install_collection_impl(&pool, &active_target, &collection_id, &agent_ids)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// Tauri command: export a collection to a JSON string.
#[tauri::command]
pub async fn export_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary_async!("export_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("export_collection")
            .expect("export_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("export_collection must be auditable")
        };
        let context = OperationContext::new(audit_target).subject(
            OperationSubjectKind::Collection,
            SafeIdentifier::new(&collection_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Collection exported."),
            || async move {
                export_collection_impl(&pool, &collection_id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// Tauri command: import a collection from a JSON string.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn import_collection(
    state: State<'_, AppState>,
    json: String,
) -> crate::ipc_error::IpcResult<Collection> {
    crate::ipc_boundary_async!("import_collection", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("import_collection")
            .expect("import_collection must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("import_collection must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |collection: &Collection| {
                SafeOperationResult::succeeded("Collection imported.").identifier(
                    SafeDetailKey::Identifier,
                    SafeIdentifier::new(&collection.id),
                )
            },
            || async move {
                import_collection_impl(&pool, &json)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[cfg(test)]
mod tests;
