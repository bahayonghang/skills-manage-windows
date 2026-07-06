//! Operation-level transport seam: Local vs Remote (SSH/WSL) adapters for
//! skill install / uninstall.
//!
//! Commands resolve the transport once via [`InstallTransport::for_target`]
//! and never branch on `ActiveTarget` again; the business orchestration in
//! `install.rs` is written once and calls the per-transport hooks below.
//! Each hook preserves the historical semantics of its side verbatim —
//! including the deliberate asymmetries (local-only skip detection and auto
//! fallback; the remote side's single atomic install script).

use std::path::PathBuf;

use crate::db::{self, DbPool};
use crate::targets::{connect_remote_target, ActiveTarget, ConnectedRemoteTarget, TargetsError};

use super::error::InstallationError;
use super::native;
use super::remote;
use super::types::SkippedInstall;

/// Single stringify boundary for transport-layer errors entering the
/// installation domain.
pub(crate) fn transport_error(error: TargetsError) -> InstallationError {
    InstallationError::Remote(error.to_string())
}

pub enum InstallTransport {
    Local,
    Remote(Box<ConnectedRemoteTarget>),
}

/// Install method after per-transport normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedMethod {
    Symlink,
    Copy,
    /// Local only: try symlink, fall back to copy when the platform rejects
    /// symlink creation (Windows privileges / non-NTFS).
    Auto,
}

/// Per-transport source-skill context resolved before the shared-root branch.
/// The remote install script centralizes from the source directory
/// server-side, so the skill row must resolve up front; the local side reads
/// it lazily inside `ensure_centralized`.
pub(crate) enum SourceContext {
    Local,
    Remote {
        skill: Box<db::Skill>,
        source_dir: String,
    },
}

/// What `place_install` laid down, in DB-record vocabulary.
pub(crate) struct Placement {
    pub installed_path: String,
    pub link_type: &'static str,
    pub symlink_target: Option<String>,
}

impl InstallTransport {
    /// Resolve the transport for the active target. `Local` is free; SSH/WSL
    /// opens one connection that batch callers reuse across the whole loop.
    pub async fn for_target(target: &ActiveTarget) -> Result<Self, InstallationError> {
        match target {
            ActiveTarget::Local => Ok(Self::Local),
            ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => Ok(Self::Remote(Box::new(
                connect_remote_target(target).await.map_err(transport_error)?,
            ))),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }

    pub(crate) fn shares_central_root(&self, agent: &db::Agent, central: &db::Agent) -> bool {
        match self {
            // Local compares resolved filesystem identity; remote agent rows
            // hold POSIX strings and have always been compared literally.
            Self::Local => super::centralize::agents_share_skills_dir(agent, central),
            Self::Remote(_) => agent.global_skills_dir == central.global_skills_dir,
        }
    }

    pub(crate) fn resolve_method(&self, method: &str) -> Result<ResolvedMethod, InstallationError> {
        match self {
            Self::Local => Ok(match method {
                "copy" => ResolvedMethod::Copy,
                "symlink" => ResolvedMethod::Symlink,
                _ => ResolvedMethod::Auto,
            }),
            Self::Remote(connection) => {
                if method == "symlink" {
                    if !connection.symlink_allowed() {
                        return Err(InstallationError::RemoteSymlinkDisabled);
                    }
                    Ok(ResolvedMethod::Symlink)
                } else {
                    Ok(ResolvedMethod::Copy)
                }
            }
        }
    }

    pub(crate) async fn validate_source(
        &self,
        pool: &DbPool,
        skill_id: &str,
    ) -> Result<SourceContext, InstallationError> {
        match self {
            Self::Local => Ok(SourceContext::Local),
            Self::Remote(_) => {
                let skill = db::get_skill_by_id(pool, skill_id)
                    .await?
                    .ok_or_else(|| InstallationError::SkillNotFound(skill_id.to_string()))?;
                let source_dir = remote::remote_skill_source_dir(&skill, skill_id)?;
                Ok(SourceContext::Remote {
                    skill: Box::new(skill),
                    source_dir,
                })
            }
        }
    }

    /// Shared-root branch: centralize the skill and return the canonical
    /// directory (which doubles as the installed path for a native record).
    pub(crate) async fn centralize_shared_root(
        &self,
        pool: &DbPool,
        skill_id: &str,
        central: &db::Agent,
    ) -> Result<String, InstallationError> {
        match self {
            Self::Local => {
                native::centralize_shared_root_local(pool, skill_id, central).await
            }
            Self::Remote(connection) => {
                let canonical_dir =
                    crate::targets::remote_join(&central.global_skills_dir, skill_id);
                remote::ensure_remote_centralized(connection, pool, skill_id, &canonical_dir)
                    .await?;
                Ok(canonical_dir)
            }
        }
    }

    /// Pre-placement work. Local centralizes eagerly and creates the agent
    /// skills directory; the remote script does both server-side in one
    /// round trip, so remote is a no-op here.
    pub(crate) async fn prepare_target(
        &self,
        pool: &DbPool,
        skill_id: &str,
        agent: &db::Agent,
        central: &db::Agent,
    ) -> Result<(), InstallationError> {
        match self {
            Self::Local => native::prepare_target_local(pool, skill_id, agent, central).await,
            Self::Remote(_) => Ok(()),
        }
    }

    /// Same-origin "already installed" detection. Remote has no skip
    /// detection — the install script replaces managed entries.
    pub(crate) async fn detect_existing(
        &self,
        pool: &DbPool,
        skill_id: &str,
        agent_id: &str,
        agent: &db::Agent,
        central: &db::Agent,
    ) -> Result<Option<SkippedInstall>, InstallationError> {
        match self {
            Self::Local => {
                let target_path = PathBuf::from(&agent.global_skills_dir).join(skill_id);
                let canonical_dir = PathBuf::from(&central.global_skills_dir).join(skill_id);
                super::skip::detect_existing_agent_install(
                    pool,
                    skill_id,
                    agent_id,
                    &target_path,
                    &canonical_dir,
                )
                .await
            }
            Self::Remote(_) => Ok(None),
        }
    }

    pub(crate) async fn place_install(
        &self,
        pool: &DbPool,
        source: SourceContext,
        agent: &db::Agent,
        central: &db::Agent,
        skill_id: &str,
        method: ResolvedMethod,
    ) -> Result<Placement, InstallationError> {
        match self {
            Self::Local => native::place_install_local(agent, central, skill_id, method).await,
            Self::Remote(connection) => {
                remote::place_install_remote(
                    pool, connection, source, agent, central, skill_id, method,
                )
                .await
            }
        }
    }

    /// Remove an installed entry. Local classifies by recorded link type and
    /// refuses to delete unmanaged real directories; remote has always been
    /// an unconditional `rm -rf` of the install slot.
    pub(crate) async fn remove_install(
        &self,
        pool: &DbPool,
        agent: &db::Agent,
        skill_id: &str,
    ) -> Result<(), InstallationError> {
        match self {
            Self::Local => native::remove_install_local(pool, agent, skill_id).await,
            Self::Remote(connection) => {
                let install_path =
                    crate::targets::remote_join(&agent.global_skills_dir, skill_id);
                connection
                    .remove_tree(&install_path)
                    .await
                    .map_err(transport_error)
            }
        }
    }
}
