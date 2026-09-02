use crate::db::DbPool;
use crate::services::central_updates::error::CentralUpdatesError;
use crate::services::central_updates::fs::CentralFs;
use crate::services::central_updates::types::{CentralUpdateFailurePhase, CentralUpdateItemError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CommitOutcomeFault {
    VisibleAfterCommit,
    InvisibleWithoutCommit,
}

std::thread_local! {
    static COMMIT_OUTCOME_FAULT: std::cell::Cell<Option<CommitOutcomeFault>> =
        const { std::cell::Cell::new(None) };
}

pub(super) fn set_commit_outcome_fault(fault: Option<CommitOutcomeFault>) {
    COMMIT_OUTCOME_FAULT.with(|cell| cell.set(fault));
}

fn take_commit_outcome_fault() -> Option<CommitOutcomeFault> {
    COMMIT_OUTCOME_FAULT.with(|cell| cell.take())
}

pub(super) async fn commit_or_inject_outcome(
    pool: &DbPool,
    fs: &CentralFs,
    update: &super::PreparedUpdate,
    transaction: sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<Result<(), sqlx::Error>, (usize, CentralUpdateItemError)> {
    let commit_fault = take_commit_outcome_fault();
    if commit_fault == Some(CommitOutcomeFault::InvisibleWithoutCommit) {
        if let Err(rollback_error) = transaction.rollback().await {
            return Err(super::phased_item_error(
                update.index,
                CentralUpdateFailurePhase::DatabaseCommit,
                rollback_error.into(),
            ));
        }
        let error = super::rollback_staged_after_db_failure(
            pool,
            fs,
            &update.operation_id,
            &update.manifest,
            CentralUpdatesError::Batch(
                "injected commit-unknown without a visible db_committed row".to_string(),
            ),
        )
        .await;
        return Err(super::phased_item_error(
            update.index,
            CentralUpdateFailurePhase::DatabaseCommit,
            error,
        ));
    }

    let commit_result = transaction.commit().await;
    match commit_fault {
        Some(CommitOutcomeFault::VisibleAfterCommit) => {
            commit_result.map_err(|error| {
                super::phased_item_error(
                    update.index,
                    CentralUpdateFailurePhase::DatabaseCommit,
                    error.into(),
                )
            })?;
            Ok(Err(sqlx::Error::Protocol(
                "injected commit-unknown after a visible db_committed row".into(),
            )))
        }
        Some(CommitOutcomeFault::InvisibleWithoutCommit) | None => Ok(commit_result),
    }
}
