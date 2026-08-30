//! Successful-check status precedence for Skills CLI updates.

use super::SkillsCliUpdateStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsCliPersistedUpdateStatus {
    NotChecked,
    Current,
    UpdateAvailable,
    LocalModified,
    BaselineRequired,
    Unsupported,
    RateLimited,
    Failed,
}

impl SkillsCliPersistedUpdateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotChecked => "not_checked",
            Self::Current => "current",
            Self::UpdateAvailable => "update_available",
            Self::LocalModified => "local_modified",
            Self::BaselineRequired => "baseline_required",
            Self::Unsupported => "unsupported",
            Self::RateLimited => "rate_limited",
            Self::Failed => "failed",
        }
    }

    pub fn from_persisted(value: &str) -> Self {
        match value {
            "current" => Self::Current,
            "update_available" => Self::UpdateAvailable,
            "local_modified" => Self::LocalModified,
            "baseline_required" => Self::BaselineRequired,
            "unsupported" => Self::Unsupported,
            "rate_limited" => Self::RateLimited,
            "failed" => Self::Failed,
            _ => Self::NotChecked,
        }
    }

    pub fn to_public(self) -> SkillsCliUpdateStatus {
        match self {
            Self::NotChecked => SkillsCliUpdateStatus::NotChecked,
            Self::Current => SkillsCliUpdateStatus::Current,
            Self::UpdateAvailable => SkillsCliUpdateStatus::UpdateAvailable,
            Self::LocalModified => SkillsCliUpdateStatus::LocalModified,
            Self::BaselineRequired => SkillsCliUpdateStatus::BaselineRequired,
            Self::Unsupported => SkillsCliUpdateStatus::Unsupported,
            Self::RateLimited => SkillsCliUpdateStatus::RateLimited,
            Self::Failed => SkillsCliUpdateStatus::Failed,
        }
    }
}

pub struct CheckClassification {
    pub status: SkillsCliPersistedUpdateStatus,
    pub pending_revision_sha: Option<String>,
    pub pending_upstream_digest: Option<String>,
    pub clear_pending: bool,
}

pub fn classify_successful_check(
    source_or_path_changed: bool,
    installed_revision_sha: Option<&str>,
    installed_upstream_digest: Option<&str>,
    installed_local_digest: Option<&str>,
    local_digest: Option<&str>,
    observed_revision_sha: &str,
    observed_upstream_digest: &str,
) -> CheckClassification {
    if source_or_path_changed
        || installed_revision_sha.is_none()
        || installed_upstream_digest.is_none()
        || installed_local_digest.is_none()
    {
        return CheckClassification {
            status: SkillsCliPersistedUpdateStatus::BaselineRequired,
            pending_revision_sha: Some(observed_revision_sha.to_string()),
            pending_upstream_digest: Some(observed_upstream_digest.to_string()),
            clear_pending: false,
        };
    }
    if local_digest != installed_local_digest {
        return CheckClassification {
            status: SkillsCliPersistedUpdateStatus::LocalModified,
            pending_revision_sha: Some(observed_revision_sha.to_string()),
            pending_upstream_digest: Some(observed_upstream_digest.to_string()),
            clear_pending: false,
        };
    }
    if installed_revision_sha != Some(observed_revision_sha)
        || installed_upstream_digest != Some(observed_upstream_digest)
    {
        return CheckClassification {
            status: SkillsCliPersistedUpdateStatus::UpdateAvailable,
            pending_revision_sha: Some(observed_revision_sha.to_string()),
            pending_upstream_digest: Some(observed_upstream_digest.to_string()),
            clear_pending: false,
        };
    }
    CheckClassification {
        status: SkillsCliPersistedUpdateStatus::Current,
        pending_revision_sha: None,
        pending_upstream_digest: None,
        clear_pending: true,
    }
}
