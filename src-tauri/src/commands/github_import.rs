use chrono::{DateTime, Duration, Utc};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, State};

use crate::{
    db::{self, DbPool, Skill},
    targets::{connect_ssh_target, remote_join, ActiveTarget, RemoteTargetConfig},
    AppState,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepoRef {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub normalized_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateResolution {
    Overwrite,
    Skip,
    Rename,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSkillConflict {
    pub existing_skill_id: String,
    pub existing_name: String,
    pub existing_canonical_path: Option<String>,
    pub proposed_skill_id: String,
    pub proposed_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSkillPreview {
    pub source_path: String,
    pub skill_id: String,
    pub skill_name: String,
    pub description: Option<String>,
    pub root_directory: String,
    pub skill_directory_name: String,
    pub download_url: String,
    pub conflict: Option<GitHubSkillConflict>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepoPreview {
    pub repo: GitHubRepoRef,
    pub skills: Vec<GitHubSkillPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_workspace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSkillImportSelection {
    pub source_path: String,
    pub resolution: DuplicateResolution,
    pub renamed_skill_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedGitHubSkillSummary {
    pub source_path: String,
    pub original_skill_id: String,
    pub imported_skill_id: String,
    pub skill_name: String,
    pub target_directory: String,
    pub resolution: DuplicateResolution,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepoImportResult {
    pub repo: GitHubRepoRef,
    pub imported_skills: Vec<ImportedGitHubSkillSummary>,
    pub skipped_skills: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitHubImportProgressPhase {
    Preparing,
    Writing,
    Finalizing,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubImportProgressPayload {
    pub phase: GitHubImportProgressPhase,
    pub current_skill: Option<String>,
    pub current_path: Option<String>,
    pub completed_files: usize,
    pub total_files: usize,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillFrontmatter {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteSkillCandidate {
    pub(crate) source_path: String,
    pub(crate) skill_id: String,
    pub(crate) skill_name: String,
    pub(crate) description: Option<String>,
    pub(crate) root_directory: String,
    pub(crate) skill_directory_name: String,
    pub(crate) download_url: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GitHubRepoSnapshot {
    pub(crate) files: HashMap<String, Vec<u8>>,
}

const GITHUB_PAT_SETTING_KEY: &str = "github_pat";
const NO_IMPORTABLE_SKILLS_ERROR: &str = "No importable skills found in this repository. Supported layouts include root SKILL.md, common skill directories such as skills/, .agents/skills/, .claude/skills/, direct repository subpaths, and bounded recursive SKILL.md discovery.";
const REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES: i64 = 30;
const RECURSIVE_DISCOVERY_MAX_DEPTH: usize = 5;
const SKIP_DISCOVERY_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    "target",
    "outputs",
    "__pycache__",
];
const PRIORITY_SKILL_ROOTS: &[&str] = &[
    ".",
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
    ".agents/skills",
    ".augment/skills",
    ".bob/skills",
    ".claude/skills",
    ".cline/skills",
    ".codebuddy/skills",
    ".codex/skills",
    ".commandcode/skills",
    ".continue/skills",
    ".cortex/skills",
    ".crush/skills",
    ".factory/skills",
    ".github/skills",
    ".goose/skills",
    ".iflow/skills",
    ".junie/skills",
    ".kilocode/skills",
    ".kiro/skills",
    ".kode/skills",
    ".mcpjam/skills",
    ".mux/skills",
    ".neovate/skills",
    ".opencode/skills",
    ".openhands/skills",
    ".pi/skills",
    ".qoder/skills",
    ".qwen/skills",
    ".roo/skills",
    ".trae/skills",
    ".vibe/skills",
    ".windsurf/skills",
    ".zencoder/skills",
];

static GITHUB_PREVIEW_WORKSPACES: OnceLock<Mutex<HashMap<String, GitHubPreviewWorkspace>>> =
    OnceLock::new();

#[derive(Debug, Clone)]
struct GitHubPreviewWorkspace {
    id: String,
    target_id: String,
    repo: GitHubRepoRef,
    source_path: Option<String>,
    remote_workspace_dir: String,
    remote_repo_dir: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl GitHubPreviewWorkspace {
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        debug_assert!(self.expires_at >= self.created_at);
        self.expires_at <= now
    }

    fn matches_source(
        &self,
        target_id: &str,
        repo: &GitHubRepoRef,
        source_path: Option<&str>,
    ) -> bool {
        self.target_id == target_id
            && &self.repo == repo
            && self.source_path.as_deref() == source_path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedGitHubRepoSource {
    pub(crate) repo: GitHubRepoRef,
    pub(crate) source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGitHubSource {
    owner: String,
    repo: String,
    branch: Option<String>,
    source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GitHubAccessDenialKind {
    RateLimited {
        reset_at: Option<String>,
        remaining: Option<String>,
    },
    AuthenticationOrPermission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubAccessDenial {
    kind: GitHubAccessDenialKind,
    operation: &'static str,
    status: reqwest::StatusCode,
    github_message: Option<String>,
    used_auth: bool,
}

impl fmt::Display for GitHubAccessDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = self.status.as_u16();
        match &self.kind {
            GitHubAccessDenialKind::RateLimited {
                reset_at,
                remaining,
            } => {
                write!(
                    f,
                    "GitHub API access was denied while {} because the rate limit was exceeded (HTTP {}). Retry later",
                    self.operation, status
                )?;
                if let Some(reset_at) = reset_at {
                    write!(f, " after {} UTC", reset_at)?;
                }
                write!(f, " or use authenticated GitHub requests")?;
                if let Some(remaining) = remaining {
                    write!(f, " (remaining quota: {})", remaining)?;
                }
                if let Some(message) = &self.github_message {
                    write!(f, ". GitHub said: {}", message)?;
                } else {
                    write!(f, ".")?;
                }
                Ok(())
            }
            GitHubAccessDenialKind::AuthenticationOrPermission => {
                if self.used_auth {
                    write!(
                        f,
                        "GitHub denied access while {} (HTTP {}). A configured GitHub token was used, but the repository may be private, the token/permissions are insufficient, or the token owner cannot read the repo. Verify repository access and token permissions, then retry",
                        self.operation, status
                    )?;
                } else {
                    write!(
                        f,
                        "GitHub denied access while {} (HTTP {}). The repository may require authentication, your API quota may need authenticated requests, or the token/permissions are insufficient. Verify repository access, sign in with a GitHub token that can read the repo, or retry later",
                        self.operation, status
                    )?;
                }
                if let Some(message) = &self.github_message {
                    write!(f, ". GitHub said: {}", message)?;
                } else {
                    write!(f, ".")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubErrorResponse {
    message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPatTestResult {
    pub configured: bool,
    pub ok: bool,
    pub status: Option<u16>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitHubFetchSurface {
    Api,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MirrorAttemptOutcome {
    status: Option<reqwest::StatusCode>,
    error_message: String,
}

#[derive(Debug, Clone, Copy)]
struct GitHubMirrorEndpoint {
    label: &'static str,
    api_base: &'static str,
    raw_base: &'static str,
}

const GITHUB_MIRROR_ENDPOINTS: &[GitHubMirrorEndpoint] = &[
    GitHubMirrorEndpoint {
        label: "github",
        api_base: "https://api.github.com",
        raw_base: "https://raw.githubusercontent.com",
    },
    GitHubMirrorEndpoint {
        label: "ghfast",
        api_base: "https://ghfast.top/https://api.github.com",
        raw_base: "https://ghfast.top/https://raw.githubusercontent.com",
    },
    GitHubMirrorEndpoint {
        label: "ghproxy",
        api_base: "https://ghproxy.net/https://api.github.com",
        raw_base: "https://ghproxy.net/https://raw.githubusercontent.com",
    },
    GitHubMirrorEndpoint {
        label: "gitproxy",
        api_base: "https://mirror.ghproxy.com/https://api.github.com",
        raw_base: "https://mirror.ghproxy.com/https://raw.githubusercontent.com",
    },
];

fn preview_workspace_registry() -> &'static Mutex<HashMap<String, GitHubPreviewWorkspace>> {
    GITHUB_PREVIEW_WORKSPACES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_preview_workspace(workspace: GitHubPreviewWorkspace) {
    if let Ok(mut registry) = preview_workspace_registry().lock() {
        registry.insert(workspace.id.clone(), workspace);
    }
}

fn get_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    preview_workspace_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(workspace_id).cloned())
}

fn take_preview_workspace(workspace_id: &str) -> Option<GitHubPreviewWorkspace> {
    preview_workspace_registry()
        .lock()
        .ok()
        .and_then(|mut registry| registry.remove(workspace_id))
}

fn prune_expired_preview_workspaces(now: DateTime<Utc>) -> Vec<GitHubPreviewWorkspace> {
    let Ok(mut registry) = preview_workspace_registry().lock() else {
        return Vec::new();
    };
    let expired_ids = registry
        .iter()
        .filter(|(_, workspace)| workspace.is_expired(now))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    expired_ids
        .into_iter()
        .filter_map(|id| registry.remove(&id))
        .collect()
}

#[tauri::command]
pub async fn preview_github_repo_import(
    state: State<'_, AppState>,
    repo_url: String,
) -> Result<GitHubRepoPreview, String> {
    let active_target = state.active_target().await?;
    let pool = state.active_db().await?;
    let auth = github_direct_auth_from_settings(&state.db).await?;
    match active_target {
        ActiveTarget::Local => {
            preview_github_repo_import_with_auth(&pool, &repo_url, auth.as_deref()).await
        }
        ActiveTarget::Ssh(target) => {
            preview_github_repo_import_ssh_with_auth(&pool, &target, &repo_url, auth.as_deref())
                .await
        }
    }
}

#[tauri::command]
pub async fn import_github_repo_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_url: String,
    selections: Vec<GitHubSkillImportSelection>,
    preview_workspace_id: Option<String>,
) -> Result<GitHubRepoImportResult, String> {
    let active_target = state.active_target().await?;
    let pool = state.active_db().await?;
    let auth = github_direct_auth_from_settings(&state.db).await?;
    match active_target {
        ActiveTarget::Local => {
            import_github_repo_skills_with_auth(
                &pool,
                &repo_url,
                selections,
                Some(&app),
                auth.as_deref(),
            )
            .await
        }
        ActiveTarget::Ssh(target) => {
            import_github_repo_skills_ssh_with_auth(
                &pool,
                &target,
                &repo_url,
                selections,
                preview_workspace_id.as_deref(),
                Some(&app),
                auth.as_deref(),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn fetch_github_skill_markdown(
    state: State<'_, AppState>,
    download_url: String,
    source_path: Option<String>,
    preview_workspace_id: Option<String>,
) -> Result<String, String> {
    if let Some(workspace_id) = preview_workspace_id.as_deref() {
        return fetch_github_skill_markdown_from_remote_workspace(
            &state,
            workspace_id,
            source_path.as_deref(),
        )
        .await;
    }

    let client = github_client()?;
    let auth = github_direct_auth_from_settings(&state.db).await?;
    fetch_raw_text(&client, &download_url, auth.as_deref()).await
}

#[tauri::command]
pub async fn discard_github_repo_preview_workspace(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<(), String> {
    discard_preview_workspace_for_active_target(&state, &workspace_id).await;
    Ok(())
}

#[tauri::command]
pub async fn get_github_pat(state: State<'_, AppState>) -> Result<Option<String>, String> {
    github_direct_auth_from_settings(&state.db).await
}

#[tauri::command]
pub async fn set_github_pat(state: State<'_, AppState>, value: String) -> Result<(), String> {
    db::set_setting(&state.db, GITHUB_PAT_SETTING_KEY, value.trim()).await
}

#[tauri::command]
pub async fn clear_github_pat(state: State<'_, AppState>) -> Result<(), String> {
    db::set_setting(&state.db, GITHUB_PAT_SETTING_KEY, "").await
}

#[tauri::command]
pub async fn test_github_pat(state: State<'_, AppState>) -> Result<GitHubPatTestResult, String> {
    let Some(token) = github_direct_auth_from_settings(&state.db).await? else {
        return Ok(GitHubPatTestResult {
            configured: false,
            ok: false,
            status: None,
            message: "No GitHub token is configured.".to_string(),
        });
    };

    let client = github_client()?;
    let response = client
        .get("https://api.github.com/rate_limit")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Failed to test GitHub token: {}", e))?;
    let status = response.status();
    if status.is_success() {
        return Ok(GitHubPatTestResult {
            configured: true,
            ok: true,
            status: Some(status.as_u16()),
            message: "GitHub token is usable for authenticated GitHub requests.".to_string(),
        });
    }

    let denial = parse_github_denial_response(response, "testing GitHub token", true)
        .await
        .map(|denial| denial.to_string())
        .unwrap_or_else(|| format!("GitHub token test returned HTTP {}", status.as_u16()));

    Ok(GitHubPatTestResult {
        configured: true,
        ok: false,
        status: Some(status.as_u16()),
        message: denial,
    })
}

pub(crate) async fn preview_github_repo_import_impl(
    pool: &DbPool,
    repo_url: &str,
) -> Result<GitHubRepoPreview, String> {
    let auth = github_direct_auth_from_settings(pool).await?;
    preview_github_repo_import_with_auth(pool, repo_url, auth.as_deref()).await
}

pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, String> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let candidates = fetch_repo_skill_candidates_from_source(
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth,
    )
    .await?;
    let skills = build_preview_skills(pool, &candidates).await?;

    if skills.is_empty() {
        return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
    }

    Ok(GitHubRepoPreview {
        repo: resolved.repo,
        skills,
        preview_workspace_id: None,
    })
}

async fn preview_github_repo_import_ssh_with_auth(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, String> {
    let resolved = resolve_repo_source(repo_url, auth).await?;
    let connection = connect_ssh_target(target).await?;
    cleanup_expired_preview_workspaces_for_connection(&connection, target).await;

    let workspace = create_remote_preview_workspace(&connection, target, &resolved, auth).await?;
    let preview_result = async {
        let candidates = build_remote_repo_skill_candidates_from_workspace(
            &connection,
            &resolved.repo,
            &workspace.remote_repo_dir,
            resolved.source_path.as_deref(),
        )
        .await?;
        let skills = build_preview_skills(pool, &candidates).await?;
        if skills.is_empty() {
            return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
        }
        Ok(skills)
    }
    .await;

    match preview_result {
        Ok(skills) => {
            register_preview_workspace(workspace.clone());
            Ok(GitHubRepoPreview {
                repo: resolved.repo,
                skills,
                preview_workspace_id: Some(workspace.id),
            })
        }
        Err(error) => {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
            Err(error)
        }
    }
}

async fn cleanup_expired_preview_workspaces_for_connection(
    connection: &crate::targets::ConnectedSshTarget,
    target: &RemoteTargetConfig,
) {
    for workspace in prune_expired_preview_workspaces(Utc::now()) {
        if workspace.target_id == target.id {
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
        }
    }
}

async fn create_remote_preview_workspace(
    connection: &crate::targets::ConnectedSshTarget,
    target: &RemoteTargetConfig,
    resolved: &ResolvedGitHubRepoSource,
    auth: Option<&str>,
) -> Result<GitHubPreviewWorkspace, String> {
    let archive_url = github_archive_url(&resolved.repo);
    let script = remote_workspace_download_script(auth)?;
    let output = connection
        .run_script(
            &script,
            &[archive_url.as_str(), crate::commands::APP_USER_AGENT],
        )
        .await?;
    let remote_workspace_dir = output
        .lines()
        .last()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| "Remote GitHub preview did not return a workspace path.".to_string())?
        .to_string();
    let remote_repo_dir = remote_join(&remote_workspace_dir, "repo");
    let now = Utc::now();

    Ok(GitHubPreviewWorkspace {
        id: format!("github-preview-{}", uuid::Uuid::new_v4()),
        target_id: target.id.clone(),
        repo: resolved.repo.clone(),
        source_path: resolved.source_path.clone(),
        remote_workspace_dir,
        remote_repo_dir,
        created_at: now,
        expires_at: now + Duration::minutes(REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES),
    })
}

fn github_archive_url(repo: &GitHubRepoRef) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/tarball/{}",
        repo.owner, repo.repo, repo.branch
    )
}

fn remote_workspace_download_script(auth: Option<&str>) -> Result<String, String> {
    let auth_block = match auth.filter(|token| !token.trim().is_empty()) {
        Some(token) => {
            let header = curl_auth_header_config_line(token)?;
            format!(
                r#"curl_conf="$workspace/curl.conf"
umask 077
cat > "$curl_conf" <<'SKILLPORT_CURL_CONF'
{header}
SKILLPORT_CURL_CONF
"#
            )
        }
        None => String::new(),
    };

    Ok(format!(
        r#"set -eu
archive_url=$1
user_agent=$2
workspace=""
curl_conf=""
cleanup() {{
  status=$?
  if [ -n "$curl_conf" ]; then
    rm -f -- "$curl_conf" || true
  fi
  if [ "$status" -ne 0 ] && [ -n "$workspace" ]; then
    rm -rf -- "$workspace" || true
  fi
  exit "$status"
}}
trap cleanup EXIT
for tool in sh curl tar find mktemp; do
  command -v "$tool" >/dev/null 2>&1 || {{
    printf 'Missing required remote tool: %s\n' "$tool" >&2
    exit 127
  }}
done
workspace=$(mktemp -d "${{TMPDIR:-/tmp}}/skillport-github-preview.XXXXXX")
repo_dir="$workspace/repo"
archive_file="$workspace/repo.tar.gz"
mkdir -p -- "$repo_dir"
{auth_block}
if [ -n "$curl_conf" ]; then
  curl -fL --retry 2 --connect-timeout 30 -A "$user_agent" -K "$curl_conf" -o "$archive_file" "$archive_url"
else
  curl -fL --retry 2 --connect-timeout 30 -A "$user_agent" -o "$archive_file" "$archive_url"
fi
tar -xzf "$archive_file" -C "$repo_dir" --strip-components=1
rm -f -- "$archive_file"
printf '%s\n' "$workspace"
"#
    ))
}

fn curl_auth_header_config_line(token: &str) -> Result<String, String> {
    if token.contains('\n') || token.contains('\r') {
        return Err("GitHub token contains unsupported newline characters.".to_string());
    }
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("header = \"Authorization: Bearer {escaped}\""))
}

pub(crate) async fn import_github_repo_skills_impl(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
) -> Result<GitHubRepoImportResult, String> {
    let auth = github_direct_auth_from_settings(pool).await?;
    import_github_repo_skills_with_auth(pool, repo_url, selections, app, auth.as_deref()).await
}

pub(crate) async fn import_github_repo_skills_with_auth(
    pool: &DbPool,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<GitHubRepoImportResult, String> {
    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Preparing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files: 0,
            completed_bytes: 0,
            total_bytes: 0,
        },
    );

    let resolved = resolve_repo_source(repo_url, auth).await?;
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, &resolved.repo, auth).await?;
    let candidates = build_repo_skill_candidates_from_snapshot_at_path(
        &resolved.repo,
        &snapshot,
        resolved.source_path.as_deref(),
    )?;
    if candidates.is_empty() {
        return Err(NO_IMPORTABLE_SKILLS_ERROR.to_string());
    }

    let central_root = central_skills_root(pool).await?;
    std::fs::create_dir_all(&central_root)
        .map_err(|e| format!("Failed to create central skills directory: {}", e))?;

    let (mut staging_ops, skipped_skills) =
        plan_import_staging(pool, &candidates, selections).await?;

    for op in &mut staging_ops {
        op.source_files = collect_snapshot_source_files(&snapshot, &op.candidate.source_path)?;
    }

    let total_files = staging_ops
        .iter()
        .map(|op| op.source_files.len())
        .sum::<usize>();
    let total_bytes = staging_ops
        .iter()
        .flat_map(|op| op.source_files.iter())
        .map(|file| file.byte_len as u64)
        .sum::<u64>();
    let mut progress_state = GitHubImportProgressState {
        completed_files: 0,
        total_files,
        completed_bytes: 0,
        total_bytes,
    };

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Writing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files,
            completed_bytes: 0,
            total_bytes,
        },
    );

    let mut imported_skills = Vec::new();
    let mut created_paths = Vec::new();

    for op in &staging_ops {
        let target_dir = central_root.join(&op.final_skill_id);
        if target_dir.exists() {
            if op.resolution == DuplicateResolution::Overwrite {
                std::fs::remove_dir_all(&target_dir).map_err(|e| {
                    format!(
                        "Failed to replace existing canonical skill '{}': {}",
                        op.final_skill_id, e
                    )
                })?;
            } else {
                cleanup_created_directories(&created_paths);
                return Err(format!(
                    "Target directory '{}' already exists.",
                    target_dir.display()
                ));
            }
        }

        if let Err(error) = write_snapshot_source_to_target(
            &snapshot,
            &op.source_files,
            &target_dir,
            &op.candidate.source_path,
            &mut progress_state,
            app,
        ) {
            cleanup_created_directories(&created_paths);
            if target_dir.exists() {
                let _ = std::fs::remove_dir_all(&target_dir);
            }
            return Err(error);
        }

        created_paths.push(target_dir.clone());

        let skill_md_path = target_dir.join("SKILL.md");
        let raw = std::fs::read_to_string(&skill_md_path)
            .map_err(|e| format!("Failed to read imported SKILL.md: {}", e))?;
        let frontmatter = parse_frontmatter(&raw).ok_or_else(|| {
            format!(
                "Imported skill '{}' is missing valid frontmatter.",
                op.candidate.source_path
            )
        })?;

        let db_skill = Skill {
            id: op.final_skill_id.clone(),
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            file_path: skill_md_path.to_string_lossy().into_owned(),
            canonical_path: Some(target_dir.to_string_lossy().into_owned()),
            is_central: true,
            source: Some(format!(
                "github:{}/{}",
                resolved.repo.owner, resolved.repo.repo
            )),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        };
        db::upsert_skill(pool, &db_skill).await?;
        db::assign_github_repository_to_skill(
            pool,
            &resolved.repo.owner,
            &resolved.repo.repo,
            &resolved.repo.branch,
            &resolved.repo.normalized_url,
            &op.final_skill_id,
            &op.candidate.source_path,
        )
        .await?;

        imported_skills.push(ImportedGitHubSkillSummary {
            source_path: op.candidate.source_path.clone(),
            original_skill_id: op.candidate.skill_id.clone(),
            imported_skill_id: op.final_skill_id.clone(),
            skill_name: frontmatter.name,
            target_directory: target_dir.to_string_lossy().into_owned(),
            resolution: op.resolution.clone(),
        });
    }

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Finalizing,
            current_skill: None,
            current_path: None,
            completed_files: progress_state.completed_files,
            total_files: progress_state.total_files,
            completed_bytes: progress_state.completed_bytes,
            total_bytes: progress_state.total_bytes,
        },
    );

    Ok(GitHubRepoImportResult {
        repo: resolved.repo,
        imported_skills,
        skipped_skills,
    })
}

async fn import_github_repo_skills_ssh_with_auth(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    repo_url: &str,
    selections: Vec<GitHubSkillImportSelection>,
    preview_workspace_id: Option<&str>,
    app: Option<&AppHandle>,
    auth: Option<&str>,
) -> Result<GitHubRepoImportResult, String> {
    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Preparing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files: 0,
            completed_bytes: 0,
            total_bytes: 0,
        },
    );

    if selections.is_empty() {
        return Err("Select at least one skill to import.".to_string());
    }

    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    let central_root = central.global_skills_dir;
    let connection = connect_ssh_target(target).await?;
    connection.mkdir_p(&central_root).await?;

    let resolved = resolve_repo_source(repo_url, auth).await?;
    let workspace =
        resolve_ssh_import_workspace(&connection, target, &resolved, preview_workspace_id, auth)
            .await?;
    let candidates = build_remote_repo_skill_candidates_from_workspace(
        &connection,
        &resolved.repo,
        &workspace.remote_repo_dir,
        resolved.source_path.as_deref(),
    )
    .await?;

    let (staging_ops, skipped_skills) = plan_import_staging(pool, &candidates, selections).await?;

    let total_files = staging_ops.len();
    let total_bytes = 0;
    let mut progress_state = GitHubImportProgressState {
        completed_files: 0,
        total_files,
        completed_bytes: 0,
        total_bytes,
    };

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Writing,
            current_skill: None,
            current_path: None,
            completed_files: 0,
            total_files,
            completed_bytes: 0,
            total_bytes,
        },
    );

    let mut imported_skills = Vec::new();
    let mut created_paths: Vec<String> = Vec::new();
    let mut created_stages: Vec<String> = Vec::new();

    for op in &staging_ops {
        let target_dir = remote_join(&central_root, &op.final_skill_id);
        let source_dir =
            remote_skill_source_dir(&workspace.remote_repo_dir, &op.candidate.source_path)?;
        let stage_dir = remote_join(
            &central_root,
            &format!(
                ".skillport-import-{}-{}",
                op.final_skill_id,
                uuid::Uuid::new_v4()
            ),
        );
        if connection.exists(&target_dir).await? && op.resolution != DuplicateResolution::Overwrite
        {
            for path in created_paths.iter().rev() {
                let _ = connection.remove_tree(path).await;
            }
            return Err(format!("Target directory '{}' already exists.", target_dir));
        }

        created_stages.push(stage_dir.clone());
        let overwrite = if op.resolution == DuplicateResolution::Overwrite {
            "1"
        } else {
            "0"
        };
        if let Err(error) = connection
            .run_script(
                remote_import_skill_script(),
                &[
                    source_dir.as_str(),
                    stage_dir.as_str(),
                    target_dir.as_str(),
                    overwrite,
                ],
            )
            .await
        {
            let _ = connection.remove_tree(&stage_dir).await;
            for path in created_paths.iter().rev() {
                let _ = connection.remove_tree(path).await;
            }
            return Err(error);
        }
        created_stages.pop();

        created_paths.push(target_dir.clone());

        let skill_md_path = remote_join(&target_dir, "SKILL.md");
        let frontmatter = SkillFrontmatter {
            name: op.candidate.skill_name.clone(),
            description: op.candidate.description.clone(),
        };

        progress_state.completed_files += 1;
        emit_github_import_progress(
            app,
            GitHubImportProgressPayload {
                phase: GitHubImportProgressPhase::Writing,
                current_skill: Some(op.candidate.source_path.clone()),
                current_path: Some("SKILL.md".to_string()),
                completed_files: progress_state.completed_files,
                total_files: progress_state.total_files,
                completed_bytes: progress_state.completed_bytes,
                total_bytes: progress_state.total_bytes,
            },
        );

        let db_skill = Skill {
            id: op.final_skill_id.clone(),
            name: frontmatter.name.clone(),
            description: frontmatter.description.clone(),
            file_path: skill_md_path,
            canonical_path: Some(target_dir.clone()),
            is_central: true,
            source: Some(format!(
                "github:{}/{}",
                resolved.repo.owner, resolved.repo.repo
            )),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
        };
        db::upsert_skill(pool, &db_skill).await?;
        db::assign_github_repository_to_skill(
            pool,
            &resolved.repo.owner,
            &resolved.repo.repo,
            &resolved.repo.branch,
            &resolved.repo.normalized_url,
            &op.final_skill_id,
            &op.candidate.source_path,
        )
        .await?;

        imported_skills.push(ImportedGitHubSkillSummary {
            source_path: op.candidate.source_path.clone(),
            original_skill_id: op.candidate.skill_id.clone(),
            imported_skill_id: op.final_skill_id.clone(),
            skill_name: frontmatter.name,
            target_directory: target_dir,
            resolution: op.resolution.clone(),
        });
    }

    for stage in created_stages.iter().rev() {
        let _ = connection.remove_tree(stage).await;
    }

    emit_github_import_progress(
        app,
        GitHubImportProgressPayload {
            phase: GitHubImportProgressPhase::Finalizing,
            current_skill: None,
            current_path: None,
            completed_files: progress_state.completed_files,
            total_files: progress_state.total_files,
            completed_bytes: progress_state.completed_bytes,
            total_bytes: progress_state.total_bytes,
        },
    );

    let _ = take_preview_workspace(&workspace.id);
    let _ = connection
        .remove_tree(&workspace.remote_workspace_dir)
        .await;

    Ok(GitHubRepoImportResult {
        repo: resolved.repo,
        imported_skills,
        skipped_skills,
    })
}

async fn resolve_ssh_import_workspace(
    connection: &crate::targets::ConnectedSshTarget,
    target: &RemoteTargetConfig,
    resolved: &ResolvedGitHubRepoSource,
    preview_workspace_id: Option<&str>,
    auth: Option<&str>,
) -> Result<GitHubPreviewWorkspace, String> {
    cleanup_expired_preview_workspaces_for_connection(connection, target).await;

    if let Some(workspace_id) = preview_workspace_id {
        if let Some(workspace) = get_preview_workspace(workspace_id) {
            if !workspace.matches_source(
                &target.id,
                &resolved.repo,
                resolved.source_path.as_deref(),
            ) {
                return Err(
                    "GitHub preview workspace does not match the active target or repository. Preview the repository again."
                        .to_string(),
                );
            }
            if !workspace.is_expired(Utc::now()) {
                return Ok(workspace);
            }
            let _ = take_preview_workspace(workspace_id);
            let _ = connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await;
        }
    }

    let workspace = create_remote_preview_workspace(connection, target, resolved, auth).await?;
    register_preview_workspace(workspace.clone());
    Ok(workspace)
}

fn remote_skill_source_dir(remote_repo_dir: &str, source_path: &str) -> Result<String, String> {
    if source_path == "." {
        return Ok(remote_repo_dir.to_string());
    }
    Ok(remote_join(
        remote_repo_dir,
        &normalize_repo_path(source_path)?,
    ))
}

fn remote_import_skill_script() -> &'static str {
    r#"set -eu
source_dir=$1
stage_dir=$2
target_dir=$3
overwrite=$4
backup_dir="${target_dir}.skillport-backup-$$"
rm -rf -- "$stage_dir"
mkdir -p -- "$stage_dir"
cp -a "$source_dir/." "$stage_dir/"
if [ -e "$target_dir" ]; then
  if [ "$overwrite" != "1" ]; then
    rm -rf -- "$stage_dir"
    printf 'Target directory already exists: %s\n' "$target_dir" >&2
    exit 23
  fi
  rm -rf -- "$backup_dir"
  mv "$target_dir" "$backup_dir"
fi
if mv "$stage_dir" "$target_dir"; then
  rm -rf -- "$backup_dir"
else
  status=$?
  if [ -e "$backup_dir" ] && [ ! -e "$target_dir" ]; then
    mv "$backup_dir" "$target_dir" || true
  fi
  rm -rf -- "$stage_dir"
  exit "$status"
fi
"#
}

#[derive(Debug, Clone)]
struct StagedImport {
    candidate: RemoteSkillCandidate,
    final_skill_id: String,
    resolution: DuplicateResolution,
    source_files: Vec<SnapshotSourceFile>,
}

async fn plan_import_staging(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
    selections: Vec<GitHubSkillImportSelection>,
) -> Result<(Vec<StagedImport>, Vec<String>), String> {
    if selections.is_empty() {
        return Err("Select at least one skill to import.".to_string());
    }

    let mut selected_paths = HashSet::new();
    let mut occupied_ids = current_central_skill_ids(pool).await?;
    let mut staging_ops = Vec::new();
    let mut skipped_skills = Vec::new();

    for selection in selections {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source_path == selection.source_path)
            .ok_or_else(|| {
                format!(
                    "Selected skill '{}' is no longer available in the preview.",
                    selection.source_path
                )
            })?;

        if !selected_paths.insert(candidate.source_path.clone()) {
            return Err(format!(
                "Skill '{}' was selected more than once.",
                candidate.source_path
            ));
        }

        match selection.resolution {
            DuplicateResolution::Skip => {
                skipped_skills.push(candidate.source_path.clone());
            }
            DuplicateResolution::Overwrite => {
                occupied_ids.insert(candidate.skill_id.clone());
                staging_ops.push(StagedImport {
                    candidate: candidate.clone(),
                    final_skill_id: candidate.skill_id.clone(),
                    resolution: DuplicateResolution::Overwrite,
                    source_files: Vec::new(),
                });
            }
            DuplicateResolution::Rename => {
                let requested_id =
                    sanitize_skill_id(selection.renamed_skill_id.as_deref().ok_or_else(|| {
                        format!(
                            "Skill '{}' requires a renamed skill id for rename resolution.",
                            candidate.source_path
                        )
                    })?)?;
                if occupied_ids.contains(&requested_id) {
                    return Err(format!(
                        "Renamed skill id '{}' is already in use.",
                        requested_id
                    ));
                }
                occupied_ids.insert(requested_id.clone());
                staging_ops.push(StagedImport {
                    candidate: candidate.clone(),
                    final_skill_id: requested_id,
                    resolution: DuplicateResolution::Rename,
                    source_files: Vec::new(),
                });
            }
        }
    }

    if staging_ops.is_empty() && skipped_skills.is_empty() {
        return Err("No valid import operations were requested.".to_string());
    }

    Ok((staging_ops, skipped_skills))
}

fn cleanup_created_directories(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = std::fs::remove_dir_all(path);
    }
}

async fn central_skills_root(pool: &DbPool) -> Result<PathBuf, String> {
    let central = db::get_agent_by_id(pool, "central")
        .await?
        .ok_or_else(|| "Central agent not found in database".to_string())?;
    Ok(PathBuf::from(central.global_skills_dir))
}

async fn current_central_skill_ids(pool: &DbPool) -> Result<HashSet<String>, String> {
    let rows = sqlx::query("SELECT id FROM skills WHERE is_central = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<HashSet<_>>())
}

async fn build_preview_skills(
    pool: &DbPool,
    candidates: &[RemoteSkillCandidate],
) -> Result<Vec<GitHubSkillPreview>, String> {
    let mut skills = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let existing = db::get_skill_by_id(pool, &candidate.skill_id).await?;
        let conflict = existing.and_then(|existing| {
            if existing.is_central {
                Some(GitHubSkillConflict {
                    existing_skill_id: existing.id,
                    existing_name: existing.name,
                    existing_canonical_path: existing.canonical_path,
                    proposed_skill_id: candidate.skill_id.clone(),
                    proposed_name: candidate.skill_name.clone(),
                })
            } else {
                None
            }
        });

        skills.push(GitHubSkillPreview {
            source_path: candidate.source_path.clone(),
            skill_id: candidate.skill_id.clone(),
            skill_name: candidate.skill_name.clone(),
            description: candidate.description.clone(),
            root_directory: candidate.root_directory.clone(),
            skill_directory_name: candidate.skill_directory_name.clone(),
            download_url: candidate.download_url.clone(),
            conflict,
        });
    }
    Ok(skills)
}

pub(crate) async fn resolve_repo_source(
    repo_url: &str,
    auth_token: Option<&str>,
) -> Result<ResolvedGitHubRepoSource, String> {
    let parsed = parse_github_source(repo_url)?;
    let owner = parsed.owner;
    let repo = parsed.repo;
    let client = github_client()?;
    let response = send_github_request_with_fallback(
        &client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!("/repos/{owner}/{repo}"),
            )
        },
        "Failed to inspect GitHub repository",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("GitHub repository not found.".to_string());
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(
            classify_github_denial_response(response, "inspecting the repository")
                .await
                .unwrap_or_else(|| format!("Failed to inspect GitHub repository: HTTP {}", status)),
        );
    }

    let payload: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let branch = payload
        .get("default_branch")
        .and_then(|v| v.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("main")
        .to_string();
    let branch = parsed.branch.unwrap_or(branch);

    Ok(ResolvedGitHubRepoSource {
        repo: GitHubRepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            branch,
            normalized_url: format!("https://github.com/{owner}/{repo}"),
        },
        source_path: parsed.source_path,
    })
}

pub(crate) async fn github_direct_auth_from_settings(
    pool: &DbPool,
) -> Result<Option<String>, String> {
    Ok(db::get_setting(pool, GITHUB_PAT_SETTING_KEY)
        .await?
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty()))
}

pub(crate) fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(crate::commands::APP_USER_AGENT)
        .build()
        .map_err(|e| e.to_string())
}

fn parse_github_source(url: &str) -> Result<ParsedGitHubSource, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("Invalid GitHub repository URL.".to_string());
    }
    if has_raw_path_traversal(trimmed) {
        return Err("Repository subpath traversal is not supported.".to_string());
    }

    let parse_target =
        if trimmed.starts_with("github.com/") || trimmed.starts_with("www.github.com/") {
            format!("https://{trimmed}")
        } else if is_github_shorthand_source(trimmed) {
            format!("https://github.com/{trimmed}")
        } else {
            trimmed.to_string()
        };

    let parsed = reqwest::Url::parse(&parse_target)
        .map_err(|_| "Invalid GitHub repository URL.".to_string())?;

    if parsed.scheme() != "https" {
        return Err("Only https:// GitHub repository URLs are supported.".to_string());
    }
    let host = parsed.host_str().unwrap_or_default();
    if host != "github.com" && host != "www.github.com" {
        return Err("Only github.com repository URLs are supported.".to_string());
    }

    let segments = parsed
        .path_segments()
        .ok_or_else(|| "Invalid GitHub repository URL.".to_string())?
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let owner = segments
        .first()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| "GitHub repository URL must include an owner.".to_string())?;
    let repo = segments
        .get(1)
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| "GitHub repository URL must include a repository name.".to_string())?;

    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if owner.is_empty() || repo.is_empty() {
        return Err("GitHub repository URL is missing owner or repository.".to_string());
    }

    let (branch, source_segments) = match segments.get(2).copied() {
        Some("tree") => {
            let branch = segments
                .get(3)
                .filter(|segment| !segment.is_empty())
                .ok_or_else(|| "GitHub tree URL must include a branch.".to_string())?;
            (Some((*branch).to_string()), &segments[4..])
        }
        Some("blob") => {
            return Err("GitHub blob URLs are not supported for repository import.".to_string());
        }
        Some(_) => (None, &segments[2..]),
        None => (None, &segments[2..]),
    };
    let source_path = normalize_repo_subpath(source_segments)?;

    Ok(ParsedGitHubSource {
        owner: owner.to_lowercase(),
        repo: repo.to_lowercase(),
        branch,
        source_path,
    })
}

fn is_github_shorthand_source(value: &str) -> bool {
    let mut segments = value.split('/').filter(|segment| !segment.is_empty());
    let Some(owner) = segments.next() else {
        return false;
    };
    let Some(repo) = segments.next() else {
        return false;
    };

    !owner.contains(':')
        && !owner.contains('\\')
        && !repo.contains(':')
        && !repo.contains('\\')
        && !owner.starts_with('.')
        && !repo.starts_with('.')
}

fn has_raw_path_traversal(value: &str) -> bool {
    let path_only = value
        .split(['?', '#'])
        .next()
        .unwrap_or(value)
        .replace('\\', "/")
        .to_ascii_lowercase();
    path_only
        .split('/')
        .any(|segment| segment == ".." || segment == "%2e%2e")
}

fn normalize_repo_subpath(segments: &[&str]) -> Result<Option<String>, String> {
    if segments.is_empty() {
        return Ok(None);
    }

    let path = segments.join("/");
    if !is_safe_repo_relative_path(&path) {
        return Err(format!("Repository subpath '{}' is not supported.", path));
    }

    Ok(Some(path))
}

pub(crate) async fn fetch_repo_skill_candidates_from_source(
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth_token: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, String> {
    let client = github_client()?;
    let snapshot = download_repo_snapshot(&client, repo, auth_token).await?;
    build_repo_skill_candidates_from_snapshot_at_path(repo, &snapshot, source_path)
}

#[cfg(test)]
fn build_repo_skill_candidates_from_snapshot(
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
) -> Result<Vec<RemoteSkillCandidate>, String> {
    build_repo_skill_candidates_from_snapshot_at_path(repo, snapshot, None)
}

pub(crate) fn build_repo_skill_candidates_from_snapshot_at_path(
    repo: &GitHubRepoRef,
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, String> {
    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let manifests = discover_skill_manifests(snapshot, source_path)?;

    let mut candidates = Vec::with_capacity(manifests.len());
    let mut seen_names = HashSet::new();
    for manifest in manifests {
        let raw = snapshot
            .files
            .get(&manifest.skill_md_path)
            .ok_or_else(|| format!("Missing snapshot file '{}'.", manifest.skill_md_path))?;
        let content = String::from_utf8(raw.clone())
            .map_err(|_| format!("Skill '{}' is not valid UTF-8.", manifest.source_path))?;
        let frontmatter = parse_frontmatter(&content).ok_or_else(|| {
            if manifest.source_path == "." {
                "Repository root SKILL.md is missing valid frontmatter.".to_string()
            } else {
                format!(
                    "Skill '{}' is missing valid frontmatter.",
                    manifest.source_path
                )
            }
        })?;
        if !seen_names.insert(frontmatter.name.clone()) {
            continue;
        }

        let skill_id = if manifest.source_path == "." {
            let repo_skill_id = sanitize_skill_id(&repo.repo)?;
            repo_skill_id
                .strip_suffix("-skill")
                .unwrap_or(&repo_skill_id)
                .to_string()
        } else {
            sanitize_skill_id(&manifest.skill_directory_name)?
        };

        candidates.push(RemoteSkillCandidate {
            source_path: manifest.source_path.clone(),
            skill_id,
            skill_name: frontmatter.name,
            description: frontmatter.description,
            root_directory: manifest.root_directory,
            skill_directory_name: if manifest.source_path == "." {
                repo.repo.clone()
            } else {
                manifest.skill_directory_name
            },
            download_url: raw_file_url(direct_endpoint, repo, &manifest.skill_md_path),
        });
    }

    Ok(candidates)
}

async fn build_remote_repo_skill_candidates_from_workspace(
    connection: &crate::targets::ConnectedSshTarget,
    repo: &GitHubRepoRef,
    remote_repo_dir: &str,
    source_path: Option<&str>,
) -> Result<Vec<RemoteSkillCandidate>, String> {
    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let manifest_paths = remote_skill_manifest_paths(connection, remote_repo_dir).await?;
    let manifests = discover_skill_manifests_from_paths(
        manifest_paths.iter().map(String::as_str),
        source_path,
    )?;

    let mut candidates = Vec::with_capacity(manifests.len());
    let mut seen_names = HashSet::new();
    for manifest in manifests {
        let skill_md_remote_path = remote_join(remote_repo_dir, &manifest.skill_md_path);
        let raw = connection.read_file(&skill_md_remote_path).await?;
        let content = String::from_utf8(raw)
            .map_err(|_| format!("Skill '{}' is not valid UTF-8.", manifest.source_path))?;
        let frontmatter = parse_frontmatter(&content).ok_or_else(|| {
            if manifest.source_path == "." {
                "Repository root SKILL.md is missing valid frontmatter.".to_string()
            } else {
                format!(
                    "Skill '{}' is missing valid frontmatter.",
                    manifest.source_path
                )
            }
        })?;
        if !seen_names.insert(frontmatter.name.clone()) {
            continue;
        }

        let skill_id = if manifest.source_path == "." {
            let repo_skill_id = sanitize_skill_id(&repo.repo)?;
            repo_skill_id
                .strip_suffix("-skill")
                .unwrap_or(&repo_skill_id)
                .to_string()
        } else {
            sanitize_skill_id(&manifest.skill_directory_name)?
        };

        candidates.push(RemoteSkillCandidate {
            source_path: manifest.source_path.clone(),
            skill_id,
            skill_name: frontmatter.name,
            description: frontmatter.description,
            root_directory: manifest.root_directory,
            skill_directory_name: if manifest.source_path == "." {
                repo.repo.clone()
            } else {
                manifest.skill_directory_name
            },
            download_url: raw_file_url(direct_endpoint, repo, &manifest.skill_md_path),
        });
    }

    Ok(candidates)
}

async fn remote_skill_manifest_paths(
    connection: &crate::targets::ConnectedSshTarget,
    remote_repo_dir: &str,
) -> Result<Vec<String>, String> {
    let output = connection
        .run_script(
            r#"set -eu
repo_dir=$1
cd "$repo_dir"
find . -type f -iname 'SKILL.md' -print | sed 's#^\./##'
"#,
            &[remote_repo_dir],
        )
        .await?;

    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize_repo_path)
        .filter_map(|result| match result {
            Ok(path) if is_skill_md_repo_path(&path) => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct SnapshotSkillManifest {
    source_path: String,
    root_directory: String,
    skill_directory_name: String,
    skill_md_path: String,
}

fn discover_skill_manifests(
    snapshot: &GitHubRepoSnapshot,
    source_path: Option<&str>,
) -> Result<Vec<SnapshotSkillManifest>, String> {
    discover_skill_manifests_from_paths(snapshot.files.keys().map(String::as_str), source_path)
}

fn discover_skill_manifests_from_paths<'a, I>(
    paths: I,
    source_path: Option<&str>,
) -> Result<Vec<SnapshotSkillManifest>, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let path_set = paths
        .into_iter()
        .map(normalize_repo_path)
        .collect::<Result<HashSet<_>, _>>()?;
    let base_path = source_path
        .map(normalize_repo_path)
        .transpose()?
        .unwrap_or_default();
    let mut manifests = Vec::new();
    let mut seen_source_paths = HashSet::new();

    if let Some(manifest) = direct_skill_manifest(&path_set, &base_path) {
        insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
    }

    for root in PRIORITY_SKILL_ROOTS {
        let search_root = join_repo_path(&base_path, root)?;
        for manifest in immediate_skill_manifests(&path_set, &search_root)? {
            insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
        }
    }

    if manifests.is_empty() {
        for manifest in recursive_skill_manifests(&path_set, &base_path)? {
            insert_manifest(&mut manifests, &mut seen_source_paths, manifest);
        }
    }

    Ok(manifests)
}

fn direct_skill_manifest(
    paths: &HashSet<String>,
    base_path: &str,
) -> Option<SnapshotSkillManifest> {
    let skill_md_path = join_repo_path(base_path, "SKILL.md").ok()?;
    paths
        .contains(&skill_md_path)
        .then(|| manifest_from_skill_md_path(&skill_md_path))
        .flatten()
}

fn immediate_skill_manifests(
    paths: &HashSet<String>,
    search_root: &str,
) -> Result<Vec<SnapshotSkillManifest>, String> {
    let mut manifests = paths
        .iter()
        .filter(|path| is_immediate_skill_manifest(path, search_root))
        .filter_map(|path| manifest_from_skill_md_path(path))
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(manifests)
}

fn recursive_skill_manifests(
    paths: &HashSet<String>,
    base_path: &str,
) -> Result<Vec<SnapshotSkillManifest>, String> {
    let mut manifests = paths
        .iter()
        .filter(|path| is_recursive_skill_manifest(path, base_path))
        .filter_map(|path| manifest_from_skill_md_path(path))
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(manifests)
}

fn insert_manifest(
    manifests: &mut Vec<SnapshotSkillManifest>,
    seen_source_paths: &mut HashSet<String>,
    manifest: SnapshotSkillManifest,
) {
    if seen_source_paths.insert(manifest.source_path.clone()) {
        manifests.push(manifest);
    }
}

fn is_immediate_skill_manifest(path: &str, search_root: &str) -> bool {
    if !is_skill_md_repo_path(path) {
        return false;
    }
    if has_skipped_discovery_segment(path) {
        return false;
    }

    let Some(source_path) = source_path_from_skill_md(path) else {
        return false;
    };
    let source_segments = repo_path_segments(&source_path);
    let root_segments = repo_path_segments(search_root);
    source_segments.len() == root_segments.len() + 1 && source_segments.starts_with(&root_segments)
}

fn is_recursive_skill_manifest(path: &str, base_path: &str) -> bool {
    if !is_skill_md_repo_path(path) {
        return false;
    }
    if has_skipped_discovery_segment(path) {
        return false;
    }

    let Some(source_path) = source_path_from_skill_md(path) else {
        return false;
    };
    let source_segments = repo_path_segments(&source_path);
    let base_segments = repo_path_segments(base_path);
    source_segments.starts_with(&base_segments)
        && source_segments.len().saturating_sub(base_segments.len())
            <= RECURSIVE_DISCOVERY_MAX_DEPTH
}

fn manifest_from_skill_md_path(path: &str) -> Option<SnapshotSkillManifest> {
    let normalized = normalize_repo_path(path).ok()?;
    let source_path = source_path_from_skill_md(&normalized)?;
    let skill_directory_name = if source_path == "." {
        String::new()
    } else {
        source_path.rsplit('/').next()?.to_string()
    };
    let root_directory = if source_path == "." {
        "/".to_string()
    } else {
        source_path
            .rsplit_once('/')
            .map(|(root, _)| {
                if root.is_empty() {
                    "/".to_string()
                } else {
                    root.to_string()
                }
            })
            .unwrap_or_else(|| "/".to_string())
    };

    Some(SnapshotSkillManifest {
        source_path,
        root_directory,
        skill_directory_name,
        skill_md_path: normalized,
    })
}

fn source_path_from_skill_md(path: &str) -> Option<String> {
    let normalized = normalize_repo_path(path).ok()?;
    if normalized.eq_ignore_ascii_case("SKILL.md") {
        return Some(".".to_string());
    }

    let lower = normalized.to_ascii_lowercase();
    lower
        .strip_suffix("/skill.md")
        .map(|_| normalized[..normalized.len() - "/SKILL.md".len()].to_string())
}

fn join_repo_path(base_path: &str, child: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    for part in base_path.split('/').chain(child.split('/')) {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }
        parts.push(trimmed);
    }
    normalize_repo_path(&parts.join("/"))
}

fn normalize_repo_path(path: &str) -> Result<String, String> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        return Ok(String::new());
    }
    if !is_safe_repo_relative_path(&normalized) {
        return Err(format!("Repository path '{}' is not supported.", path));
    }
    Ok(normalized)
}

fn repo_path_segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn has_skipped_discovery_segment(path: &str) -> bool {
    repo_path_segments(path).iter().any(|segment| {
        SKIP_DISCOVERY_DIRS
            .iter()
            .any(|skip| segment.eq_ignore_ascii_case(skip))
    })
}

fn is_skill_md_repo_path(path: &str) -> bool {
    path.eq_ignore_ascii_case("SKILL.md") || path.to_ascii_lowercase().ends_with("/skill.md")
}

pub(crate) async fn download_repo_snapshot(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<GitHubRepoSnapshot, String> {
    let archive = download_repository_archive(client, repo, auth_token).await?;
    snapshot_from_repository_archive(&archive)
}

async fn download_repository_archive(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
) -> Result<Vec<u8>, String> {
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Api,
        |endpoint| {
            github_endpoint_url(
                endpoint,
                GitHubFetchSurface::Api,
                &format!(
                    "/repos/{}/{}/tarball/{}",
                    repo.owner, repo.repo, repo.branch
                ),
            )
        },
        "Failed to download GitHub repository archive",
        auth_token,
    )
    .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("GitHub repository archive is unavailable.".to_string());
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(classify_github_denial_response(
            response,
            "downloading the repository archive",
        )
        .await
        .unwrap_or_else(|| {
            format!(
                "Failed to download GitHub repository archive: HTTP {}",
                status
            )
        }));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| format!("Failed to read GitHub repository archive: {}", e))
}

fn snapshot_from_repository_archive(archive_bytes: &[u8]) -> Result<GitHubRepoSnapshot, String> {
    let cursor = Cursor::new(archive_bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);
    let mut files = HashMap::new();

    for entry_result in archive
        .entries()
        .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?
    {
        let mut entry = entry_result
            .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?;

        if !entry.header().entry_type().is_file() {
            continue;
        }

        let relative_path = relative_archive_path(&entry)?;
        let mut content = Vec::new();
        entry.read_to_end(&mut content).map_err(|e| {
            format!(
                "Failed to read GitHub repository archive entry '{}': {}",
                relative_path, e
            )
        })?;
        files.insert(relative_path, content);
    }

    Ok(GitHubRepoSnapshot { files })
}

fn relative_archive_path<R: Read>(entry: &tar::Entry<'_, R>) -> Result<String, String> {
    let archive_path = entry
        .path()
        .map_err(|e| format!("Failed to inspect GitHub repository archive: {}", e))?;
    let relative = archive_path
        .components()
        .skip(1)
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err("GitHub repository archive contains an unsupported path.".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if relative.is_empty() {
        return Err("GitHub repository archive contains an unsupported path.".to_string());
    }

    let joined = relative.join("/");
    if !is_safe_repo_relative_path(&joined) {
        return Err(format!(
            "GitHub repository archive contains an unsupported path '{}'.",
            joined
        ));
    }

    Ok(joined)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotSourceFile {
    repo_path: String,
    relative_path: String,
    byte_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GitHubImportProgressState {
    completed_files: usize,
    total_files: usize,
    completed_bytes: u64,
    total_bytes: u64,
}

fn collect_snapshot_source_files(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<Vec<SnapshotSourceFile>, String> {
    let mut files = snapshot
        .files
        .iter()
        .filter_map(|(path, bytes)| {
            let relative_path = if source_path == "." {
                if path.contains('/') {
                    return None;
                }
                path.clone()
            } else {
                let prefix = format!("{}/", source_path.trim_matches('/'));
                let relative = path.strip_prefix(&prefix)?;
                if relative.is_empty() {
                    return None;
                }
                relative.to_string()
            };

            Some(SnapshotSourceFile {
                repo_path: path.clone(),
                relative_path,
                byte_len: bytes.len(),
            })
        })
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));

    if files.is_empty() {
        return Err(format!(
            "Repository path '{}' is no longer available in the archive.",
            source_path
        ));
    }

    Ok(files)
}

fn write_snapshot_source_to_target(
    snapshot: &GitHubRepoSnapshot,
    files: &[SnapshotSourceFile],
    target_dir: &Path,
    source_path: &str,
    progress_state: &mut GitHubImportProgressState,
    app: Option<&AppHandle>,
) -> Result<(), String> {
    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create import target directory: {}", e))?;

    for file in files {
        if !is_safe_repo_relative_path(&file.relative_path) {
            return Err(format!(
                "Repository contains an unsupported path '{}'.",
                file.repo_path
            ));
        }

        let bytes = snapshot.files.get(&file.repo_path).ok_or_else(|| {
            format!(
                "Repository file '{}' is no longer available in the archive.",
                file.repo_path
            )
        })?;

        let destination = target_dir.join(&file.relative_path);
        let parent = destination
            .parent()
            .ok_or_else(|| "Failed to determine imported file parent directory.".to_string())?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create imported file parent directory: {}", e))?;
        std::fs::write(&destination, bytes).map_err(|e| {
            format!(
                "Failed to write imported file '{}': {}",
                destination.display(),
                e
            )
        })?;

        progress_state.completed_files += 1;
        progress_state.completed_bytes += file.byte_len as u64;
        emit_github_import_progress(
            app,
            GitHubImportProgressPayload {
                phase: GitHubImportProgressPhase::Writing,
                current_skill: Some(source_path.to_string()),
                current_path: Some(file.relative_path.clone()),
                completed_files: progress_state.completed_files,
                total_files: progress_state.total_files,
                completed_bytes: progress_state.completed_bytes,
                total_bytes: progress_state.total_bytes,
            },
        );
    }

    Ok(())
}

fn emit_github_import_progress(app: Option<&AppHandle>, payload: GitHubImportProgressPayload) {
    if let Some(app) = app {
        let _ = app.emit("github-import:progress", payload);
    }
}

fn is_safe_repo_relative_path(path: &str) -> bool {
    let relative = Path::new(path);
    !relative.is_absolute()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

async fn fetch_github_skill_markdown_from_remote_workspace(
    state: &State<'_, AppState>,
    workspace_id: &str,
    source_path: Option<&str>,
) -> Result<String, String> {
    let workspace = get_preview_workspace(workspace_id).ok_or_else(|| {
        "GitHub preview workspace has expired. Preview the repository again.".to_string()
    })?;
    if workspace.is_expired(Utc::now()) {
        let _ = take_preview_workspace(workspace_id);
        return Err(
            "GitHub preview workspace has expired. Preview the repository again.".to_string(),
        );
    }
    let source_path = source_path.ok_or_else(|| {
        "A source path is required for remote GitHub markdown preview.".to_string()
    })?;
    let active_target = state.active_target().await?;
    let target = match active_target {
        ActiveTarget::Ssh(target) if target.id == workspace.target_id => target,
        ActiveTarget::Ssh(_) => {
            return Err(
                "The active SSH target changed after preview. Preview the repository again."
                    .to_string(),
            )
        }
        ActiveTarget::Local => {
            return Err(
                "Remote GitHub preview workspace is only available on its SSH target.".to_string(),
            )
        }
    };
    let skill_md_path = if source_path == "." {
        "SKILL.md".to_string()
    } else {
        join_repo_path(source_path, "SKILL.md")?
    };
    let connection = connect_ssh_target(&target).await?;
    let bytes = connection
        .read_file(&remote_join(&workspace.remote_repo_dir, &skill_md_path))
        .await?;
    String::from_utf8(bytes).map_err(|e| format!("Remote SKILL.md is not valid UTF-8: {}", e))
}

async fn discard_preview_workspace_for_active_target(
    state: &State<'_, AppState>,
    workspace_id: &str,
) {
    let Some(workspace) = take_preview_workspace(workspace_id) else {
        return;
    };
    let Ok(active_target) = state.active_target().await else {
        return;
    };
    let ActiveTarget::Ssh(target) = active_target else {
        return;
    };
    if target.id != workspace.target_id {
        return;
    }
    if let Ok(connection) = connect_ssh_target(&target).await {
        let _ = connection
            .remove_tree(&workspace.remote_workspace_dir)
            .await;
    }
}

async fn fetch_raw_text(
    client: &reqwest::Client,
    url: &str,
    auth_token: Option<&str>,
) -> Result<String, String> {
    let response = send_github_request_with_fallback(
        client,
        GitHubFetchSurface::Raw,
        |endpoint| {
            if let Some(path) = raw_url_to_repo_path(url) {
                raw_file_url(endpoint, &path.repo, &path.file_path)
            } else {
                url.to_string()
            }
        },
        "Failed to download skill metadata",
        auth_token,
    )
    .await?;

    if !response.status().is_success() {
        return Err(
            classify_github_denial_response(response, "downloading skill metadata")
                .await
                .unwrap_or_else(|| "Failed to download skill metadata.".to_string()),
        );
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read skill metadata: {}", e))
}

#[derive(Debug, Clone)]
struct RawRepoPath {
    repo: GitHubRepoRef,
    file_path: String,
}

fn raw_url_to_repo_path(url: &str) -> Option<RawRepoPath> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if host != "raw.githubusercontent.com" {
        return None;
    }

    let segments = parsed.path_segments()?;
    let parts = segments.collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }

    Some(RawRepoPath {
        repo: GitHubRepoRef {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
            branch: parts[2].to_string(),
            normalized_url: format!("https://github.com/{}/{}", parts[0], parts[1]),
        },
        file_path: parts[3..].join("/"),
    })
}

fn github_endpoint_url(
    endpoint: &GitHubMirrorEndpoint,
    surface: GitHubFetchSurface,
    path: &str,
) -> String {
    let base = match surface {
        GitHubFetchSurface::Api => endpoint.api_base,
        GitHubFetchSurface::Raw => endpoint.raw_base,
    };
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn raw_file_url(endpoint: &GitHubMirrorEndpoint, repo: &GitHubRepoRef, file_path: &str) -> String {
    github_endpoint_url(
        endpoint,
        GitHubFetchSurface::Raw,
        &format!(
            "/{}/{}/{}/{}",
            repo.owner,
            repo.repo,
            repo.branch,
            file_path.trim_start_matches('/')
        ),
    )
}

async fn send_github_request_with_fallback<F>(
    client: &reqwest::Client,
    surface: GitHubFetchSurface,
    build_url: F,
    failure_prefix: &str,
    auth_token: Option<&str>,
) -> Result<reqwest::Response, String>
where
    F: Fn(&GitHubMirrorEndpoint) -> String,
{
    let mut attempts = Vec::new();
    let mut last_retryable_denial = None;

    for endpoint in GITHUB_MIRROR_ENDPOINTS {
        let url = build_url(endpoint);
        let mut request = client.get(&url);
        let mirrors_share_same_url = GITHUB_MIRROR_ENDPOINTS
            .iter()
            .filter(|candidate| candidate.label != "github")
            .any(|candidate| build_url(candidate) == url);
        if endpoint.label == "github" && !mirrors_share_same_url {
            if let Some(token) = auth_token {
                request = request.bearer_auth(token);
            }
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if matches!(
                    status,
                    reqwest::StatusCode::UNAUTHORIZED
                        | reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::TOO_MANY_REQUESTS
                ) {
                    let denial = parse_github_denial_response(
                        response,
                        "contacting GitHub",
                        auth_token.is_some(),
                    )
                    .await;
                    let can_retry_public_mirror = auth_token.is_none()
                        && denial.as_ref().is_some_and(|denial| {
                            matches!(denial.kind, GitHubAccessDenialKind::RateLimited { .. })
                        });
                    if can_retry_public_mirror {
                        last_retryable_denial = denial;
                        attempts.push(MirrorAttemptOutcome {
                            status: Some(status),
                            error_message: format!(
                                "{} mirror '{}' returned HTTP {} due to rate limiting",
                                surface_label(surface),
                                endpoint.label,
                                status
                            ),
                        });
                        continue;
                    }

                    return Err(denial
                        .map(|denial| denial.to_string())
                        .unwrap_or_else(|| format!("{}: HTTP {}", failure_prefix, status)));
                }

                if status.is_success() {
                    return Ok(response);
                }

                if status == reqwest::StatusCode::NOT_FOUND {
                    if last_retryable_denial.is_some() && auth_token.is_none() {
                        attempts.push(MirrorAttemptOutcome {
                            status: Some(status),
                            error_message: format!(
                                "{} mirror '{}' returned HTTP 404 after a prior rate-limit denial",
                                surface_label(surface),
                                endpoint.label
                            ),
                        });
                        continue;
                    }
                    return Ok(response);
                }

                if should_retry_via_mirror_status(surface, status) {
                    attempts.push(MirrorAttemptOutcome {
                        status: Some(status),
                        error_message: format!(
                            "{} mirror '{}' returned HTTP {}",
                            surface_label(surface),
                            endpoint.label,
                            status
                        ),
                    });
                    continue;
                }

                return Err(format!("{}: HTTP {}", failure_prefix, status));
            }
            Err(error) => {
                if is_retryable_github_transport_error(&error) {
                    attempts.push(MirrorAttemptOutcome {
                        status: error.status(),
                        error_message: format!(
                            "{} mirror '{}' failed: {}",
                            surface_label(surface),
                            endpoint.label,
                            error
                        ),
                    });
                    continue;
                }

                return Err(format!("{}: {}", failure_prefix, error));
            }
        }
    }

    if let Some(denial) = last_retryable_denial {
        return Err(denial.to_string());
    }

    Err(format!(
        "{}. Direct GitHub access and built-in mirrors were unreachable. Retry later or try a different network path. Last errors: {}",
        failure_prefix,
        summarize_mirror_attempts(&attempts)
    ))
}

fn should_retry_via_mirror_status(
    surface: GitHubFetchSurface,
    status: reqwest::StatusCode,
) -> bool {
    match surface {
        GitHubFetchSurface::Api | GitHubFetchSurface::Raw => {
            status.is_server_error()
                || status == reqwest::StatusCode::BAD_GATEWAY
                || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                || status == reqwest::StatusCode::GATEWAY_TIMEOUT
        }
    }
}

fn is_retryable_github_transport_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request() || error.is_body()
}

fn summarize_mirror_attempts(attempts: &[MirrorAttemptOutcome]) -> String {
    attempts
        .iter()
        .map(|attempt| attempt.error_message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

fn surface_label(surface: GitHubFetchSurface) -> &'static str {
    match surface {
        GitHubFetchSurface::Api => "API",
        GitHubFetchSurface::Raw => "raw",
    }
}

async fn classify_github_denial_response(
    response: reqwest::Response,
    operation: &'static str,
) -> Option<String> {
    parse_github_denial_response(response, operation, false)
        .await
        .map(|denial| denial.to_string())
}

async fn parse_github_denial_response(
    response: reqwest::Response,
    operation: &'static str,
    used_auth: bool,
) -> Option<GitHubAccessDenial> {
    let status = response.status();
    if status != reqwest::StatusCode::UNAUTHORIZED
        && status != reqwest::StatusCode::FORBIDDEN
        && status != reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return None;
    }

    let headers = response.headers().clone();
    let body = response.text().await.ok();
    let github_message = body.as_deref().and_then(parse_github_error_message);

    let remaining = header_value(&headers, "x-ratelimit-remaining");
    let reset_at = header_value(&headers, "x-ratelimit-reset")
        .as_deref()
        .and_then(parse_rate_limit_reset_epoch);

    let message_lower = github_message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let remaining_is_zero = remaining.as_deref() == Some("0");
    let kind = if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || remaining_is_zero
        || message_lower.contains("rate limit")
        || message_lower.contains("api rate limit exceeded")
        || header_value(&headers, "x-ratelimit-resource").is_some()
    {
        GitHubAccessDenialKind::RateLimited {
            reset_at,
            remaining,
        }
    } else {
        GitHubAccessDenialKind::AuthenticationOrPermission
    };

    Some(GitHubAccessDenial {
        kind,
        operation,
        status,
        github_message,
        used_auth,
    })
}

fn parse_github_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<GitHubErrorResponse>(body)
        .ok()
        .and_then(|payload| payload.message)
}

fn header_value(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_rate_limit_reset_epoch(raw: &str) -> Option<String> {
    let epoch = raw.parse::<i64>().ok()?;
    chrono::DateTime::<Utc>::from_timestamp(epoch, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub(crate) fn parse_frontmatter(content: &str) -> Option<SkillFrontmatter> {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("---")?;
    serde_yaml::from_str::<SkillFrontmatter>(&rest[..end]).ok()
}

fn sanitize_skill_id(raw: &str) -> Result<String, String> {
    let lowered = raw.trim().to_lowercase();
    let mut sanitized = String::new();
    let mut last_was_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }
    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        return Err(format!("Skill identifier '{}' is not supported.", raw));
    }
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::collections::HashMap;
    use tempfile::tempdir;

    async fn setup_test_db() -> DbPool {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("github-import.sqlite");
        let pool = db::create_pool(db_path.to_str().unwrap())
            .await
            .expect("create db");
        db::init_database(&pool).await.expect("init db");
        std::mem::forget(dir);
        pool
    }

    fn sample_frontmatter(name: &str, description: &str) -> String {
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n")
    }

    fn repo_snapshot(files: &[(&str, String)]) -> GitHubRepoSnapshot {
        GitHubRepoSnapshot {
            files: files
                .iter()
                .map(|(path, content)| (path.to_string(), content.as_bytes().to_vec()))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn root_repo_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "SKILL.md",
                sample_frontmatter("twitterapi-io", "root skill"),
            ),
            ("README.md", "# repo\n".to_string()),
        ])
    }

    fn multi_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/agent-planner/SKILL.md",
                sample_frontmatter("Agent Planner", "Agent Planner description"),
            ),
            (
                "skills/commit/SKILL.md",
                sample_frontmatter("Commit", "Commit description"),
            ),
            (
                "skills/code-review/SKILL.md",
                sample_frontmatter("Code Review", "Code Review description"),
            ),
            ("skills/commit/README.md", "# commit\n".to_string()),
        ])
    }

    fn namespaced_skill_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/.curated/openai-docs/SKILL.md",
                sample_frontmatter("openai-docs", "OpenAI docs skill"),
            ),
            (
                "skills/.curated/openai-docs/references/api.md",
                "# api\n".to_string(),
            ),
            (
                "skills/.system/skill-creator/SKILL.md",
                sample_frontmatter("skill-creator", "Create skills"),
            ),
            (
                "skills/.system/skill-creator/scripts/init_skill.py",
                "print('hi')\n".to_string(),
            ),
        ])
    }

    fn content_skills_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "content/skills/development-workflows/code-auditor/SKILL.md",
                sample_frontmatter("code-auditor", "Audit code"),
            ),
            (
                "content/skills/development-workflows/code-auditor/references/checklist.md",
                "# checklist\n".to_string(),
            ),
            (
                "content/skills/git-github-collaboration/git-commit/SKILL.md",
                sample_frontmatter("git-commit", "Commit changes"),
            ),
            ("README.md", "# repo\n".to_string()),
        ])
    }

    fn agent_path_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                ".agents/skills/universal-skill/SKILL.md",
                sample_frontmatter("universal-skill", "Universal skill"),
            ),
            (
                ".claude/skills/claude-skill/SKILL.md",
                sample_frontmatter("claude-skill", "Claude skill"),
            ),
            (
                ".codex/skills/codex-skill/SKILL.md",
                sample_frontmatter("codex-skill", "Codex skill"),
            ),
        ])
    }

    fn recursive_fallback_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "packages/example/skill/SKILL.md",
                sample_frontmatter("fallback-skill", "Fallback skill"),
            ),
            (
                "node_modules/example/ignored/SKILL.md",
                sample_frontmatter("ignored-node-module", "Ignored"),
            ),
            (
                "target/example/ignored/SKILL.md",
                sample_frontmatter("ignored-target", "Ignored"),
            ),
        ])
    }

    fn duplicate_name_snapshot() -> GitHubRepoSnapshot {
        repo_snapshot(&[
            (
                "skills/preferred/SKILL.md",
                sample_frontmatter("duplicate-skill", "Preferred"),
            ),
            (
                "packages/fallback/duplicate/SKILL.md",
                sample_frontmatter("duplicate-skill", "Fallback"),
            ),
        ])
    }

    fn repository_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for (path, content) in files {
            let archive_path = format!("repo-snapshot/{}", path);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append_data(&mut header, archive_path, *content)
                .expect("append archive entry");
        }
        let encoder = builder.into_inner().expect("finalize tar");
        encoder.finish().expect("finalize gzip")
    }

    #[test]
    fn parse_github_source_normalizes_owner_repo_and_subpath() {
        let parsed = parse_github_source("https://github.com/Anthropics/Skills/content/skills/")
            .expect("parse");
        assert_eq!(parsed.owner, "anthropics");
        assert_eq!(parsed.repo, "skills");
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_accepts_shorthand_repo_subpaths() {
        let parsed = parse_github_source("bahayonghang/my-claude-code-settings/content/skills")
            .expect("parse");
        assert_eq!(parsed.owner, "bahayonghang");
        assert_eq!(parsed.repo, "my-claude-code-settings");
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_accepts_tree_urls() {
        let parsed = parse_github_source(
            "https://github.com/bahayonghang/my-claude-code-settings/tree/main/content/skills",
        )
        .expect("parse");
        assert_eq!(parsed.owner, "bahayonghang");
        assert_eq!(parsed.repo, "my-claude-code-settings");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.source_path.as_deref(), Some("content/skills"));
    }

    #[test]
    fn parse_github_source_rejects_non_github_hosts() {
        let error = parse_github_source("https://gitlab.com/example/repo").unwrap_err();
        assert!(error.contains("github.com"));
    }

    #[test]
    fn parse_github_source_rejects_unsafe_subpaths() {
        let error = parse_github_source("owner/repo/../escape").unwrap_err();
        assert!(error.contains("not supported"));
    }

    #[test]
    fn sanitize_skill_id_collapses_symbols() {
        let skill_id = sanitize_skill_id("My Cool_Skill!").expect("sanitize");
        assert_eq!(skill_id, "my-cool-skill");
    }

    #[test]
    fn parse_frontmatter_requires_yaml_block() {
        assert!(parse_frontmatter("# nope").is_none());
        let parsed = parse_frontmatter(&sample_frontmatter("alpha", "desc")).expect("fm");
        assert_eq!(parsed.name, "alpha");
        assert_eq!(parsed.description.as_deref(), Some("desc"));
    }

    #[test]
    fn classify_github_rate_limit_denial_returns_actionable_message() {
        let denial = GitHubAccessDenial {
            kind: GitHubAccessDenialKind::RateLimited {
                reset_at: Some("2026-04-17 12:34:56".to_string()),
                remaining: Some("0".to_string()),
            },
            operation: "inspecting the repository",
            status: reqwest::StatusCode::FORBIDDEN,
            github_message: Some("API rate limit exceeded for 1.2.3.4.".to_string()),
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("rate limit was exceeded"));
        assert!(message.contains("Retry later after 2026-04-17 12:34:56 UTC"));
        assert!(message.contains("authenticated GitHub requests"));
        assert!(message.contains("API rate limit exceeded"));
    }

    #[test]
    fn classify_github_permission_denial_returns_actionable_message() {
        let denial = GitHubAccessDenial {
            kind: GitHubAccessDenialKind::AuthenticationOrPermission,
            operation: "reading repository contents",
            status: reqwest::StatusCode::UNAUTHORIZED,
            github_message: Some("Requires authentication".to_string()),
            used_auth: false,
        };

        let message = denial.to_string();

        assert!(message.contains("denied access"));
        assert!(message.contains("require authentication"));
        assert!(message.contains("token/permissions are insufficient"));
        assert!(message.contains("Requires authentication"));
    }

    #[test]
    fn raw_url_to_repo_path_parses_github_raw_urls() {
        let parsed = raw_url_to_repo_path(
            "https://raw.githubusercontent.com/owner/repo/main/skills/demo/SKILL.md",
        )
        .expect("parsed");

        assert_eq!(parsed.repo.owner, "owner");
        assert_eq!(parsed.repo.repo, "repo");
        assert_eq!(parsed.repo.branch, "main");
        assert_eq!(parsed.file_path, "skills/demo/SKILL.md");
    }

    #[test]
    fn raw_url_to_repo_path_ignores_non_github_raw_hosts() {
        assert!(raw_url_to_repo_path("https://example.com/file.txt").is_none());
    }

    #[test]
    fn mirror_status_retry_excludes_auth_denials() {
        assert!(should_retry_via_mirror_status(
            GitHubFetchSurface::Api,
            reqwest::StatusCode::BAD_GATEWAY
        ));
        assert!(!should_retry_via_mirror_status(
            GitHubFetchSurface::Api,
            reqwest::StatusCode::FORBIDDEN
        ));
        assert!(!should_retry_via_mirror_status(
            GitHubFetchSurface::Raw,
            reqwest::StatusCode::TOO_MANY_REQUESTS
        ));
    }

    #[test]
    fn summarize_mirror_attempts_reports_all_failures() {
        let message = summarize_mirror_attempts(&[
            MirrorAttemptOutcome {
                status: None,
                error_message: "API mirror 'github' failed: timeout".to_string(),
            },
            MirrorAttemptOutcome {
                status: Some(reqwest::StatusCode::BAD_GATEWAY),
                error_message: "API mirror 'ghfast' returned HTTP 502".to_string(),
            },
        ]);

        assert!(message.contains("timeout"));
        assert!(message.contains("HTTP 502"));
    }

    #[test]
    fn snapshot_from_repository_archive_strips_archive_root_directory() {
        let archive = repository_archive(&[
            (
                "skills/demo/SKILL.md",
                sample_frontmatter("Demo", "Archive demo").as_bytes(),
            ),
            ("README.md", b"# readme\n"),
        ]);

        let snapshot = snapshot_from_repository_archive(&archive).expect("snapshot");

        assert!(snapshot.files.contains_key("skills/demo/SKILL.md"));
        assert!(snapshot.files.contains_key("README.md"));
    }

    #[tokio::test]
    async fn preview_marks_canonical_conflicts_without_writing() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let existing_dir = central_root.path().join("twitterapi-io");
        std::fs::create_dir_all(&existing_dir).expect("mkdir");
        std::fs::write(
            existing_dir.join("SKILL.md"),
            sample_frontmatter("twitterapi-io", "existing"),
        )
        .expect("write skill");

        db::upsert_skill(
            &pool,
            &Skill {
                id: "twitterapi-io".to_string(),
                name: "twitterapi-io".to_string(),
                description: Some("existing".to_string()),
                file_path: existing_dir.join("SKILL.md").to_string_lossy().into_owned(),
                canonical_path: Some(existing_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .expect("upsert skill");

        let repo = GitHubRepoRef {
            owner: "dorukardahan".to_string(),
            repo: "twitterapi-io-skill".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/dorukardahan/twitterapi-io-skill".to_string(),
        };
        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &root_repo_snapshot())
            .expect("candidates");
        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills"),
            preview_workspace_id: None,
        };

        assert!(!preview.skills.is_empty());
        let conflict = preview
            .skills
            .iter()
            .find(|skill| skill.skill_id == "twitterapi-io")
            .and_then(|skill| skill.conflict.clone())
            .expect("conflict");
        assert_eq!(conflict.existing_skill_id, "twitterapi-io");

        let central_entries = std::fs::read_dir(central_root.path())
            .expect("read dir")
            .count();
        assert_eq!(central_entries, 1, "preview should not write to central");
    }

    #[tokio::test]
    async fn import_staging_allows_reclaiming_non_central_record_after_delete() {
        let pool = setup_test_db().await;
        let candidate = RemoteSkillCandidate {
            source_path: "skills/web-access".to_string(),
            skill_id: "web-access".to_string(),
            skill_name: "web-access".to_string(),
            description: Some("remote import".to_string()),
            root_directory: "skills".to_string(),
            skill_directory_name: "web-access".to_string(),
            download_url: "https://raw.githubusercontent.com/eze-is/web-access/main/SKILL.md"
                .to_string(),
        };

        db::upsert_skill(
            &pool,
            &Skill {
                id: "web-access".to_string(),
                name: "web-access".to_string(),
                description: Some("platform copy left after central delete".to_string()),
                file_path: "/tmp/codex/web-access/SKILL.md".to_string(),
                canonical_path: None,
                is_central: false,
                source: Some("copy".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .expect("seed non-central record");

        let preview = build_preview_skills(&pool, std::slice::from_ref(&candidate))
            .await
            .expect("preview");
        assert!(
            preview[0].conflict.is_none(),
            "non-central rows should not be presented as Central overwrite conflicts"
        );

        let (staging_ops, skipped_skills) = plan_import_staging(
            &pool,
            std::slice::from_ref(&candidate),
            vec![GitHubSkillImportSelection {
                source_path: candidate.source_path.clone(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
        )
        .await
        .expect("stage import");

        assert!(skipped_skills.is_empty());
        assert_eq!(staging_ops.len(), 1);
        assert_eq!(staging_ops[0].final_skill_id, "web-access");
        assert_eq!(staging_ops[0].resolution, DuplicateResolution::Overwrite);
    }

    #[tokio::test]
    async fn import_repo_skills_honors_skip_rename_and_overwrite() {
        let pool = setup_test_db().await;
        let snapshot = multi_skill_snapshot();
        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &snapshot).expect("candidates");

        let agent_planner = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/agent-planner")
            .expect("agent planner");
        let commit = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/commit")
            .expect("commit");
        let code_review = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/code-review")
            .expect("code review");

        db::upsert_skill(
            &pool,
            &Skill {
                id: agent_planner.skill_id.clone(),
                name: "Agent Planner".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/agent-planner/SKILL.md".to_string(),
                canonical_path: Some("/tmp/agent-planner".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .expect("seed rename conflict");
        db::upsert_skill(
            &pool,
            &Skill {
                id: commit.skill_id.clone(),
                name: "Commit".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/commit/SKILL.md".to_string(),
                canonical_path: Some("/tmp/commit".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .expect("seed skip conflict");
        db::upsert_skill(
            &pool,
            &Skill {
                id: code_review.skill_id.clone(),
                name: "Code Review".to_string(),
                description: Some("existing".to_string()),
                file_path: "/tmp/code-review/SKILL.md".to_string(),
                canonical_path: Some("/tmp/code-review".to_string()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .expect("seed overwrite conflict");

        let mut occupied = current_central_skill_ids(&pool).await.expect("occupied");
        assert!(occupied.contains(&agent_planner.skill_id));
        assert!(occupied.contains(&commit.skill_id));
        assert!(occupied.contains(&code_review.skill_id));

        let rename_target = sanitize_skill_id("agent-planner-imported").expect("rename target");
        assert!(
            !occupied.contains(&rename_target),
            "rename target should be available before import"
        );
        occupied.insert(rename_target.clone());

        assert!(
            occupied.contains(&rename_target),
            "rename should reserve the requested canonical id"
        );
        assert!(
            occupied.contains(&code_review.skill_id),
            "overwrite keeps the original canonical id occupied"
        );
        assert!(
            occupied.contains(&commit.skill_id),
            "skip leaves the existing canonical id occupied without needing a new id"
        );
    }

    #[tokio::test]
    async fn import_invalid_repo_leaves_central_storage_unchanged() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let result = import_github_repo_skills_impl(
            &pool,
            "https://github.com/example/definitely-missing-repo",
            vec![GitHubSkillImportSelection {
                source_path: "skills/foo".to_string(),
                resolution: DuplicateResolution::Skip,
                renamed_skill_id: None,
            }],
            None,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            std::fs::read_dir(central_root.path())
                .expect("read central")
                .count(),
            0
        );
        let central_skills = db::get_central_skills(&pool).await.expect("central skills");
        assert!(central_skills.is_empty());
    }

    #[tokio::test]
    async fn denied_import_selection_performs_no_writes_or_db_mutations() {
        let pool = setup_test_db().await;
        let central_root = tempdir().expect("central");
        sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
            .bind(central_root.path().to_string_lossy().into_owned())
            .execute(&pool)
            .await
            .expect("update central");

        let before_skills = db::get_central_skills(&pool).await.expect("before skills");
        let before_entries = std::fs::read_dir(central_root.path())
            .expect("read central before")
            .count();

        let result = import_github_repo_skills_impl(
            &pool,
            "https://github.com/example/restricted-repo",
            vec![GitHubSkillImportSelection {
                source_path: "skills/private-skill".to_string(),
                resolution: DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            None,
        )
        .await;

        let error = result.expect_err("denied import should fail");
        assert!(
            !error.trim().is_empty(),
            "failure should return an error message"
        );

        let after_skills = db::get_central_skills(&pool).await.expect("after skills");
        let after_entries = std::fs::read_dir(central_root.path())
            .expect("read central after")
            .count();
        assert_eq!(
            before_entries, after_entries,
            "denied import should not write files"
        );
        assert_eq!(
            before_skills.len(),
            after_skills.len(),
            "denied import should not mutate DB"
        );
    }

    #[tokio::test]
    async fn preview_top_level_skills_directory_discovers_candidates() {
        let pool = setup_test_db().await;
        let repo = GitHubRepoRef {
            owner: "anthropics".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/anthropics/skills".to_string(),
        };
        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &multi_skill_snapshot())
            .expect("candidates");
        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("skills"),
            preview_workspace_id: None,
        };

        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path.starts_with("skills/")));
    }

    #[tokio::test]
    async fn preview_namespaced_skills_directory_discovers_candidates() {
        let pool = setup_test_db().await;
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &namespaced_skill_snapshot())
                .expect("candidates");

        assert_eq!(
            candidates.len(),
            2,
            "expected two namespaced skill candidates"
        );

        let curated = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/.curated/openai-docs")
            .expect("curated skill");
        assert_eq!(curated.root_directory, "skills/.curated");
        assert_eq!(curated.skill_directory_name, "openai-docs");
        assert_eq!(curated.skill_id, "openai-docs");

        let system = candidates
            .iter()
            .find(|candidate| candidate.source_path == "skills/.system/skill-creator")
            .expect("system skill");
        assert_eq!(system.root_directory, "skills/.system");
        assert_eq!(system.skill_directory_name, "skill-creator");
        assert_eq!(system.skill_id, "skill-creator");

        let preview = GitHubRepoPreview {
            repo,
            skills: build_preview_skills(&pool, &candidates)
                .await
                .expect("preview skills"),
            preview_workspace_id: None,
        };

        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path == "skills/.curated/openai-docs"));
        assert!(preview
            .skills
            .iter()
            .any(|skill| skill.source_path == "skills/.system/skill-creator"));
    }

    #[test]
    fn preview_content_skills_catalog_is_found_by_recursive_fallback() {
        let repo = GitHubRepoRef {
            owner: "bahayonghang".to_string(),
            repo: "my-claude-code-settings".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/bahayonghang/my-claude-code-settings".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &content_skills_snapshot())
                .expect("candidates");

        assert!(candidates.iter().any(|candidate| {
            candidate.source_path == "content/skills/development-workflows/code-auditor"
                && candidate.skill_id == "code-auditor"
                && candidate.root_directory == "content/skills/development-workflows"
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.source_path == "content/skills/git-github-collaboration/git-commit"
        }));
    }

    #[test]
    fn preview_content_skills_subpath_discovers_catalog() {
        let repo = GitHubRepoRef {
            owner: "bahayonghang".to_string(),
            repo: "my-claude-code-settings".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/bahayonghang/my-claude-code-settings".to_string(),
        };

        let candidates = build_repo_skill_candidates_from_snapshot_at_path(
            &repo,
            &content_skills_snapshot(),
            Some("content/skills"),
        )
        .expect("candidates");

        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.source_path.starts_with("content/skills/")));
    }

    #[test]
    fn preview_agent_specific_skill_roots_are_supported() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "agent-paths".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/agent-paths".to_string(),
        };

        let candidates = build_repo_skill_candidates_from_snapshot(&repo, &agent_path_snapshot())
            .expect("candidates");

        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".agents/skills/universal-skill"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".claude/skills/claude-skill"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.source_path == ".codex/skills/codex-skill"));
    }

    #[test]
    fn recursive_fallback_skips_large_generated_directories() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "fallback".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/fallback".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &recursive_fallback_snapshot())
                .expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, "packages/example/skill");
        assert_eq!(candidates[0].skill_id, "skill");
    }

    #[test]
    fn duplicate_skill_names_keep_priority_manifest() {
        let repo = GitHubRepoRef {
            owner: "example".to_string(),
            repo: "duplicates".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/example/duplicates".to_string(),
        };

        let candidates =
            build_repo_skill_candidates_from_snapshot(&repo, &duplicate_name_snapshot())
                .expect("candidates");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source_path, "skills/preferred");
        assert_eq!(candidates[0].description.as_deref(), Some("Preferred"));
    }

    #[test]
    fn remote_manifest_discovery_preserves_snapshot_priority_order() {
        let paths = [
            "packages/fallback/skill/SKILL.md",
            ".agents/skills/universal/SKILL.md",
            "skills/agent-planner/SKILL.md",
            "SKILL.md",
        ];

        let manifests =
            discover_skill_manifests_from_paths(paths.iter().copied(), None).expect("manifests");

        assert_eq!(manifests[0].source_path, ".");
        assert!(manifests
            .iter()
            .any(|manifest| manifest.source_path == "skills/agent-planner"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.source_path == ".agents/skills/universal"));
        assert!(!manifests
            .iter()
            .any(|manifest| manifest.source_path == "packages/fallback/skill"));
    }

    #[test]
    fn remote_manifest_discovery_honors_source_subpath() {
        let paths = [
            "content/skills/code/SKILL.md",
            "content/skills/git/SKILL.md",
            "other/skills/ignored/SKILL.md",
        ];

        let manifests =
            discover_skill_manifests_from_paths(paths.iter().copied(), Some("content/skills"))
                .expect("manifests");

        assert_eq!(manifests.len(), 2);
        assert!(manifests
            .iter()
            .all(|manifest| manifest.source_path.starts_with("content/skills/")));
    }

    #[test]
    fn remote_import_script_uses_remote_copy_and_move_not_streamed_cat() {
        let script = remote_import_skill_script();

        assert!(script.contains("cp -a"));
        assert!(script.contains("mv \"$stage_dir\" \"$target_dir\""));
        assert!(!script.contains("cat >"));
    }

    #[test]
    fn preview_workspace_reuse_requires_matching_target_repo_and_path() {
        let repo = GitHubRepoRef {
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/openai/skills".to_string(),
        };
        let now = Utc::now();
        let workspace = GitHubPreviewWorkspace {
            id: "workspace-1".to_string(),
            target_id: "ssh-demo".to_string(),
            repo: repo.clone(),
            source_path: Some("content/skills".to_string()),
            remote_workspace_dir: "/tmp/skillport-github-preview.abc123".to_string(),
            remote_repo_dir: "/tmp/skillport-github-preview.abc123/repo".to_string(),
            created_at: now,
            expires_at: now + Duration::minutes(30),
        };

        assert!(workspace.matches_source("ssh-demo", &repo, Some("content/skills")));
        assert!(!workspace.matches_source("ssh-other", &repo, Some("content/skills")));
        assert!(!workspace.matches_source("ssh-demo", &repo, Some("other")));

        let other_repo = GitHubRepoRef {
            repo: "other".to_string(),
            ..repo
        };
        assert!(!workspace.matches_source("ssh-demo", &other_repo, Some("content/skills")));
    }

    #[test]
    fn remote_workspace_download_script_puts_token_only_in_stdin_script() {
        let token = "ghp_secret_for_test";
        let script = remote_workspace_download_script(Some(token)).expect("script");
        let command = crate::targets::shell_quote("sh -s --");

        assert!(script.contains("curl.conf"));
        assert!(script.contains("Authorization: Bearer ghp_secret_for_test"));
        assert!(
            !command.contains(token),
            "ssh command string must not contain the GitHub token"
        );
    }

    #[test]
    fn nested_import_copy_is_limited_to_selected_skill_directory() {
        let snapshot = content_skills_snapshot();
        let files = collect_snapshot_source_files(
            &snapshot,
            "content/skills/development-workflows/code-auditor",
        )
        .expect("files");

        assert!(files.iter().any(|file| file.relative_path == "SKILL.md"));
        assert!(files
            .iter()
            .any(|file| file.relative_path == "references/checklist.md"));
        assert!(!files
            .iter()
            .any(|file| file.repo_path.contains("git-commit")));
    }

    #[tokio::test]
    async fn github_pat_setting_is_trimmed_and_empty_values_are_ignored() {
        let pool = setup_test_db().await;

        db::set_setting(&pool, GITHUB_PAT_SETTING_KEY, "  test-token  ")
            .await
            .expect("set token");
        assert_eq!(
            github_direct_auth_from_settings(&pool)
                .await
                .expect("read token"),
            Some("test-token".to_string())
        );

        db::set_setting(&pool, GITHUB_PAT_SETTING_KEY, "   ")
            .await
            .expect("clear token");
        assert_eq!(
            github_direct_auth_from_settings(&pool)
                .await
                .expect("read empty"),
            None
        );
    }

    #[tokio::test]
    async fn authenticated_api_fallback_does_not_forward_bearer_auth_to_mirror() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                requests_clone
                    .lock()
                    .expect("lock")
                    .push(request_text.clone());
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if request_text.contains("GET /direct") {
                    let response =
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 11\r\n\r\nbad gateway";
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/direct", address);
        let mirror_url = format!("http://{}/mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Api,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "direct request failed",
            Some("direct-token"),
        )
        .await
        .expect("fallback response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        let direct_request = captured
            .iter()
            .find(|request| request.contains("GET /direct"))
            .expect("captured direct request");
        let mirror_request = captured
            .iter()
            .find(|request| request.contains("GET /mirror"))
            .expect("captured mirror request");
        assert!(
            direct_request.contains("authorization: Bearer direct-token")
                || direct_request.contains("Authorization: Bearer direct-token"),
            "direct github request should include bearer auth"
        );
        assert!(
            !mirror_request.contains("authorization: Bearer direct-token")
                && !mirror_request.contains("Authorization: Bearer direct-token"),
            "mirror request should not include bearer auth"
        );
    }

    #[tokio::test]
    async fn authenticated_raw_fallback_does_not_forward_bearer_auth_to_mirror() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                requests_clone
                    .lock()
                    .expect("lock")
                    .push(request_text.clone());
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if request_text.contains("GET /raw-direct") {
                    let response = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 19\r\n\r\nservice unavailable";
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/raw-direct", address);
        let mirror_url = format!("http://{}/raw-mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Raw,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "raw request failed",
            Some("direct-token"),
        )
        .await
        .expect("fallback response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        let direct_request = captured
            .iter()
            .find(|request| request.contains("GET /raw-direct"))
            .expect("captured direct request");
        let mirror_request = captured
            .iter()
            .find(|request| request.contains("GET /raw-mirror"))
            .expect("captured mirror request");
        assert!(
            direct_request.contains("authorization: Bearer direct-token")
                || direct_request.contains("Authorization: Bearer direct-token"),
            "direct raw request should include bearer auth"
        );
        assert!(
            !mirror_request.contains("authorization: Bearer direct-token")
                && !mirror_request.contains("Authorization: Bearer direct-token"),
            "mirror raw request should not include bearer auth"
        );
    }

    #[tokio::test]
    async fn unauthenticated_rate_limit_retries_public_mirror_before_failing() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let accepted = Arc::new(AtomicUsize::new(0));
        let requests_clone = Arc::clone(&requests);
        let accepted_clone = Arc::clone(&accepted);

        let server = std::thread::spawn(move || {
            while accepted_clone.load(Ordering::SeqCst) < 2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut buffer = [0_u8; 2048];
                let bytes_read = stream.read(&mut buffer).expect("read");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                let is_direct = request_text.contains("GET /direct");
                requests_clone.lock().expect("lock").push(request_text);
                accepted_clone.fetch_add(1, Ordering::SeqCst);

                if is_direct {
                    let response = concat!(
                        "HTTP/1.1 403 Forbidden\r\n",
                        "Content-Type: application/json\r\n",
                        "X-RateLimit-Remaining: 0\r\n",
                        "X-RateLimit-Reset: 1786576453\r\n",
                        "Content-Length: 48\r\n\r\n",
                        "{\"message\":\"API rate limit exceeded for 1.2.3.4\"}"
                    );
                    stream.write_all(response.as_bytes()).expect("write direct");
                } else {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    stream.write_all(response.as_bytes()).expect("write mirror");
                }
            }
        });

        let client = github_client().expect("client");
        let direct_url = format!("http://{}/direct", address);
        let mirror_url = format!("http://{}/mirror", address);

        let response = send_github_request_with_fallback(
            &client,
            GitHubFetchSurface::Api,
            |endpoint| {
                if endpoint.label == "github" {
                    direct_url.clone()
                } else {
                    mirror_url.clone()
                }
            },
            "request failed",
            None,
        )
        .await
        .expect("mirror retry response");
        assert!(response.status().is_success());

        server.join().expect("server join");
        let captured = requests.lock().expect("captured");
        assert!(captured
            .iter()
            .any(|request| request.contains("GET /direct")));
        assert!(captured
            .iter()
            .any(|request| request.contains("GET /mirror")));
    }
}
