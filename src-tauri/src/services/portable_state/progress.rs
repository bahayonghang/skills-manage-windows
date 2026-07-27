use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use super::error::PortableStateError;
use super::types::{
    CancelFlag, PortabilityProgressUpdate, SkillportStatePortabilityPhase,
    SkillportStatePortabilityProgressPayload, SkillportStatePortabilityStatus,
    PORTABILITY_PROGRESS_EVENT,
};

pub(crate) fn check_cancel(cancel: Option<&CancelFlag>) -> Result<(), PortableStateError> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::SeqCst)) {
        Err(PortableStateError::Cancelled)
    } else {
        Ok(())
    }
}

pub(crate) fn emit_portability_step(
    app: Option<&AppHandle>,
    job_id: &str,
    phase: SkillportStatePortabilityPhase,
    total: usize,
    completed: usize,
    message: Option<&str>,
    current_item: Option<&str>,
) {
    if let Some(app) = app {
        emit_portability_progress(
            app,
            job_id,
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

pub(crate) fn emit_portability_progress(
    app: &AppHandle,
    job_id: &str,
    update: PortabilityProgressUpdate<'_>,
) {
    let payload = SkillportStatePortabilityProgressPayload {
        job_id: job_id.to_string(),
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
