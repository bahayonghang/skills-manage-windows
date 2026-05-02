use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

use crate::{
    db::{
        self, DbPool, Skill, SkillAiTagReview, SkillRepository, SkillRepositoryWithStats, SkillTag,
        UNCATEGORIZED_TAG_ID,
    },
    AppState,
};

const AI_TAG_PROGRESS_EVENT: &str = "central://ai-tag-progress";
const DEFAULT_AI_TAGGING_CONCURRENCY_LIMIT: usize = 1;
const DEFAULT_AI_TAGGING_INTERVAL_MS: u64 = 4_000;
const DEFAULT_AI_TAG_STOP_ON_RATE_LIMIT: bool = true;
const AI_TAG_AUTO_APPLY_CONFIDENCE: f64 = 0.7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTagSuggestion {
    pub skill_id: String,
    pub tag: SkillTag,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTagSuggestionResult {
    pub skill_id: String,
    pub skill_name: Option<String>,
    pub suggestions: Vec<SkillTagSuggestion>,
    pub succeeded: bool,
    pub error: Option<String>,
    pub low_confidence_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiTagProgressStatus {
    Started,
    Running,
    Succeeded,
    Failed,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTagProgressPayload {
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    pub status: AiTagProgressStatus,
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<SkillTagSuggestion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub low_confidence_count: usize,
}

#[derive(Debug, Clone, Default)]
struct AiTagCounters {
    completed: usize,
    succeeded: usize,
    failed: usize,
    low_confidence_count: usize,
}

#[derive(Debug, Clone)]
struct AiTagRateSettings {
    concurrency_limit: usize,
    interval_ms: u64,
    stop_on_rate_limit: bool,
}

#[derive(Debug)]
struct AiTagRateLimiter {
    interval: Duration,
    next_request_at: AsyncMutex<Instant>,
}

#[derive(Debug, Clone)]
struct AiTagRunControl {
    cancel_flag: Arc<AtomicBool>,
    rate_settings: AiTagRateSettings,
    rate_limiter: Arc<AiTagRateLimiter>,
}

impl AiTagRunControl {
    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    async fn wait_for_rate_limit(&self) -> Result<(), String> {
        if self.is_cancelled() {
            return Err("AI tagging canceled".to_string());
        }

        let mut next_request_at = self.rate_limiter.next_request_at.lock().await;
        let now = Instant::now();
        if *next_request_at > now {
            let wait_for = *next_request_at - now;
            sleep_cancelable(wait_for, &self.cancel_flag).await?;
        }

        *next_request_at = Instant::now() + self.rate_limiter.interval;
        Ok(())
    }
}

async fn sleep_cancelable(duration: Duration, cancel_flag: &AtomicBool) -> Result<(), String> {
    let deadline = Instant::now() + duration;
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err("AI tagging canceled".to_string());
        }

        let now = Instant::now();
        if now >= deadline {
            return Ok(());
        }

        sleep((deadline - now).min(Duration::from_millis(200))).await;
    }
}

#[derive(Debug, Clone)]
struct AiTaggingContext {
    pool: DbPool,
    api_key: String,
    api_url: String,
    model: String,
    tags: Arc<Vec<SkillTag>>,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct RawAiTagSuggestion {
    tag: String,
    confidence: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAiTagSuggestionEnvelope {
    tags: Vec<RawAiTagSuggestion>,
}

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
    let context = Arc::new(prepare_ai_tagging_context(&pool).await?);
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
    let result = bulk_suggest_skill_tags_impl(
        &pool,
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

async fn bulk_suggest_skill_tags_impl<F>(
    pool: &DbPool,
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

    let context = Arc::new(prepare_ai_tagging_context(pool).await?);
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

    let snapshot = counters
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
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

struct AiTagRunningNotifier<F>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    job_id: String,
    total: usize,
    counters: Arc<Mutex<AiTagCounters>>,
    emit_progress: Arc<F>,
}

impl<F> AiTagRunningNotifier<F>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    fn emit(&self, skill_id: &str, skill_name: &str) {
        let snapshot = self
            .counters
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        (self.emit_progress)(AiTagProgressPayload {
            job_id: self.job_id.clone(),
            skill_id: Some(skill_id.to_string()),
            skill_name: Some(skill_name.to_string()),
            status: AiTagProgressStatus::Running,
            total: self.total,
            completed: snapshot.completed,
            succeeded: snapshot.succeeded,
            failed: snapshot.failed,
            suggestions: None,
            error: None,
            low_confidence_count: snapshot.low_confidence_count,
        });
    }
}

fn update_counters(
    counters: &Arc<Mutex<AiTagCounters>>,
    result: &SkillTagSuggestionResult,
) -> AiTagCounters {
    let mut guard = counters.lock().expect("AI tag counter mutex poisoned");
    guard.completed += 1;
    if result.succeeded {
        guard.succeeded += 1;
    } else {
        guard.failed += 1;
    }
    guard.low_confidence_count += result.low_confidence_count;
    guard.clone()
}

async fn prepare_ai_tagging_context(pool: &DbPool) -> Result<AiTaggingContext, String> {
    let api_key = get_ai_setting(pool, "ai_api_key")
        .await
        .ok_or_else(|| "请先在设置中配置 AI API Key".to_string())?;
    let api_url = get_ai_setting(pool, "ai_api_url")
        .await
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    let model = get_ai_setting(pool, "ai_model")
        .await
        .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());
    let tags = db::get_skill_tags(pool).await?;
    if tags.is_empty() {
        return Err("No candidate tags are available.".to_string());
    }
    let client = Client::builder()
        .user_agent(crate::commands::APP_USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(AiTaggingContext {
        pool: pool.clone(),
        api_key,
        api_url,
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

async fn get_ai_setting(pool: &DbPool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn get_ai_tag_rate_settings(pool: &DbPool) -> AiTagRateSettings {
    let concurrency_limit = get_ai_setting(pool, "ai_tag_concurrency")
        .await
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_AI_TAGGING_CONCURRENCY_LIMIT)
        .clamp(1, 8);
    let interval_ms = get_ai_setting(pool, "ai_tag_interval_ms")
        .await
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_AI_TAGGING_INTERVAL_MS)
        .min(60_000);
    let stop_on_rate_limit = get_ai_setting(pool, "ai_tag_stop_on_rate_limit")
        .await
        .map(|value| parse_bool_setting(&value, DEFAULT_AI_TAG_STOP_ON_RATE_LIMIT))
        .unwrap_or(DEFAULT_AI_TAG_STOP_ON_RATE_LIMIT);

    AiTagRateSettings {
        concurrency_limit,
        interval_ms,
        stop_on_rate_limit,
    }
}

fn parse_bool_setting(value: &str, fallback: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn is_ai_rate_limit_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("429")
        || normalized.contains("too many requests")
        || normalized.contains("rate limit")
        || normalized.contains("ratelimit")
}

async fn suggest_skill_tags_for_skill(
    context: &AiTaggingContext,
    skill: &Skill,
) -> Result<Vec<SkillTagSuggestion>, String> {
    let content = skill
        .content
        .clone()
        .or_else(|| std::fs::read_to_string(&skill.file_path).ok())
        .unwrap_or_default();
    let prompt = build_tagging_prompt(
        &skill.name,
        skill.description.as_deref(),
        &content,
        &context.tags,
    );
    let raw = call_ai_for_tagging(
        &context.client,
        &context.api_url,
        &context.api_key,
        &context.model,
        &prompt,
    )
    .await?;
    let parsed = parse_ai_tag_suggestions(&raw)?;
    map_ai_suggestions(&skill.id, &context.tags, parsed)
}

fn build_tagging_prompt(
    name: &str,
    description: Option<&str>,
    content: &str,
    tags: &[SkillTag],
) -> String {
    let candidates = tags
        .iter()
        .map(|tag| format!("- {} ({})", tag.name, tag.id))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = content.chars().take(4_000).collect::<String>();

    format!(
        "你是 SkillPort 的本地分类器。请只从候选大类中选择 1 到 3 个标签。\n\
         输出必须是 JSON，不要解释额外文本。\n\
         JSON 格式：{{\"tags\":[{{\"tag\":\"标签名或ID\",\"confidence\":0.0,\"reason\":\"不超过20字\"}}]}}\n\n\
         候选大类：\n{candidates}\n\n\
         Skill 名称：{name}\n\
         Description：{}\n\
         SKILL.md 摘要：\n{}",
        description.unwrap_or(""),
        summary
    )
}

async fn call_ai_for_tagging(
    client: &Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let is_openai = api_url.to_ascii_lowercase().contains("/chat/completions");
    let body = if is_openai {
        serde_json::json!({
            "model": model,
            "temperature": 0.1,
            "messages": [{ "role": "user", "content": prompt }],
        })
    } else {
        serde_json::json!({
            "model": model,
            "max_tokens": 600,
            "messages": [{ "role": "user", "content": prompt }],
        })
    };

    let mut request = client.post(api_url).json(&body);
    request = if is_openai {
        request.header("authorization", format!("Bearer {}", api_key))
    } else {
        request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
    };

    let response = request
        .send()
        .await
        .map_err(|e| format!("AI tagging request failed: {}", e))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read AI tagging response: {}", e))?;
    if !status.is_success() {
        if status.as_u16() == 429 {
            return Err(format!(
                "AI tagging returned 429 Too Many Requests: {text}\n请降低 AI Tag 并发或增大请求间隔。"
            ));
        }
        return Err(format!("AI tagging returned {}: {}", status, text));
    }

    extract_ai_response_text(&text, is_openai)
}

fn extract_ai_response_text(response_text: &str, is_openai: bool) -> Result<String, String> {
    let value: Value = serde_json::from_str(response_text)
        .map_err(|e| format!("AI tagging response is not JSON: {}", e))?;
    if is_openai {
        return value["choices"][0]["message"]["content"]
            .as_str()
            .map(ToString::to_string)
            .ok_or_else(|| "AI tagging response did not include message content.".to_string());
    }

    value["content"]
        .as_array()
        .and_then(|items| {
            items.iter().find_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
        })
        .ok_or_else(|| "AI tagging response did not include text content.".to_string())
}

fn parse_ai_tag_suggestions(raw: &str) -> Result<Vec<RawAiTagSuggestion>, String> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(envelope) = serde_json::from_str::<RawAiTagSuggestionEnvelope>(cleaned) {
        return Ok(envelope.tags);
    }
    if let Ok(list) = serde_json::from_str::<Vec<RawAiTagSuggestion>>(cleaned) {
        return Ok(list);
    }

    let start = cleaned
        .find('{')
        .or_else(|| cleaned.find('['))
        .ok_or_else(|| "AI tagging response did not include JSON.".to_string())?;
    let end = cleaned
        .rfind('}')
        .or_else(|| cleaned.rfind(']'))
        .ok_or_else(|| "AI tagging response did not include complete JSON.".to_string())?;
    let json_slice = &cleaned[start..=end];
    serde_json::from_str::<RawAiTagSuggestionEnvelope>(json_slice)
        .map(|envelope| envelope.tags)
        .or_else(|_| serde_json::from_str::<Vec<RawAiTagSuggestion>>(json_slice))
        .map_err(|e| format!("Failed to parse AI tagging JSON: {}", e))
}

fn map_ai_suggestions(
    skill_id: &str,
    tags: &[SkillTag],
    raw: Vec<RawAiTagSuggestion>,
) -> Result<Vec<SkillTagSuggestion>, String> {
    let mut suggestions = Vec::new();
    for item in raw {
        let key = item.tag.trim();
        let Some(tag) = tags
            .iter()
            .find(|tag| tag.id == key || tag.name == key)
            .cloned()
        else {
            continue;
        };
        let confidence = item.confidence.unwrap_or(0.6).clamp(0.0, 1.0);
        suggestions.push(SkillTagSuggestion {
            skill_id: skill_id.to_string(),
            tag,
            confidence,
            reason: item.reason.unwrap_or_else(|| "AI 自动标注".to_string()),
        });
    }

    if suggestions.is_empty() {
        let fallback = tags
            .iter()
            .find(|tag| tag.id == UNCATEGORIZED_TAG_ID)
            .cloned()
            .ok_or_else(|| "AI tagging returned no usable candidate tags.".to_string())?;
        suggestions.push(SkillTagSuggestion {
            skill_id: skill_id.to_string(),
            tag: fallback,
            confidence: 0.2,
            reason: "未命中候选大类".to_string(),
        });
    }

    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    use super::{
        bulk_suggest_skill_tags_impl, map_ai_suggestions, parse_ai_tag_suggestions,
        AiTagProgressPayload, AiTagProgressStatus,
    };
    use crate::db::{self, DbPool, Skill, SkillTag, UNCATEGORIZED_TAG_ID};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::{sleep, Duration};

    fn tag(id: &str, name: &str) -> SkillTag {
        SkillTag {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            color: None,
            is_builtin: true,
            created_at: "2026-04-24T00:00:00Z".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    fn make_skill(id: &str, name: &str) -> Skill {
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: Some(format!("{name} description")),
            file_path: format!("/tmp/{id}/SKILL.md"),
            canonical_path: Some(format!("/tmp/{id}")),
            is_central: true,
            source: Some("test".to_string()),
            content: Some(format!("# {name}\nTest skill content")),
            scanned_at: "2026-04-24T00:00:00Z".to_string(),
        }
    }

    async fn setup_test_db() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .expect("db");
        db::init_database(&pool).await.expect("init");
        pool
    }

    async fn configure_ai(pool: &DbPool, api_url: &str) {
        db::set_setting(pool, "ai_api_key", "test-key")
            .await
            .expect("api key");
        db::set_setting(pool, "ai_api_url", api_url)
            .await
            .expect("api url");
        db::set_setting(pool, "ai_model", "test-model")
            .await
            .expect("model");
        db::set_setting(pool, "ai_tag_concurrency", "4")
            .await
            .expect("tag concurrency");
        db::set_setting(pool, "ai_tag_interval_ms", "0")
            .await
            .expect("tag interval");
        db::set_setting(pool, "ai_tag_stop_on_rate_limit", "true")
            .await
            .expect("tag stop on rate limit");
    }

    async fn spawn_ai_server(
        response_text: &'static str,
        fail_first: bool,
    ) -> (String, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("addr");
        let current = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let request_count = Arc::new(AtomicUsize::new(0));
        let current_for_task = Arc::clone(&current);
        let max_for_task = Arc::clone(&max_seen);
        let count_for_task = Arc::clone(&request_count);

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let current = Arc::clone(&current_for_task);
                let max_seen = Arc::clone(&max_for_task);
                let request_count = Arc::clone(&count_for_task);
                tokio::spawn(async move {
                    let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, Ordering::SeqCst);

                    let mut buffer = [0_u8; 4096];
                    let _ = socket.read(&mut buffer).await;
                    sleep(Duration::from_millis(80)).await;

                    let index = request_count.fetch_add(1, Ordering::SeqCst);
                    let (status, body) = if fail_first && index == 0 {
                        (
                            "500 Internal Server Error",
                            "{\"error\":\"boom\"}".to_string(),
                        )
                    } else {
                        let escaped = response_text.replace('"', "\\\"");
                        (
                            "200 OK",
                            format!("{{\"content\":[{{\"text\":\"{}\"}}]}}", escaped),
                        )
                    };
                    let response = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                    current.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });

        (format!("http://{address}/v1/messages"), current, max_seen)
    }

    #[test]
    fn parses_tag_json_envelope() {
        let parsed = parse_ai_tag_suggestions(
            r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.91,"reason":"开发工具"}]}"#,
        )
        .expect("parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].tag, "编程与 Agent 工程");
    }

    #[test]
    fn maps_unknown_ai_tags_to_uncategorized() {
        let tags = vec![
            tag("programming-agent-engineering", "编程与 Agent 工程"),
            tag("uncategorized", "未分类"),
        ];
        let parsed = parse_ai_tag_suggestions(
            r#"{"tags":[{"tag":"不存在","confidence":0.8,"reason":"测试"}]}"#,
        )
        .expect("parse");
        let mapped = map_ai_suggestions("skill-a", &tags, parsed).expect("map");
        assert_eq!(mapped[0].tag.id, "uncategorized");
    }

    #[tokio::test]
    async fn bulk_ai_tagging_emits_progress_limits_parallelism_and_continues_on_failure() {
        let pool = setup_test_db().await;
        let response = r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.9,"reason":"开发工具"},{"tag":"未分类","confidence":0.4,"reason":"不确定"}]}"#;
        let (api_url, _current, max_seen) = spawn_ai_server(response, true).await;
        configure_ai(&pool, &api_url).await;

        for index in 0..6 {
            db::upsert_skill(
                &pool,
                &make_skill(&format!("skill-{index}"), &format!("Skill {index}")),
            )
            .await
            .expect("skill");
        }

        let events: Arc<Mutex<Vec<AiTagProgressPayload>>> = Arc::new(Mutex::new(Vec::new()));
        let events_for_emit = Arc::clone(&events);
        let results = bulk_suggest_skill_tags_impl(
            &pool,
            (0..6).map(|index| format!("skill-{index}")).collect(),
            "job-test".to_string(),
            Arc::new(AtomicBool::new(false)),
            move |payload| {
                events_for_emit.lock().expect("events").push(payload);
            },
        )
        .await
        .expect("bulk");

        assert_eq!(results.len(), 6);
        assert!(results.iter().any(|result| !result.succeeded));
        assert!(results.iter().any(|result| result.succeeded));
        assert!(max_seen.load(Ordering::SeqCst) <= 4);
        assert!(max_seen.load(Ordering::SeqCst) > 1);

        {
            let captured = events.lock().expect("events");
            assert_eq!(
                captured.first().map(|event| event.status),
                Some(AiTagProgressStatus::Started)
            );
            assert_eq!(
                captured.last().map(|event| event.status),
                Some(AiTagProgressStatus::Completed)
            );
            assert!(captured
                .iter()
                .any(|event| event.status == AiTagProgressStatus::Running));
            assert!(captured
                .iter()
                .any(|event| event.status == AiTagProgressStatus::Failed));
        }

        let tags = db::get_skill_tags_for_skill(&pool, "skill-1")
            .await
            .expect("tags");
        assert!(tags
            .iter()
            .any(|tag| tag.id == "programming-agent-engineering"));
        let reviews = db::get_pending_ai_tag_reviews(&pool)
            .await
            .expect("reviews");
        assert!(reviews
            .iter()
            .any(|review| review.tag.id == UNCATEGORIZED_TAG_ID));
    }

    #[tokio::test]
    async fn bulk_ai_tagging_requires_configuration_before_writing() {
        let pool = setup_test_db().await;
        db::upsert_skill(&pool, &make_skill("skill-a", "Skill A"))
            .await
            .expect("skill");

        let result = bulk_suggest_skill_tags_impl(
            &pool,
            vec!["skill-a".to_string()],
            "job-test".to_string(),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await;

        assert!(result.expect_err("missing setting").contains("AI API Key"));
        let tags = db::get_skill_tags_for_skill(&pool, "skill-a")
            .await
            .expect("tags");
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn bulk_ai_tagging_can_be_cancelled_before_requests_start() {
        let pool = setup_test_db().await;
        let response =
            r#"{"tags":[{"tag":"编程与 Agent 工程","confidence":0.9,"reason":"开发工具"}]}"#;
        let (api_url, _current, _max_seen) = spawn_ai_server(response, false).await;
        configure_ai(&pool, &api_url).await;
        db::upsert_skill(&pool, &make_skill("skill-a", "Skill A"))
            .await
            .expect("skill");

        let cancel_flag = Arc::new(AtomicBool::new(true));
        let events: Arc<Mutex<Vec<AiTagProgressPayload>>> = Arc::new(Mutex::new(Vec::new()));
        let events_for_emit = Arc::clone(&events);
        let results = bulk_suggest_skill_tags_impl(
            &pool,
            vec!["skill-a".to_string()],
            "job-cancel".to_string(),
            cancel_flag,
            move |payload| {
                events_for_emit.lock().expect("events").push(payload);
            },
        )
        .await
        .expect("bulk");

        assert_eq!(results.len(), 1);
        assert!(!results[0].succeeded);
        assert!(results[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("canceled"));
        assert_eq!(
            events
                .lock()
                .expect("events")
                .last()
                .map(|event| event.status),
            Some(AiTagProgressStatus::Cancelled)
        );
        let tags = db::get_skill_tags_for_skill(&pool, "skill-a")
            .await
            .expect("tags");
        assert!(tags.is_empty());
    }
}
