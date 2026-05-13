use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Duration, Instant};

use crate::db::{DbPool, SkillTag};

pub(crate) const AI_TAG_PROGRESS_EVENT: &str = "central://ai-tag-progress";
pub(crate) const DEFAULT_AI_TAGGING_CONCURRENCY_LIMIT: usize = 1;
pub(crate) const DEFAULT_AI_TAGGING_INTERVAL_MS: u64 = 4_000;
pub(crate) const DEFAULT_AI_TAG_STOP_ON_RATE_LIMIT: bool = true;
pub(crate) const AI_TAG_AUTO_APPLY_CONFIDENCE: f64 = 0.7;

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
pub(crate) struct AiTagCounters {
    pub(crate) completed: usize,
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    pub(crate) low_confidence_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct AiTagRateSettings {
    pub(crate) concurrency_limit: usize,
    pub(crate) interval_ms: u64,
    pub(crate) stop_on_rate_limit: bool,
}

#[derive(Debug)]
pub(crate) struct AiTagRateLimiter {
    pub(crate) interval: Duration,
    pub(crate) next_request_at: AsyncMutex<Instant>,
}

#[derive(Debug, Clone)]
pub(crate) struct AiTagRunControl {
    pub(crate) cancel_flag: Arc<AtomicBool>,
    pub(crate) rate_settings: AiTagRateSettings,
    pub(crate) rate_limiter: Arc<AiTagRateLimiter>,
}

#[derive(Debug, Clone)]
pub(crate) struct AiTaggingContext {
    pub(crate) pool: DbPool,
    pub(crate) api_key: String,
    pub(crate) api_url: String,
    pub(crate) model: String,
    pub(crate) tags: Arc<Vec<SkillTag>>,
    pub(crate) client: Client,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawAiTagSuggestion {
    pub(crate) tag: String,
    pub(crate) confidence: Option<f64>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawAiTagSuggestionEnvelope {
    pub(crate) tags: Vec<RawAiTagSuggestion>,
}

pub(crate) struct AiTagRunningNotifier<F>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    pub(crate) job_id: String,
    pub(crate) total: usize,
    pub(crate) counters: Arc<Mutex<AiTagCounters>>,
    pub(crate) emit_progress: Arc<F>,
}

impl<F> AiTagRunningNotifier<F>
where
    F: Fn(AiTagProgressPayload) + Send + Sync + 'static,
{
    fn counters_snapshot(&self) -> AiTagCounters {
        match self.counters.lock() {
            Ok(guard) => guard.clone(),
            Err(error) => {
                tracing::warn!(error = %error, "AI tag counter mutex poisoned during progress emit");
                AiTagCounters::default()
            }
        }
    }

    pub(crate) fn emit(&self, skill_id: &str, skill_name: &str) {
        let snapshot = self.counters_snapshot();
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
