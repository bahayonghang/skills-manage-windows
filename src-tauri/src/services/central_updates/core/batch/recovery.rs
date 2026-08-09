use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::db::{self, DbPool};
use crate::services::central_operation::{CentralOperationError, OperationPhase, MANIFEST_VERSION};
use crate::services::central_updates::error::CentralUpdatesError;
use crate::services::central_updates::fs::{CentralFs, CopyRefreshRequest};
use crate::services::central_updates::types::{CentralUpdateFailurePhase, CentralUpdateItemError};

use super::{persist_update_manifest, record_update_error};

pub(crate) async fn recover_pending_update_operations(
    pool: &DbPool,
    fs: &CentralFs,
) -> Result<(), CentralUpdatesError> {
    crate::services::central_operation::recover_pending_delete_operations_with_transport(
        pool,
        fs.target_id(),
        fs.target_kind(),
        fs.connected_remote(),
    )
    .await?;

    for row in db::list_pending_fs_db_operations(pool, fs.target_id()).await? {
        if row.operation_kind != "central_update" {
            continue;
        }
        recover_pending_update_operation(pool, fs, &row).await?;
    }
    Ok(())
}

pub(super) async fn recover_selected_pending_update_operations(
    pool: &DbPool,
    fs: &CentralFs,
    skill_ids: &[String],
) -> Result<HashMap<String, CentralUpdateItemError>, CentralUpdatesError> {
    let selected = skill_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let rows = db::list_pending_fs_db_operations(pool, fs.target_id()).await?;
    let mut failures = HashMap::new();
    for row in rows {
        if !selected.contains(row.skill_id.as_str()) {
            continue;
        }
        let recovery = match row.operation_kind.as_str() {
            "central_delete" => {
                crate::services::central_operation::recover_pending_delete_operation_with_transport(
                    pool,
                    fs.target_id(),
                    fs.target_kind(),
                    fs.connected_remote(),
                    &row,
                )
                .await
                .map_err(CentralUpdatesError::from)
            }
            "central_update" => recover_pending_update_operation(pool, fs, &row).await,
            _ => continue,
        };
        if let Err(error) = recovery {
            failures.entry(row.skill_id).or_insert_with(|| {
                CentralUpdateItemError::new(CentralUpdateFailurePhase::Recovery, error)
            });
        }
    }
    Ok(failures)
}

pub(crate) async fn recover_pending_update_operation(
    pool: &DbPool,
    fs: &CentralFs,
    row: &db::FsDbOperationRow,
) -> Result<(), CentralUpdatesError> {
    if row.target_id != fs.target_id() || row.target_kind != fs.target_kind() {
        return Err(CentralOperationError::InvalidManifest(
            "operation target identity mismatch".to_string(),
        )
        .into());
    }
    if row.operation_kind != "central_update" {
        return Err(CentralOperationError::InvalidManifest(
            "update recovery received a non-update row".to_string(),
        )
        .into());
    }
    if row.manifest_version != MANIFEST_VERSION {
        return Err(CentralOperationError::InvalidManifest(format!(
            "unsupported manifest version {}",
            row.manifest_version
        ))
        .into());
    }
    let manifest_value: crate::services::central_operation::OperationManifest =
        serde_json::from_str(&row.manifest_json)
            .map_err(|error| CentralUpdatesError::Json(error.to_string()))?;
    manifest_value
        .validate(&row.id)
        .map_err(CentralOperationError::InvalidManifest)?;
    let crate::services::central_operation::OperationManifest::Update(mut manifest) =
        manifest_value
    else {
        return Err(CentralOperationError::InvalidManifest(
            "update row contains a delete manifest".to_string(),
        )
        .into());
    };

    let phase = row
        .phase
        .parse::<OperationPhase>()
        .map_err(CentralOperationError::InvalidManifest)?;
    match phase {
        OperationPhase::Prepared | OperationPhase::FsStaged | OperationPhase::FsSwapped => {
            if let Err(error) = fs.rollback_operation_update(&manifest, phase).await {
                record_update_error(pool, &row.id, &error).await?;
                return Err(error);
            }
            db::transition_fs_db_operation(pool, &row.id, phase.as_str(), "rolled_back").await?;
        }
        OperationPhase::DbCommitted => {
            if manifest.copies.iter().all(|copy| copy.completed) {
                fs.finalize_operation_update(&manifest).await?;
                db::transition_fs_db_operation(pool, &row.id, "db_committed", "completed").await?;
                return Ok(());
            }
            db::transition_fs_db_operation(pool, &row.id, "db_committed", "copies_pending").await?;
            recover_copy_projections(pool, fs, &row.id, &mut manifest).await?;
        }
        OperationPhase::CopiesPending => {
            recover_copy_projections(pool, fs, &row.id, &mut manifest).await?;
        }
        OperationPhase::Completed | OperationPhase::RolledBack => {}
    }
    Ok(())
}

async fn recover_copy_projections(
    pool: &DbPool,
    fs: &CentralFs,
    operation_id: &str,
    manifest: &mut crate::services::central_operation::UpdateManifest,
) -> Result<(), CentralUpdatesError> {
    let skill_id = db::get_fs_db_operation(pool, operation_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?
        .skill_id;
    let requests = manifest
        .copies
        .iter()
        .filter(|copy| !copy.completed)
        .map(|copy| CopyRefreshRequest {
            skill_id: skill_id.clone(),
            source_dir: Path::new(&manifest.target).to_path_buf(),
            target: copy.target.clone(),
        })
        .collect();
    let mut first_error = None;
    for outcome in fs.refresh_copy_installs_cancellable(requests, None).await {
        if outcome.result.is_ok() {
            if let Some(copy) = manifest
                .copies
                .iter_mut()
                .find(|copy| copy.target == outcome.target)
            {
                copy.completed = true;
            }
        } else if first_error.is_none() {
            first_error = outcome.result.err();
        }
    }
    persist_update_manifest(pool, operation_id, "copies_pending", manifest).await?;
    if let Some(error) = first_error {
        record_update_error(pool, operation_id, &error).await?;
        return Err(error);
    }
    fs.finalize_operation_update(manifest).await?;
    db::transition_fs_db_operation(pool, operation_id, "copies_pending", "completed").await?;
    Ok(())
}
