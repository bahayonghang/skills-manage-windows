use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;

use chrono::Utc;
use tracing::Instrument;

use crate::db::{self, DbPool, Skill, SkillUpdateState};

use super::state_from_remote;
use crate::services::central_updates::error::CentralUpdatesError;
use crate::services::central_updates::fs::{CentralFs, CentralSkillWrite, CopyRefreshRequest};
use crate::services::central_updates::types::RemoteSkillContent;

#[derive(Debug, Clone)]
pub(crate) struct SkillUpdatePlan {
    pub(crate) skill: Skill,
    pub(crate) remote: RemoteSkillContent,
    pub(crate) refresh_copies: bool,
}

#[derive(Debug)]
pub(crate) struct SkillUpdateBatchOutcome {
    pub(crate) skill_id: String,
    pub(crate) result: Result<SkillUpdateState, CentralUpdatesError>,
}

pub(crate) async fn update_skills_batch(
    pool: &DbPool,
    fs: &CentralFs,
    plans: Vec<SkillUpdatePlan>,
    cancel: Option<&AtomicBool>,
) -> Vec<SkillUpdateBatchOutcome> {
    let _mutation_guard = if matches!(fs, CentralFs::Local) {
        match crate::services::central_mutation::acquire_central_mutation_guard(
            "update Central skills",
            crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
        )
        .await
        {
            Ok(guard) => Some(guard),
            Err(error) => {
                let message = error.to_string();
                return plans
                    .into_iter()
                    .map(|plan| SkillUpdateBatchOutcome {
                        skill_id: plan.skill.id,
                        result: Err(CentralUpdatesError::CentralMutation(message.clone())),
                    })
                    .collect();
            }
        }
    } else {
        None
    };

    let writes = plans
        .iter()
        .map(|plan| CentralSkillWrite {
            skill_id: plan.skill.id.clone(),
            target_dir: plan.remote.target_dir.clone(),
            files: plan.remote.files.clone(),
        })
        .collect();
    let write_outcomes = fs.write_skill_dirs_atomic_cancellable(writes, cancel).await;
    let mut write_results = write_outcomes
        .into_iter()
        .map(|outcome| (outcome.skill_id, outcome.result))
        .collect::<HashMap<_, _>>();
    let mut results = HashMap::<String, Result<SkillUpdateState, CentralUpdatesError>>::new();
    let mut copy_requests = Vec::new();
    let mut seen_copy_targets = HashSet::new();

    let persist_span = tracing::info_span!(
        "central_update_phase",
        phase = "db_persist",
        skills = plans.len()
    );
    async {
        for plan in &plans {
            let write_result = write_results.remove(&plan.skill.id).unwrap_or_else(|| {
                Err(CentralUpdatesError::Batch(format!(
                    "Central write returned no outcome for skill '{}'.",
                    plan.skill.id
                )))
            });
            if let Err(error) = write_result {
                results.insert(plan.skill.id.clone(), Err(error));
                continue;
            }

            if let Err(error) = persist_updated_skill(pool, &plan.skill, &plan.remote).await {
                results.insert(plan.skill.id.clone(), Err(error));
                continue;
            }
            results.insert(
                plan.skill.id.clone(),
                Ok(state_from_remote(&plan.skill, &plan.remote, true)),
            );

            if plan.refresh_copies {
                match copy_refresh_requests(pool, &plan.skill.id, &plan.remote.target_dir).await {
                    Ok(requests) => copy_requests.extend(
                        requests
                            .into_iter()
                            .filter(|request| seen_copy_targets.insert(request.target.clone())),
                    ),
                    Err(error) => {
                        results.insert(plan.skill.id.clone(), Err(error));
                    }
                }
            }
        }
    }
    .instrument(persist_span)
    .await;

    for outcome in fs
        .refresh_copy_installs_cancellable(copy_requests, cancel)
        .await
    {
        debug_assert!(seen_copy_targets.contains(&outcome.target));
        if let Err(error) = outcome.result {
            results.entry(outcome.skill_id).and_modify(|result| {
                if result.is_ok() {
                    *result = Err(error);
                }
            });
        }
    }

    plans
        .into_iter()
        .map(|plan| SkillUpdateBatchOutcome {
            skill_id: plan.skill.id.clone(),
            result: results.remove(&plan.skill.id).unwrap_or_else(|| {
                Err(CentralUpdatesError::Batch(format!(
                    "Central update returned no outcome for skill '{}'.",
                    plan.skill.id
                )))
            }),
        })
        .collect()
}

async fn persist_updated_skill(
    pool: &DbPool,
    skill: &Skill,
    remote: &RemoteSkillContent,
) -> Result<(), CentralUpdatesError> {
    let skill_md_path = remote.target_dir.join("SKILL.md");
    let updated_skill = Skill {
        id: skill.id.clone(),
        uid: skill.uid.clone(),
        name: remote.candidate.skill_name.clone(),
        description: remote.candidate.description.clone(),
        file_path: skill_md_path.to_string_lossy().into_owned(),
        canonical_path: Some(remote.target_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some(format!(
            "github:{}/{}",
            remote.source.repo.owner, remote.source.repo.repo
        )),
        content: skill.content.clone(),
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(pool, &updated_skill).await?;
    db::assign_github_repository_to_skill(
        pool,
        &remote.source.repo.owner,
        &remote.source.repo.repo,
        &remote.source.repo.branch,
        &remote.source.repo.normalized_url,
        &skill.id,
        &remote.source.source_path,
    )
    .await?;
    Ok(())
}

async fn copy_refresh_requests(
    pool: &DbPool,
    skill_id: &str,
    source_dir: &Path,
) -> Result<Vec<CopyRefreshRequest>, CentralUpdatesError> {
    let installations = db::get_skill_installations(pool, skill_id).await?;
    let mut seen_targets = HashSet::new();
    Ok(installations
        .into_iter()
        .filter(|installation| installation.link_type == "copy")
        .filter_map(|installation| {
            if seen_targets.insert(installation.installed_path.clone()) {
                Some(CopyRefreshRequest {
                    skill_id: skill_id.to_string(),
                    source_dir: source_dir.to_path_buf(),
                    target: installation.installed_path,
                })
            } else {
                None
            }
        })
        .collect())
}
