//! Tauri IPC shells for central store location migration.
//!
//! Business logic lives in `crate::services::central_store_location`. This
//! module keeps the existing command names and payload shapes stable while
//! translating `State<AppState>` into service inputs.

use serde::Deserialize;
use tauri::State;

use crate::services::central_store_location::{
    apply_central_store_location_change_impl, ensure_local_target,
    preview_central_store_location_change_impl, CentralStoreLocationChangeResult,
    CentralStoreLocationPreview,
};
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationPreviewRequest {
    pub target_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreLocationApplyRequest {
    pub target_path: String,
    pub overwrite_existing: bool,
}

#[tauri::command]
pub async fn preview_central_store_location_change(
    state: State<'_, AppState>,
    request: CentralStoreLocationPreviewRequest,
) -> Result<CentralStoreLocationPreview, String> {
    let request_context = state.resolve_target_context().await?;
    let target = request_context.target().clone();
    ensure_local_target(&target).map_err(|e| e.to_string())?;
    let pool = request_context.db().clone();
    preview_central_store_location_change_impl(&pool, &request.target_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_central_store_location_change(
    state: State<'_, AppState>,
    request: CentralStoreLocationApplyRequest,
) -> Result<CentralStoreLocationChangeResult, String> {
    let request_context = state.resolve_target_context().await?;
    let target = request_context.target().clone();
    ensure_local_target(&target).map_err(|e| e.to_string())?;
    let pool = request_context.db().clone();
    apply_central_store_location_change_impl(
        &pool,
        &request.target_path,
        request.overwrite_existing,
    )
    .await
    .map_err(|e| e.to_string())
}
