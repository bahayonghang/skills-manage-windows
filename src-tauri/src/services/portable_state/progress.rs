use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use super::types::{
    CancelFlag, PortabilityProgressUpdate, SkillportStatePortabilityPhase,
    SkillportStatePortabilityProgressPayload, SkillportStatePortabilityStatus,
    PORTABILITY_CANCELLED_MESSAGE, PORTABILITY_PROGRESS_EVENT, STATUS_CANCELLED,
};

pub(crate) fn check_cancel(cancel: Option<&CancelFlag>) -> Result<(), String> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::SeqCst)) {
        Err(PORTABILITY_CANCELLED_MESSAGE.to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn is_cancelled_error(error: &str) -> bool {
    error.contains(PORTABILITY_CANCELLED_MESSAGE) || error == STATUS_CANCELLED
}

pub(crate) fn emit_portability_step(
    app: Option<&AppHandle>,
    phase: SkillportStatePortabilityPhase,
    total: usize,
    completed: usize,
    message: Option<&str>,
    current_item: Option<&str>,
) {
    if let Some(app) = app {
        emit_portability_progress(
            app,
            PortabilityProgressUpdate {
                phase,
                status: SkillportStatePortabilityStatus::Running,
                total,
                completed,
                message,
                current_item,
                error: None,
            },
        );
    }
}

pub(crate) fn emit_portability_progress(app: &AppHandle, update: PortabilityProgressUpdate<'_>) {
    let payload = SkillportStatePortabilityProgressPayload {
        phase: update.phase,
        status: update.status,
        total: update.total,
        completed: update.completed,
        message: update.message.map(str::to_string),
        current_item: update.current_item.map(str::to_string),
        error: update.error.map(str::to_string),
    };
    let _ = app.emit(PORTABILITY_PROGRESS_EVENT, payload);
}
