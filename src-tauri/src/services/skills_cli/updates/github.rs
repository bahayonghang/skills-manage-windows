//! Fake and production GitHub observation for Skills CLI updates.
//!
//! Production uses the existing SecretStore-injected client. Tests inject a
//! fake that never touches the public network.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

use async_trait::async_trait;

use crate::services::github_import::{
    download_repo_snapshot, pinned_repo_ref, resolve_commit_sha, GithubImportError, GitHubRepoRef,
    GitHubRepoSnapshot,
};

use super::super::SkillsCliError;

#[derive(Debug, Clone)]
pub(crate) struct GithubObserveRequest {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub etag: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct GithubObserveResult {
    pub revision_sha: String,
    pub snapshot: GitHubRepoSnapshot,
    pub etag: Option<String>,
    pub rate_limit_remaining: Option<i64>,
    pub rate_limit_reset_at: Option<String>,
}

#[async_trait]
pub(crate) trait SkillsCliUpdateGithub: Send + Sync {
    async fn observe_repository(
        &self,
        request: GithubObserveRequest,
    ) -> Result<GithubObserveResult, SkillsCliError>;

    async fn snapshot_at_sha(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<GitHubRepoSnapshot, SkillsCliError>;
}

pub(crate) struct ProductionSkillsCliGithub {
    pub client: reqwest::Client,
    pub auth: Option<String>,
}

#[async_trait]
impl SkillsCliUpdateGithub for ProductionSkillsCliGithub {
    async fn observe_repository(
        &self,
        request: GithubObserveRequest,
    ) -> Result<GithubObserveResult, SkillsCliError> {
        let repo = GitHubRepoRef {
            owner: request.owner.clone(),
            repo: request.repo.clone(),
            branch: request.branch.clone(),
            normalized_url: format!(
                "https://github.com/{}/{}",
                request.owner, request.repo
            ),
        };
        let auth = self.auth.as_deref();
        let revision_sha = resolve_commit_sha(&self.client, &repo, auth)
            .await
            .map_err(map_github_error)?;
        let pinned = pinned_repo_ref(&repo, &revision_sha);
        let snapshot = download_repo_snapshot(&self.client, &pinned, auth)
            .await
            .map_err(map_github_error)?;
        let _ = request.etag;
        Ok(GithubObserveResult {
            revision_sha,
            snapshot,
            etag: None,
            rate_limit_remaining: None,
            rate_limit_reset_at: None,
        })
    }

    async fn snapshot_at_sha(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<GitHubRepoSnapshot, SkillsCliError> {
        let repo = GitHubRepoRef {
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: sha.to_string(),
            normalized_url: format!("https://github.com/{owner}/{repo}"),
        };
        download_repo_snapshot(&self.client, &repo, self.auth.as_deref())
            .await
            .map_err(map_github_error)
    }
}

fn map_github_error(error: GithubImportError) -> SkillsCliError {
    match error {
        GithubImportError::RateLimited(_) => SkillsCliError::UpdateRateLimited { reset_at: None },
        _ => SkillsCliError::UpdateCheckFailed,
    }
}

#[cfg(test)]
#[derive(Clone)]
enum FakeOutcome {
    Ok(GithubObserveResult),
    RateLimited { reset_at: Option<String> },
    Failed,
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct FakeSkillsCliGithub {
    results: Mutex<HashMap<String, FakeOutcome>>,
    sha_snapshots: Mutex<HashMap<String, GitHubRepoSnapshot>>,
    calls: Mutex<Vec<String>>,
}

#[cfg(test)]
impl FakeSkillsCliGithub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_result(&self, repository_key: &str, result: GithubObserveResult) {
        self.results
            .lock()
            .expect("fake github mutex")
            .insert(repository_key.to_string(), FakeOutcome::Ok(result));
    }

    pub fn set_rate_limited(&self, repository_key: &str, reset_at: Option<String>) {
        self.results.lock().expect("fake github mutex").insert(
            repository_key.to_string(),
            FakeOutcome::RateLimited { reset_at },
        );
    }

    pub fn set_failed(&self, repository_key: &str) {
        self.results.lock().expect("fake github mutex").insert(
            repository_key.to_string(),
            FakeOutcome::Failed,
        );
    }

    pub fn set_sha_snapshot(&self, sha: &str, snapshot: GitHubRepoSnapshot) {
        self.sha_snapshots
            .lock()
            .expect("fake github mutex")
            .insert(sha.to_string(), snapshot);
    }

    pub fn call_keys(&self) -> Vec<String> {
        self.calls.lock().expect("fake github mutex").clone()
    }
}

#[cfg(test)]
#[async_trait]
impl SkillsCliUpdateGithub for FakeSkillsCliGithub {
    async fn observe_repository(
        &self,
        request: GithubObserveRequest,
    ) -> Result<GithubObserveResult, SkillsCliError> {
        let key = format!("{}/{}@{}", request.owner, request.repo, request.branch);
        self.calls
            .lock()
            .expect("fake github mutex")
            .push(key.clone());
        match self
            .results
            .lock()
            .expect("fake github mutex")
            .get(&key)
            .cloned()
        {
            Some(FakeOutcome::Ok(result)) => Ok(result),
            Some(FakeOutcome::RateLimited { reset_at }) => {
                Err(SkillsCliError::UpdateRateLimited { reset_at })
            }
            Some(FakeOutcome::Failed) | None => Err(SkillsCliError::UpdateCheckFailed),
        }
    }

    async fn snapshot_at_sha(
        &self,
        _owner: &str,
        _repo: &str,
        sha: &str,
    ) -> Result<GitHubRepoSnapshot, SkillsCliError> {
        self.calls
            .lock()
            .expect("fake github mutex")
            .push(format!("sha:{sha}"));
        self.sha_snapshots
            .lock()
            .expect("fake github mutex")
            .get(sha)
            .cloned()
            .ok_or(SkillsCliError::UpdateStale)
    }
}
