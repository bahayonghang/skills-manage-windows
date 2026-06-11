//! AI tagging service for Central skill metadata.
//!
//! Tauri IPC shells live in `crate::commands::central_metadata`; this service
//! owns AI provider calls, rate limiting, progress aggregation, and persistence
//! of suggested / pending-review skill tags.

mod prompt;
mod rate_limit;
mod types;

#[cfg(test)]
mod tests;

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Duration, Instant};

use crate::{
    db::{self, DbPool},
    secrets::SecretStore,
    services::ai_provider,
};

use prompt::suggest_skill_tags_for_skill;
#[cfg(test)]
pub(crate) use prompt::{build_tagging_prompt, map_ai_suggestions, parse_ai_tag_suggestions};
use rate_limit::{get_ai_tag_rate_settings, is_ai_rate_limit_error};
pub(crate) use types::AI_TAG_PROGRESS_EVENT;
use types::{
    AiTagCounters, AiTagRateLimiter, AiTagRunControl, AiTagRunningNotifier, AiTaggingContext,
    AI_TAG_AUTO_APPLY_CONFIDENCE,
};
pub use types::{
    AiTagProgressPayload, AiTagProgressStatus, SkillTagSuggestion, SkillTagSuggestionResult,
};

pub async fn suggest_skill_tags_for_skill_id(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    skill_id: String,
) -> Result<Vec<SkillTagSuggestion>, String> {
    let context = Arc::new(prepare_ai_tagging_context(pool, secrets).await?);
    let result = process_skill_for_ai_tags(
        context,
        skill_id,
        None::<AiTagRunningNotifier<fn(AiTagProgressPayload)>>,
        None,
    )
    .await;
    if result.succeeded {
        Ok(result.suggestions)
    } else {
        Err(result
            .error
            .unwrap_or_else(|| "AI tagging failed".to_string()))
    }
}

pub async fn bulk_suggest_skill_tags_impl<F>(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    skill_ids: Vec<String>,
    job_id: String,
    cancel_flag: Arc<AtomicBool>,
    emit_progress: F,
) -> Result<Vec<SkillTagSuggestionResult>, String>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let context = Arc::new(prepare_ai_tagging_context(pool, secrets).await?);
    let rate_settings = get_ai_tag_rate_settings(pool).await;
    let run_control = AiTagRunControl {
        cancel_flag,
        rate_limiter: Arc::new(AiTagRateLimiter {
            interval: Duration::from_millis(rate_settings.interval_ms),
            next_request_at: AsyncMutex::new(Instant::now()),
        }),
        rate_settings,
    };
    let total = skill_ids.len();
    let counters = Arc::new(Mutex::new(AiTagCounters::default()));
    let emit_progress = Arc::new(emit_progress);
    let concurrency_limit = run_control.rate_settings.concurrency_limit;

    emit_progress(AiTagProgressPayload {
        job_id: job_id.clone(),
        skill_id: None,
        skill_name: None,
        status: AiTagProgressStatus::Started,
        total,
        completed: 0,
        succeeded: 0,
        failed: 0,
        suggestions: None,
        error: None,
        low_confidence_count: 0,
    });

    let results = stream::iter(skill_ids.into_iter().map(|skill_id| {
        let context = Arc::clone(&context);
        let counters = Arc::clone(&counters);
        let emit_progress = Arc::clone(&emit_progress);
        let job_id = job_id.clone();
        let run_control = run_control.clone();

        async move {
            let result = process_skill_for_ai_tags(
                context,
                skill_id.clone(),
                Some(AiTagRunningNotifier {
                    job_id: job_id.clone(),
                    total,
                    counters: Arc::clone(&counters),
                    emit_progress: Arc::clone(&emit_progress),
                }),
                Some(run_control),
            )
            .await;

            let snapshot = update_counters(&counters, &result);
            emit_progress(AiTagProgressPayload {
                job_id,
                skill_id: Some(result.skill_id.clone()),
                skill_name: result.skill_name.clone(),
                status: if result.succeeded {
                    AiTagProgressStatus::Succeeded
                } else {
                    AiTagProgressStatus::Failed
                },
                total,
                completed: snapshot.completed,
                succeeded: snapshot.succeeded,
                failed: snapshot.failed,
                suggestions: Some(result.suggestions.clone()),
                error: result.error.clone(),
                low_confidence_count: snapshot.low_confidence_count,
            });

            result
        }
    }))
    .buffer_unordered(concurrency_limit)
    .collect::<Vec<_>>()
    .await;

    let snapshot = counters_snapshot(&counters, "final progress emit");
    emit_progress(AiTagProgressPayload {
        job_id: job_id.clone(),
        skill_id: None,
        skill_name: None,
        status: if run_control.is_cancelled() {
            AiTagProgressStatus::Cancelled
        } else {
            AiTagProgressStatus::Completed
        },
        total,
        completed: snapshot.completed,
        succeeded: snapshot.succeeded,
        failed: snapshot.failed,
        suggestions: None,
        error: if run_control.is_cancelled() {
            Some("AI tagging canceled".to_string())
        } else {
            None
        },
        low_confidence_count: snapshot.low_confidence_count,
    });
    Ok(results)
}

fn counters_snapshot(counters: &Arc<Mutex<AiTagCounters>>, context: &str) -> AiTagCounters {
    match counters.lock() {
        Ok(guard) => guard.clone(),
        Err(error) => {
            tracing::warn!(context = %context, error = %error, "AI tag counter mutex poisoned");
            AiTagCounters::default()
        }
    }
}

fn update_counters(
    counters: &Arc<Mutex<AiTagCounters>>,
    result: &SkillTagSuggestionResult,
) -> AiTagCounters {
    match counters.lock() {
        Ok(mut guard) => {
            guard.completed += 1;
            if result.succeeded {
                guard.succeeded += 1;
            } else {
                guard.failed += 1;
            }
            guard.low_confidence_count += result.low_confidence_count;
            guard.clone()
        }
        Err(error) => {
            tracing::warn!(error = %error, "AI tag counter mutex poisoned during update");
            AiTagCounters::default()
        }
    }
}

async fn prepare_ai_tagging_context(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<AiTaggingContext, String> {
    let config = ai_provider::resolve_ai_provider_config(pool).await;
    let api_key = ai_provider::get_ai_api_key_for_provider(pool, secrets, &config.provider)
        .await?
        .ok_or_else(|| {
            ai_provider::coded_error(
                ai_provider::AI_MISSING_API_KEY,
                "Configure an AI API key in Settings before running AI tagging.",
            )
        })?;
    let api_url = config.api_url;
    let protocol = config.protocol;
    let model = config.model;
    let tags = db::get_skill_tags(pool).await?;
    if tags.is_empty() {
        return Err("No candidate tags are available.".to_string());
    }
    let client = {
        let builder = Client::builder().user_agent(crate::commands::APP_USER_AGENT);
        #[cfg(test)]
        let builder = builder.no_proxy();
        builder.build().map_err(|e| {
            ai_provider::coded_error_with_details(
                ai_provider::AI_CLIENT_BUILD_FAILED,
                "Failed to initialize the AI HTTP client.",
                e.to_string(),
            )
        })?
    };

    Ok(AiTaggingContext {
        pool: pool.clone(),
        api_key,
        api_url,
        protocol,
        model,
        tags: Arc::new(tags),
        client,
    })
}

async fn process_skill_for_ai_tags<F>(
    context: Arc<AiTaggingContext>,
    skill_id: String,
    running_notifier: Option<AiTagRunningNotifier<F>>,
    run_control: Option<AiTagRunControl>,
) -> SkillTagSuggestionResult
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    match try_process_skill_for_ai_tags(&context, &skill_id, running_notifier, run_control).await {
        Ok(result) => result,
        Err(error) => SkillTagSuggestionResult {
            skill_id,
            skill_name: None,
            suggestions: Vec::new(),
            succeeded: false,
            error: Some(error),
            low_confidence_count: 0,
        },
    }
}

async fn try_process_skill_for_ai_tags<F>(
    context: &AiTaggingContext,
    skill_id: &str,
    running_notifier: Option<AiTagRunningNotifier<F>>,
    run_control: Option<AiTagRunControl>,
) -> Result<SkillTagSuggestionResult, String>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    if let Some(control) = run_control.as_ref() {
        if control.is_cancelled() {
            return Err("AI tagging canceled".to_string());
        }
    }

    let skill = db::get_skill_by_id(&context.pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    if let Some(notifier) = running_notifier.as_ref() {
        notifier.emit(&skill.id, &skill.name);
    }

    if let Some(control) = run_control.as_ref() {
        control.wait_for_rate_limit().await?;
    }

    let suggestions = match suggest_skill_tags_for_skill(context, &skill).await {
        Ok(suggestions) => suggestions,
        Err(error) => {
            if run_control
                .as_ref()
                .is_some_and(|control| control.rate_settings.stop_on_rate_limit)
                && is_ai_rate_limit_error(&error)
            {
                if let Some(control) = run_control.as_ref() {
                    control.cancel();
                }
            }
            return Err(error);
        }
    };
    let (auto_apply, pending_review): (Vec<_>, Vec<_>) = suggestions
        .iter()
        .cloned()
        .partition(|suggestion| suggestion.confidence >= AI_TAG_AUTO_APPLY_CONFIDENCE);

    persist_ai_suggestions(&context.pool, skill_id, &auto_apply).await?;
    persist_ai_review_suggestions(&context.pool, skill_id, &pending_review).await?;

    Ok(SkillTagSuggestionResult {
        skill_id: skill_id.to_string(),
        skill_name: Some(skill.name),
        suggestions,
        succeeded: true,
        error: None,
        low_confidence_count: pending_review.len(),
    })
}

async fn persist_ai_suggestions(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[SkillTagSuggestion],
) -> Result<(), String> {
    let rows = suggestions
        .iter()
        .map(|suggestion| {
            (
                suggestion.tag.id.clone(),
                suggestion.confidence,
                suggestion.reason.clone(),
            )
        })
        .collect::<Vec<_>>();
    db::replace_skill_ai_tags(pool, skill_id, &rows).await
}

async fn persist_ai_review_suggestions(
    pool: &DbPool,
    skill_id: &str,
    suggestions: &[SkillTagSuggestion],
) -> Result<(), String> {
    let rows = suggestions
        .iter()
        .map(|suggestion| {
            (
                suggestion.tag.id.clone(),
                suggestion.confidence,
                suggestion.reason.clone(),
            )
        })
        .collect::<Vec<_>>();
    db::replace_pending_ai_tag_reviews(pool, skill_id, &rows).await
}
