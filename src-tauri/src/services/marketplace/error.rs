//! Typed errors for the marketplace domain.
//!
//! Variants cover the real failure categories of registry CRUD, GitHub
//! registry sync, skills.sh search/browse, and remote skill installation.
//! Display texts intentionally preserve the historical string-error wording:
//! the IPC boundary stringifies these errors and the frontend shows them in
//! toasts verbatim.
//!
//! HTTP failures follow the parent design's Http-variant convention
//! (`Http` / `Parse` carry preformatted messages); GitHub-flow failures
//! propagate transparently from the github_import domain.

use crate::services::github_import::GithubImportError;

/// Failure categories for marketplace operations.
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    /// IO failure with an operation-context prefix.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Direct sqlx calls (registry/skill cache queries) flow through
    /// transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Errors propagated from the GitHub import domain (source resolution,
    /// snapshot download, candidate classification, PAT access).
    #[error(transparent)]
    GithubImport(#[from] GithubImportError),

    /// HTTP transport/protocol failure (skills.sh search, skill download).
    /// Message preformatted at the call site.
    #[error("{0}")]
    Http(String),

    /// Response-body parse failure (skills.sh search payload).
    #[error("{0}")]
    Parse(String),

    /// Remote-target transport failures (connect / mkdir / write over the
    /// SSH or WSL channel; targets module returns String).
    #[error("{0}")]
    Remote(String),

    #[error("Cannot remove built-in registry")]
    BuiltinRegistryRemoval,

    #[error("Registry not found")]
    RegistryNotFound,

    #[error("Unsupported source type: {0}")]
    UnsupportedSourceType(String),

    #[error("Skill not found")]
    SkillNotFound,

    #[error("Central agent not found in database")]
    CentralAgentMissing,

    #[error("skills.sh source must be a GitHub owner/repo value.")]
    SkillsShSourceInvalid,

    #[error("skills.sh skill id is not supported.")]
    SkillsShSkillIdUnsupported,

    #[error("skills.sh file path is required.")]
    SkillsShFilePathRequired,

    #[error("Repository path '{0}' is not supported.")]
    UnsupportedRepoPath(String),

    #[error("Could not find SKILL.md for '{skill_id}' in {owner}/{repo}")]
    SkillsShCandidateNotFound {
        skill_id: String,
        owner: String,
        repo: String,
    },

    #[error("Skill '{0}' was not imported.")]
    SkillNotImported(String),

    #[error("Skill '{0}' already exists; pass --replace to overwrite it.")]
    DuplicateRequiresReplace(String),
}

impl MarketplaceError {
    /// Build a [`MarketplaceError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}
