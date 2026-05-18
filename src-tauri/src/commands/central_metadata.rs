//! Tauri IPC shells for Central metadata and AI tagging commands.
//!
//! Repository/tag CRUD stays in this command module. AI tagging orchestration
//! lives in `crate::services::ai_tagging` and is re-exported here to keep IPC
//! payload type paths stable.

use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::{
    db::{self, SkillAiTagReview, SkillRepository, SkillRepositoryWithStats, SkillTag},
    services::ai_tagging,
    AppState,
};

use crate::services::ai_tagging::AI_TAG_PROGRESS_EVENT;
pub use crate::services::ai_tagging::{
    AiTagProgressPayload, AiTagProgressStatus, SkillTagSuggestion, SkillTagSuggestionResult,
};

#[tauri::command]
pub async fn get_skill_repositories(
    state: State<'_, AppState>,
) -> Result<Vec<SkillRepositoryWithStats>, String> {
    let pool = state.active_db().await?;
    db::get_skill_repositories_with_stats(&pool).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_skill_repository(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    source_type: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
    branch: Option<String>,
    url: Option<String>,
    is_unknown: Option<bool>,
) -> Result<SkillRepository, String> {
    let pool = state.active_db().await?;
    db::create_or_update_skill_repository(
        &pool,
        id.as_deref(),
        &name,
        source_type.as_deref().unwrap_or("manual"),
        owner.as_deref(),
        repo.as_deref(),
        branch.as_deref(),
        url.as_deref(),
        is_unknown.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn assign_skills_to_repository(
    state: State<'_, AppState>,
    repository_id: String,
    skill_ids: Vec<String>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    db::assign_skills_to_repository(&pool, &repository_id, &skill_ids, None).await
}

#[tauri::command]
pub async fn set_skill_repository_pinned(
    state: State<'_, AppState>,
    repository_id: String,
    pinned: bool,
) -> Result<SkillRepository, String> {
    let pool = state.active_db().await?;
    db::set_skill_repository_pinned(&pool, &repository_id, pinned).await
}

#[tauri::command]
pub async fn get_skill_tags(state: State<'_, AppState>) -> Result<Vec<SkillTag>, String> {
    let pool = state.active_db().await?;
    db::get_skill_tags(&pool).await
}

#[tauri::command]
pub async fn create_skill_tag(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> Result<SkillTag, String> {
    let pool = state.active_db().await?;
    db::create_skill_tag(&pool, &name, description.as_deref(), color.as_deref()).await
}

#[tauri::command]
pub async fn assign_skill_tags(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    tag_ids: Vec<String>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    db::assign_skill_tags(&pool, &skill_ids, &tag_ids, "manual", None, None).await
}

#[tauri::command]
pub async fn get_pending_ai_tag_reviews(
    state: State<'_, AppState>,
) -> Result<Vec<SkillAiTagReview>, String> {
    let pool = state.active_db().await?;
    db::get_pending_ai_tag_reviews(&pool).await
}

#[tauri::command]
pub async fn accept_ai_tag_review(
    state: State<'_, AppState>,
    skill_id: String,
    tag_ids: Vec<String>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    db::accept_ai_tag_reviews(&pool, &skill_id, &tag_ids).await
}

#[tauri::command]
pub async fn skip_ai_tag_review(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    db::skip_ai_tag_reviews(&pool, &skill_id).await
}

#[tauri::command]
pub async fn suggest_skill_tags(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<SkillTagSuggestion>, String> {
    let pool = state.active_db().await?;
    ai_tagging::suggest_skill_tags_for_skill_id(&pool, state.secrets.as_ref(), skill_id).await
}

#[tauri::command]
pub async fn bulk_suggest_skill_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<Vec<SkillTagSuggestionResult>, String> {
    let app = Arc::new(app);
    let job_id = Uuid::new_v4().to_string();
    let cancel_flag = state.ai_tag_jobs.register(&job_id);
    let pool = state.active_db().await?;
    let result = ai_tagging::bulk_suggest_skill_tags_impl(
        &pool,
        state.secrets.as_ref(),
        skill_ids,
        job_id.clone(),
        cancel_flag,
        move |payload| {
            let _ = app.emit(AI_TAG_PROGRESS_EVENT, payload);
        },
    )
    .await;
    state.ai_tag_jobs.finish(&job_id);
    result
}

#[tauri::command]
pub async fn cancel_ai_tag_job(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    if state.ai_tag_jobs.cancel(&job_id) {
        Ok(())
    } else {
        Err(format!("AI tag job '{}' not found", job_id))
    }
}
