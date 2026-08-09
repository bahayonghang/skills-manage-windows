mod error;
mod fs;
mod path;
mod reconcile;
mod recovery;
mod types;

pub use error::CentralOperationError;
pub use reconcile::{preview_prepared_delete_reconciliation, reconcile_prepared_delete};
pub use recovery::{list_pending_operations, recover_pending_operations, retry_operation};
pub(crate) use recovery::{
    recover_pending_delete_operation_with_transport,
    recover_pending_delete_operations_with_transport,
};
pub use types::{
    CopyProjection, DeleteManifest, ManagedPath, OperationKind, OperationManifest, OperationPhase,
    PendingOperationSummary, PreparedDeleteReconciliationPreview, UpdateManifest, MANIFEST_VERSION,
};

pub(crate) use fs::{
    build_local_delete_manifest, build_remote_delete_manifest, finalize_delete_local,
    finalize_delete_remote, stage_delete_local, stage_delete_remote,
};
