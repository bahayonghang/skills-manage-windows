use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use crate::db::{self, DbPool};
use crate::operation_log::{
    local_target_context, record_operation_log_best_effort, OperationLogEvent,
};
use crate::secrets::{SecretStore, SystemSecretStore};
use crate::services::{central_skills, github_import, installation, marketplace};
use crate::targets::ActiveTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    SkillsSh { source: String, skill_id: String },
    GitHubUrl(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CliApiError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Ambiguous(String),
    #[error("{0}")]
    Duplicate(String),
    #[error("{0}")]
    Busy(String),
    #[error("{0}")]
    Internal(String),
}

impl CliApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "input.invalid",
            Self::NotFound(_) => "skill.not_found",
            Self::Ambiguous(_) => "skill.ambiguous",
            Self::Duplicate(_) => "skill.duplicate",
            Self::Busy(_) => "mutation.busy",
            Self::Internal(_) => "internal.error",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidInput(_) => 2,
            Self::NotFound(_) | Self::Ambiguous(_) | Self::Duplicate(_) => 3,
            Self::Busy(_) => 4,
            Self::Internal(_) => 1,
        }
    }
}

impl From<sqlx::Error> for CliApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<central_skills::CentralSkillsError> for CliApiError {
    fn from(error: central_skills::CentralSkillsError) -> Self {
        match error {
            central_skills::CentralSkillsError::SkillNotFound(reference) => {
                Self::NotFound(format!("Skill '{reference}' not found"))
            }
            central_skills::CentralSkillsError::AmbiguousSkillReference(reference) => {
                Self::Ambiguous(format!("Multiple skills are named '{reference}'; use uid or slug"))
            }
            central_skills::CentralSkillsError::CentralMutation(error) => mutation_error(error),
            other => Self::Internal(other.to_string()),
        }
    }
}

fn mutation_error(error: crate::services::central_mutation::CentralMutationError) -> CliApiError {
    use crate::services::central_mutation::CentralMutationError;
    match error {
        CentralMutationError::Busy { .. } | CentralMutationError::Timeout { .. } => {
            CliApiError::Busy(error.to_string())
        }
        other => CliApiError::Internal(other.to_string()),
    }
}

fn github_error(error: github_import::GithubImportError) -> CliApiError {
    match error {
        github_import::GithubImportError::CentralMutation(error) => mutation_error(error),
        github_import::GithubImportError::InvalidUrl(message) => {
            CliApiError::InvalidInput(message)
        }
        github_import::GithubImportError::InvalidRepoUrl
        | github_import::GithubImportError::RepoUrlNotHttps
        | github_import::GithubImportError::RepoUrlNotGithub
        | github_import::GithubImportError::RepoUrlMissingOwner
        | github_import::GithubImportError::RepoUrlMissingRepo
        | github_import::GithubImportError::RepoUrlMissingOwnerRepo
        | github_import::GithubImportError::TreeUrlMissingBranch
        | github_import::GithubImportError::BlobUrlUnsupported => {
            CliApiError::InvalidInput(error.to_string())
        }
        github_import::GithubImportError::RepoNotFound
        | github_import::GithubImportError::NoImportableSkills => {
            CliApiError::NotFound(error.to_string())
        }
        github_import::GithubImportError::TargetDirExists(_) => {
            CliApiError::Duplicate(error.to_string())
        }
        other => CliApiError::Internal(other.to_string()),
    }
}

fn marketplace_error(error: marketplace::MarketplaceError) -> CliApiError {
    match error {
        marketplace::MarketplaceError::DuplicateRequiresReplace(skill) => {
            CliApiError::duplicate(skill)
        }
        marketplace::MarketplaceError::GithubImport(error) => github_error(error),
        marketplace::MarketplaceError::SkillsShSourceInvalid
        | marketplace::MarketplaceError::SkillsShSkillIdUnsupported => {
            CliApiError::InvalidInput(error.to_string())
        }
        marketplace::MarketplaceError::SkillsShCandidateNotFound { .. }
        | marketplace::MarketplaceError::SkillNotImported(_) => {
            CliApiError::NotFound(error.to_string())
        }
        other => CliApiError::Internal(other.to_string()),
    }
}

impl CliApiError {
    fn duplicate(skill: String) -> Self {
        Self::Duplicate(format!("Skill '{skill}' already exists; pass --replace to overwrite it"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSkill {
    pub uid: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub canonical_path: Option<String>,
    pub linked_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlanItem {
    pub uid: String,
    pub id: String,
    pub agent_id: String,
    pub target_path: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutput {
    pub dry_run: bool,
    pub plans: Vec<SyncPlanItem>,
    pub result: Option<installation::CentralBatchInstallResult>,
}

impl SyncOutput {
    pub fn is_partial_failure(&self) -> bool {
        self.result
            .as_ref()
            .is_some_and(|result| !result.failed.is_empty())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFailure {
    pub source_path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutput {
    pub imported_skill_ids: Vec<String>,
    pub failed: Vec<InstallFailure>,
    pub sync: Option<SyncOutput>,
}

impl InstallOutput {
    pub fn is_partial_failure(&self) -> bool {
        !self.failed.is_empty()
            || self
                .sync
                .as_ref()
                .is_some_and(SyncOutput::is_partial_failure)
    }
}

pub struct CliContext {
    db: DbPool,
    secrets: Arc<dyn SecretStore>,
    target: ActiveTarget,
}

impl CliContext {
    pub async fn open_default() -> Result<Self, CliApiError> {
        let app_dir = crate::paths::app_data_dir();
        std::fs::create_dir_all(&app_dir).map_err(|error| CliApiError::Internal(error.to_string()))?;
        let db_path = crate::paths::path_to_string(&app_dir.join("db.sqlite"));
        let db = db::create_pool(&db_path).await?;
        db::init_database(&db).await?;
        Ok(Self {
            db,
            secrets: Arc::new(SystemSecretStore::default()),
            target: ActiveTarget::Local,
        })
    }

    pub fn new(db: DbPool, secrets: Arc<dyn SecretStore>) -> Self {
        Self {
            db,
            secrets,
            target: ActiveTarget::Local,
        }
    }

    pub async fn list_skills(&self) -> Result<Vec<CliSkill>, CliApiError> {
        Ok(central_skills::get_central_skills_impl(&self.db)
            .await?
            .into_iter()
            .map(|skill| CliSkill {
                uid: skill.uid,
                id: skill.id,
                name: skill.name,
                description: skill.description,
                canonical_path: skill.canonical_path,
                linked_agents: skill.linked_agents,
            })
            .collect())
    }

    pub async fn show_skill(&self, reference: &str) -> Result<CliSkill, CliApiError> {
        let skill = central_skills::resolve_skill_ref_impl(&self.db, reference).await?;
        let linked_agents = db::get_skill_installations(&self.db, &skill.id)
            .await?
            .into_iter()
            .map(|installation| installation.agent_id)
            .collect();
        Ok(CliSkill {
            uid: skill.uid,
            id: skill.id,
            name: skill.name,
            description: skill.description,
            canonical_path: skill.canonical_path,
            linked_agents,
        })
    }

    pub async fn search_skills(
        &self,
        query: String,
        limit: Option<u32>,
    ) -> Result<Vec<marketplace::SkillsShSkill>, CliApiError> {
        marketplace::search_skills_sh_impl(&self.db, self.secrets.as_ref(), query, limit)
            .await
            .map_err(marketplace_error)
    }

    pub async fn install_skill(
        &self,
        source: &str,
        replace: bool,
        yes: bool,
        sync: bool,
        agent_ids: Vec<String>,
        method: &str,
    ) -> Result<InstallOutput, CliApiError> {
        validate_method(method)?;
        let parsed = parse_install_source(source)?;
        let (imported_skill_ids, failed) = match parsed {
            InstallSource::SkillsSh { source, skill_id } => {
                let imported = marketplace::install_from_skills_sh_with_options_impl(
                    &self.db,
                    self.secrets.as_ref(),
                    ActiveTarget::Local,
                    source,
                    skill_id,
                    replace,
                )
                .await
                .map_err(marketplace_error)?;
                (vec![imported], Vec::new())
            }
            InstallSource::GitHubUrl(url) => {
                let auth = github_import::github_direct_auth_from_secret_store(
                    &self.db,
                    self.secrets.as_ref(),
                )
                .await
                .map_err(github_error)?;
                let preview = github_import::preview_github_repo_import_with_auth(
                    &self.db,
                    &url,
                    auth.as_deref(),
                )
                .await
                .map_err(github_error)?;
                let conflicts = preview
                    .skills
                    .iter()
                    .filter(|skill| skill.conflict.is_some())
                    .collect::<Vec<_>>();
                if !conflicts.is_empty() && !replace {
                    return Err(CliApiError::Duplicate(format!(
                        "{} skill(s) already exist; pass --replace to overwrite",
                        conflicts.len()
                    )));
                }
                if replace && preview.skills.len() > 1 && !yes {
                    return Err(CliApiError::InvalidInput(
                        "Replacing multiple skills requires --yes".to_string(),
                    ));
                }
                let selections = preview
                    .skills
                    .iter()
                    .map(|skill| github_import::GitHubSkillImportSelection {
                        source_path: skill.source_path.clone(),
                        resolution: github_import::DuplicateResolution::Overwrite,
                        renamed_skill_id: None,
                    })
                    .collect();
                let result = github_import::import_github_repo_skills_partially_with_auth(
                    &self.db,
                    &url,
                    selections,
                    None,
                    auth.as_deref(),
                )
                .await
                .map_err(github_error)?;
                (
                    result
                        .imported_skills
                        .into_iter()
                        .map(|skill| skill.imported_skill_id)
                        .collect(),
                    result
                        .failed_skills
                        .into_iter()
                        .map(|failure| InstallFailure {
                            source_path: failure.source_path,
                            error: failure.error,
                        })
                        .collect(),
                )
            }
        };

        let sync_output = if sync && !imported_skill_ids.is_empty() {
            Some(
                self.sync_skills(imported_skill_ids.clone(), false, agent_ids, method, false)
                    .await?,
            )
        } else {
            None
        };
        let output = InstallOutput {
            imported_skill_ids,
            failed,
            sync: sync_output,
        };
        let status = if output.is_partial_failure() {
            "partial"
        } else {
            "succeeded"
        };
        record_operation_log_best_effort(
            &self.db,
            local_target_context(),
            OperationLogEvent::new("cli", "skills_install", status, "CLI skill install")
                .details(serde_json::json!({
                    "source": "cli",
                    "importedCount": output.imported_skill_ids.len(),
                    "failedCount": output.failed.len(),
                    "syncRequested": sync,
                })),
        )
        .await;
        Ok(output)
    }

    pub async fn sync_skills(
        &self,
        references: Vec<String>,
        all: bool,
        agent_ids: Vec<String>,
        method: &str,
        dry_run: bool,
    ) -> Result<SyncOutput, CliApiError> {
        validate_method(method)?;
        if all && !references.is_empty() {
            return Err(CliApiError::InvalidInput(
                "Use explicit skill refs or --all, not both".to_string(),
            ));
        }
        if !all && references.is_empty() {
            return Err(CliApiError::InvalidInput(
                "skills sync requires refs or --all".to_string(),
            ));
        }

        let skills = if all {
            db::get_central_skills(&self.db).await?
        } else {
            let mut seen = HashSet::new();
            let mut skills = Vec::new();
            for reference in references {
                let skill = central_skills::resolve_skill_ref_impl(&self.db, &reference).await?;
                if seen.insert(skill.id.clone()) {
                    skills.push(skill);
                }
            }
            skills
        };
        let agents = selected_agents(&self.db, agent_ids).await?;
        let plans = skills
            .iter()
            .flat_map(|skill| {
                agents.iter().map(move |agent| SyncPlanItem {
                    uid: skill.uid.clone(),
                    id: skill.id.clone(),
                    agent_id: agent.id.clone(),
                    target_path: PathBuf::from(&agent.global_skills_dir)
                        .join(&skill.id)
                        .to_string_lossy()
                        .into_owned(),
                    method: method.to_string(),
                })
            })
            .collect::<Vec<_>>();
        if dry_run {
            return Ok(SyncOutput {
                dry_run: true,
                plans,
                result: None,
            });
        }

        let transport = installation::InstallTransport::for_target(&self.target)
            .await
            .map_err(|error| CliApiError::Internal(error.to_string()))?;
        let result = installation::batch_install_central_skills_impl(
            &self.db,
            &transport,
            skills.into_iter().map(|skill| skill.id).collect(),
            agents.into_iter().map(|agent| agent.id).collect(),
            method,
            None,
        )
        .await;
        let output = SyncOutput {
            dry_run: false,
            plans,
            result: Some(result),
        };
        let result = output.result.as_ref().expect("sync result");
        let status = match (result.succeeded.len() + result.skipped.len(), result.failed.len()) {
            (_, 0) => "succeeded",
            (0, _) => "failed",
            _ => "partial",
        };
        record_operation_log_best_effort(
            &self.db,
            local_target_context(),
            OperationLogEvent::new("cli", "skills_sync", status, "CLI skill sync").details(
                serde_json::json!({
                    "source": "cli",
                    "planCount": output.plans.len(),
                    "succeededCount": result.succeeded.len(),
                    "skippedCount": result.skipped.len(),
                    "failedCount": result.failed.len(),
                }),
            ),
        )
        .await;
        Ok(output)
    }
}

pub fn parse_install_source(value: &str) -> Result<InstallSource, CliApiError> {
    let value = value.trim();
    if let Ok(url) = reqwest::Url::parse(value) {
        if matches!(url.scheme(), "http" | "https") && url.host_str() == Some("github.com") {
            return Ok(InstallSource::GitHubUrl(value.to_string()));
        }
        return Err(CliApiError::InvalidInput(
            "Only github.com repository URLs are supported".to_string(),
        ));
    }

    let (source, skill_id) = value.split_once('@').ok_or_else(|| {
        CliApiError::InvalidInput(
            "Install source must be owner/repo@skill or a GitHub URL".to_string(),
        )
    })?;
    let mut parts = source.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() || skill_id.is_empty() {
        return Err(CliApiError::InvalidInput(
            "Install source must be owner/repo@skill".to_string(),
        ));
    }
    Ok(InstallSource::SkillsSh {
        source: format!("{owner}/{repo}"),
        skill_id: skill_id.to_string(),
    })
}

fn validate_method(method: &str) -> Result<(), CliApiError> {
    if matches!(method, "auto" | "symlink" | "copy") {
        Ok(())
    } else {
        Err(CliApiError::InvalidInput(
            "method must be auto, symlink, or copy".to_string(),
        ))
    }
}

async fn selected_agents(
    pool: &DbPool,
    requested: Vec<String>,
) -> Result<Vec<db::Agent>, CliApiError> {
    let all = db::get_all_agents(pool).await?;
    if requested.is_empty() {
        return Ok(all
            .into_iter()
            .filter(|agent| agent.id != "central" && agent.is_enabled)
            .collect());
    }
    let requested = requested.into_iter().collect::<HashSet<_>>();
    let selected = all
        .into_iter()
        .filter(|agent| requested.contains(&agent.id) && agent.id != "central" && agent.is_enabled)
        .collect::<Vec<_>>();
    if selected.len() != requested.len() {
        return Err(CliApiError::InvalidInput(
            "One or more requested agents are missing, disabled, or Central".to_string(),
        ));
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{mem_pool, seed_central_skill};
    use std::collections::HashMap;

    #[test]
    fn source_classification_is_deterministic() {
        assert_eq!(
            parse_install_source("vercel-labs/agent-skills@react-best-practices").unwrap(),
            InstallSource::SkillsSh {
                source: "vercel-labs/agent-skills".to_string(),
                skill_id: "react-best-practices".to_string(),
            }
        );
        assert_eq!(
            parse_install_source("https://github.com/openai/skills/tree/main/skills/docs").unwrap(),
            InstallSource::GitHubUrl(
                "https://github.com/openai/skills/tree/main/skills/docs".to_string()
            )
        );
        assert!(parse_install_source("./local-skill").is_err());
        assert!(parse_install_source("https://example.com/repo").is_err());
    }

    #[tokio::test]
    async fn list_show_and_dry_run_sync_share_stable_identity() {
        let pool = mem_pool().await;
        let temp = tempfile::tempdir().unwrap();
        let skill_dir = temp.path().join("demo");
        seed_central_skill(&pool, &skill_dir, "demo", "Demo").await;
        let mut skill = db::get_skill_by_id(&pool, "demo").await.unwrap().unwrap();
        skill.name = "Demo".to_string();
        db::upsert_skill(&pool, &skill).await.unwrap();
        let context = CliContext::new(
            pool,
            Arc::new(crate::secrets::MockSecretStore::default()),
        );

        let listed = context.list_skills().await.unwrap();
        assert_eq!(listed[0].uid, skill.uid);
        assert_eq!(context.show_skill(&skill.uid).await.unwrap().id, "demo");
        assert_eq!(context.show_skill("Demo").await.unwrap().uid, skill.uid);

        let plan = context
            .sync_skills(vec![skill.uid], false, vec!["codex".to_string()], "copy", true)
            .await
            .unwrap();
        assert!(plan.dry_run);
        assert_eq!(plan.plans.len(), 1);
        assert_eq!(plan.plans[0].id, "demo");
        assert!(plan.result.is_none());
    }

    #[tokio::test]
    async fn exact_shorthand_fixture_runs_import_list_show_and_sync_preview() {
        let parsed = parse_install_source(
            "vercel-labs/agent-skills@react-best-practices",
        )
        .unwrap();
        let InstallSource::SkillsSh { source, skill_id } = parsed else {
            panic!("expected skills.sh shorthand");
        };
        assert_eq!(source, "vercel-labs/agent-skills");

        let pool = mem_pool().await;
        let temp = tempfile::tempdir().unwrap();
        let central_root = temp.path().join("central");
        crate::test_support::set_agent_dir(&pool, "central", &central_root).await;
        std::fs::create_dir_all(&central_root).unwrap();
        let repo = github_import::GitHubRepoRef {
            owner: "vercel-labs".to_string(),
            repo: "agent-skills".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/vercel-labs/agent-skills".to_string(),
        };
        let snapshot = github_import::GitHubRepoSnapshot {
            files: HashMap::from([(
                "skills/react-best-practices/SKILL.md".to_string(),
                b"---\nname: React Best Practices\ndescription: React guidance\n---\n# React\n"
                    .to_vec(),
            )]),
        };
        let candidate = marketplace::resolve_skills_sh_candidate_from_snapshot(
            &repo,
            &snapshot,
            &skill_id,
        )
        .unwrap();
        let inspected = github_import::InspectedGitHubRepoSkills {
            repo: repo.clone(),
            valid_candidates: vec![candidate.clone()],
            invalid_candidates: Vec::new(),
        };
        let imported = github_import::import_github_repo_skills_from_snapshot_partially(
            &pool,
            &repo,
            &snapshot,
            inspected,
            vec![github_import::GitHubSkillImportSelection {
                source_path: candidate.source_path.clone(),
                resolution: github_import::DuplicateResolution::Overwrite,
                renamed_skill_id: None,
            }],
            &central_root,
            None,
        )
        .await
        .unwrap();
        assert_eq!(imported.imported_skills.len(), 1);

        let preview = github_import::build_preview_skills(&pool, &[candidate])
            .await
            .unwrap();
        assert!(preview[0].conflict.is_some());

        let context = CliContext::new(
            pool,
            Arc::new(crate::secrets::MockSecretStore::default()),
        );
        let listed = context.list_skills().await.unwrap();
        assert_eq!(listed.len(), 1);
        let shown = context.show_skill(&listed[0].uid).await.unwrap();
        assert_eq!(shown.id, "react-best-practices");
        let sync = context
            .sync_skills(
                vec![shown.uid],
                false,
                vec!["codex".to_string()],
                "copy",
                true,
            )
            .await
            .unwrap();
        assert_eq!(sync.plans.len(), 1);
        assert!(sync.result.is_none());
    }

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(CliApiError::InvalidInput("x".into()).exit_code(), 2);
        assert_eq!(CliApiError::NotFound("x".into()).exit_code(), 3);
        assert_eq!(CliApiError::Busy("x".into()).exit_code(), 4);
        assert_eq!(CliApiError::Internal("x".into()).exit_code(), 1);
    }
}
