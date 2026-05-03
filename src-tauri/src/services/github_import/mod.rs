//! GitHub repository import service layer.
//!
//! Owns GitHub/PAT access, repository source discovery, preview/import staging,
//! SSH preview workspace lifecycle, archive extraction, and shared helper types.
//! Tauri IPC shells live in `crate::commands::github_import`.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, State};
use tokio::time::{sleep, Duration as TokioDuration, Instant};

use crate::{
    db::{self, DbPool, Skill},
    targets::{connect_ssh_target, remote_join, ActiveTarget, RemoteTargetConfig},
    AppState,
};

include!("types.rs");
include!("preview_workspace.rs");
include!("preview.rs");
include!("remote.rs");
include!("import.rs");
include!("source.rs");
include!("archive.rs");
include!("raw_http.rs");
include!("pat.rs");

#[cfg(test)]
include!("tests.rs");

type Duration = ChronoDuration;

fn github_host_rate_limiters() -> &'static tokio::sync::Mutex<HashMap<String, Instant>> {
    GITHUB_HOST_RATE_LIMITERS.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

async fn wait_for_github_host_slot(url: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| format!("Invalid GitHub URL '{}': {}", url, e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("GitHub URL '{}' has no host.", url))?
        .to_string();
    let interval = TokioDuration::from_secs_f64(1.0 / DEFAULT_GITHUB_HOST_QPS);

    loop {
        let sleep_for = {
            let mut limiter = github_host_rate_limiters().lock().await;
            let now = Instant::now();
            let next_slot = limiter.entry(host.clone()).or_insert(now);
            if *next_slot <= now {
                *next_slot = now + interval;
                None
            } else {
                Some(*next_slot - now)
            }
        };

        match sleep_for {
            Some(duration) => sleep(duration.min(TokioDuration::from_millis(200))).await,
            None => return Ok(()),
        }
    }
}
