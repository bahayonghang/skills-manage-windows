use super::*;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteSkillCandidate {
    pub(crate) source_path: String,
    pub(crate) skill_id: String,
    pub(crate) skill_name: String,
    pub(crate) description: Option<String>,
    pub(crate) root_directory: String,
    pub(crate) skill_directory_name: String,
    pub(crate) download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidRemoteSkillCandidate {
    pub(crate) source_path: String,
    pub(crate) reason: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectedGitHubRepoSkills {
    pub(crate) repo: GitHubRepoRef,
    pub(crate) valid_candidates: Vec<RemoteSkillCandidate>,
    pub(crate) invalid_candidates: Vec<InvalidRemoteSkillCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartialGitHubRepoImportFailure {
    pub(crate) source_path: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartialGitHubRepoImportResult {
    pub(crate) repo: GitHubRepoRef,
    pub(crate) imported_skills: Vec<ImportedGitHubSkillSummary>,
    pub(crate) skipped_skills: Vec<String>,
    pub(crate) failed_skills: Vec<PartialGitHubRepoImportFailure>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GitHubRepoSnapshot {
    pub(crate) files: HashMap<String, Vec<u8>>,
}

pub(super) const NO_IMPORTABLE_SKILLS_ERROR: &str = "No importable skills found in this repository. Supported layouts include root SKILL.md, common skill directories such as skills/, .agents/skills/, .claude/skills/, direct repository subpaths, and bounded recursive SKILL.md discovery.";
pub(super) const REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES: i64 = 30;
pub(super) const RECURSIVE_DISCOVERY_MAX_DEPTH: usize = 5;
pub(super) const SKIP_DISCOVERY_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    "target",
    "outputs",
    "test",
    "tests",
    "fixture",
    "fixtures",
    "__pycache__",
];
pub(super) const PRIORITY_SKILL_ROOTS: &[&str] = &[
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

pub(super) static GITHUB_PREVIEW_WORKSPACES: OnceLock<
    Mutex<HashMap<String, GitHubPreviewWorkspace>>,
> = OnceLock::new();
pub(super) static GITHUB_SHARED_CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
pub(super) static GITHUB_HOST_RATE_LIMITERS: OnceLock<
    tokio::sync::Mutex<HashMap<String, tokio::time::Instant>>,
> = OnceLock::new();

pub(super) const DEFAULT_GITHUB_HOST_QPS: f64 = 10.0;

#[derive(Debug, Clone)]
pub(super) struct GitHubPreviewWorkspace {
    pub(super) id: String,
    pub(super) target_id: String,
    pub(super) repo: GitHubRepoRef,
    pub(super) source_path: Option<String>,
    pub(super) remote_workspace_dir: String,
    pub(super) remote_repo_dir: String,
    pub(super) created_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}

impl GitHubPreviewWorkspace {
    pub(super) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        debug_assert!(self.expires_at >= self.created_at);
        self.expires_at <= now
    }

    pub(super) fn matches_source(
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
pub(super) struct ParsedGitHubSource {
    pub(super) owner: String,
    pub(super) repo: String,
    pub(super) branch: Option<String>,
    pub(super) source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GitHubAccessDenialKind {
    RateLimited {
        reset_at: Option<String>,
        remaining: Option<String>,
    },
    AuthenticationOrPermission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GitHubAccessDenial {
    pub(super) kind: GitHubAccessDenialKind,
    pub(super) operation: &'static str,
    pub(super) status: reqwest::StatusCode,
    pub(super) github_message: Option<String>,
    pub(super) used_auth: bool,
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
pub(super) struct GitHubErrorResponse {
    pub(super) message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPatTestResult {
    pub configured: bool,
    pub ok: bool,
    pub status: Option<u16>,
    pub message_key: String,
    pub message: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPatState {
    pub configured: bool,
    pub storage_state: crate::secrets::SecretStorageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GitHubFetchSurface {
    Api,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MirrorAttemptOutcome {
    pub(super) status: Option<reqwest::StatusCode>,
    pub(super) error_message: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GitHubMirrorEndpoint {
    pub(super) label: &'static str,
    pub(super) api_base: &'static str,
    pub(super) raw_base: &'static str,
}

pub(super) const GITHUB_MIRROR_ENDPOINTS: &[GitHubMirrorEndpoint] = &[
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
