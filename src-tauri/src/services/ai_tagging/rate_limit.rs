use std::sync::atomic::Ordering;

use tokio::time::{sleep, Duration, Instant};

use crate::db::DbPool;

use super::types::{
    AiTagRateSettings, AiTagRunControl, DEFAULT_AI_TAGGING_CONCURRENCY_LIMIT,
    DEFAULT_AI_TAGGING_INTERVAL_MS, DEFAULT_AI_TAG_STOP_ON_RATE_LIMIT,
};

impl AiTagRunControl {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    pub(crate) fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn wait_for_rate_limit(&self) -> Result<(), String> {
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

async fn sleep_cancelable(
    duration: Duration,
    cancel_flag: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
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

pub(crate) async fn get_ai_tag_rate_settings(pool: &DbPool) -> AiTagRateSettings {
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

pub(crate) async fn get_ai_setting(pool: &DbPool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_bool_setting(value: &str, fallback: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

pub(crate) fn is_ai_rate_limit_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("429")
        || normalized.contains("too many requests")
        || normalized.contains("rate limit")
        || normalized.contains("ratelimit")
}
