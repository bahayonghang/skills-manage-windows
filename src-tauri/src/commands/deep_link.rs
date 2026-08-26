use tauri::{AppHandle, State};

use crate::services::deep_link::{mark_import_intent_ready, ImportIntentState};

#[tauri::command]
pub fn mark_import_intent_frontend_ready(
    app: AppHandle,
    state: State<'_, ImportIntentState>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "mark_import_intent_frontend_ready",
        mark_import_intent_ready(&app, state.inner()).map_err(|error| error.to_string())
    )
}
