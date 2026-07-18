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
use uuid::Uuid;

use crate::{
    db::{self, DbPool, Skill},
    targets::{connect_remote_target, remote_join, ActiveTarget, ConnectedRemoteTarget},
    AppState,
};

mod archive;
mod error;
mod import;
mod pat;
mod plugin_manifest;
mod preview;
mod preview_workspace;
mod progress;
mod raw_http;
mod remote;
mod source;
#[cfg(test)]
mod tests;
mod tree_manifest;
mod types;

#[cfg(test)]
use archive::*;
use import::*;
use plugin_manifest::*;
#[cfg(test)]
use preview::*;
use preview_workspace::*;
use progress::*;
use raw_http::*;
use remote::*;
use source::*;
use types::*;

pub use error::GithubImportError;

pub(crate) use archive::download_repo_snapshot;
#[cfg(test)]
pub(crate) use import::import_github_repo_skills_impl;
pub(crate) use import::{
    build_preview_skills, central_skills_root, import_github_repo_skills_from_snapshot_partially,
    import_github_repo_skills_partially_with_auth, import_github_repo_skills_with_auth,
};
pub(crate) use pat::{
    clear_github_pat_impl, get_github_pat_state_impl, github_client,
    github_direct_auth_from_secret_store, migrate_github_pat_on_startup, reveal_github_pat_impl,
    set_github_pat_impl, test_github_pat_impl,
};
#[cfg(test)]
use pat::{GITHUB_PAT_MIGRATION_SETTING_KEY, LEGACY_GITHUB_PAT_SETTING_KEY};
pub(crate) use preview::{
    preview_github_repo_import_remote_with_auth, preview_github_repo_import_with_auth,
};
pub(crate) use raw_http::fetch_raw_text;
pub(crate) use remote::import_github_repo_skills_remote_with_auth;
pub(crate) use remote::{
    discard_preview_workspace_for_active_target, fetch_github_skill_markdown_from_remote_workspace,
};
pub(crate) use source::{
    build_repo_skill_candidates_from_snapshot_at_path, fetch_repo_skill_candidates_from_source,
    inspect_github_repo_skills_with_auth, inspect_repo_skill_candidates_from_snapshot_at_path,
    repo_file_relative_to_source, resolve_repo_source,
};
pub use types::{
    DuplicateResolution, GitHubImportProgressPayload, GitHubImportProgressPhase, GitHubPatState,
    GitHubPatTestResult, GitHubRepoImportResult, GitHubRepoPreview, GitHubRepoRef,
    GitHubSkillConflict, GitHubSkillImportSelection, GitHubSkillPreview, GitHubSkillPreviewFile,
    ImportedGitHubSkillSummary,
};
pub(crate) use types::{
    GitHubRepoSnapshot, InspectedGitHubRepoSkills, InvalidRemoteSkillCandidate,
    RemoteSkillCandidate, ResolvedGitHubRepoSource,
};

pub(crate) type Duration = ChronoDuration;

fn github_host_rate_limiters() -> &'static tokio::sync::Mutex<HashMap<String, Instant>> {
    types::GITHUB_HOST_RATE_LIMITERS.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

async fn wait_for_github_host_slot(url: &str) -> Result<(), GithubImportError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| {
        GithubImportError::InvalidUrl(format!("Invalid GitHub URL '{}': {}", url, e))
    })?;
    let host = parsed
        .host_str()
        .ok_or_else(|| GithubImportError::InvalidUrl(format!("GitHub URL '{}' has no host.", url)))?
        .to_string();
    let interval = TokioDuration::from_secs_f64(1.0 / types::DEFAULT_GITHUB_HOST_QPS);

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
