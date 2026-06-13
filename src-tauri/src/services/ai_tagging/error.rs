//! Typed errors for the AI tagging domain.
//!
//! Variants cover the real failure categories of tagging-context preparation,
//! the AI tagging HTTP call, suggestion parsing/mapping, and persistence.
//! Display texts intentionally preserve the historical string-error wording
//! (including the `ai.*:` coded prefixes): the IPC boundary stringifies these
//! errors and the frontend shows them in toasts verbatim.
//!
//! Per-skill failures stay as `error: String` fields inside the IPC payload
//! types (`SkillTagSuggestionResult`, `AiTagProgressPayload`); only
//! whole-operation failures use this enum.

use crate::services::ai_provider::AiProviderError;

/// Failure categories for AI tag suggestion runs.
#[derive(Debug, thiserror::Error)]
pub enum AiTaggingError {
    /// Skill/tag reads and suggested-tag persistence flow through
    /// transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Errors propagated from the AI provider domain (API-key resolution,
    /// HTTP client construction).
    #[error(transparent)]
    Provider(#[from] AiProviderError),

    /// HTTP transport/protocol failure of the tagging request (send, body
    /// read, non-2xx response). Coded message preformatted at the call site.
    #[error("{0}")]
    Http(String),

    /// 429 from the provider. Coded "ai.rate_limit:..." message preformatted
    /// at the call site.
    #[error("{0}")]
    RateLimited(String),

    /// Tagging response parse failure (not JSON / missing content / bad
    /// suggestion JSON). Message preformatted at the call site.
    #[error("{0}")]
    Parse(String),

    #[error("No candidate tags are available.")]
    NoCandidateTags,

    #[error("Skill '{0}' not found")]
    SkillNotFound(String),

    #[error("AI tagging returned no usable candidate tags.")]
    NoUsableCandidateTags,

    /// The user cancelled the running tagging job.
    #[error("AI tagging canceled")]
    Cancelled,
}
