use std::collections::{HashMap, HashSet};

use crate::db::{self, DbPool};
use crate::services::central_skills::{self, BatchDeleteCentralSkillRequest};
use crate::services::central_updates::{
    keep_remote_missing_central_skills_impl, normalize_repo_path,
    CentralRepositoryAdditionSkipRequest, CentralRepositoryAdditionUnskipRequest,
    CentralUpdateFailurePhase, CentralUpdatesError,
};
use crate::services::installation::{uninstall_skill, InstallTransport};
use crate::targets::ActiveTarget;

use super::{PlatformDuplicateRemoval, SkillUpdateApplyFailure, SkillUpdateApplyResult};

/// 步骤1：keep_missing 解耦版。复用 `keep_remote_missing_central_skills_impl`，
/// 单元测试用空 app/state 也能跑通。
pub(crate) async fn apply_keep_missing_step(
    pool: &DbPool,
    keep_missing: &[String],
    result: &mut SkillUpdateApplyResult,
) {
    if keep_missing.is_empty() {
        return;
    }
    match keep_remote_missing_central_skills_impl(pool, keep_missing).await {
        Ok(kept) => result.kept_missing_skill_ids = kept,
        Err(_error) => result
            .failures
            .push(SkillUpdateApplyFailure::new("keep_missing", "batch")),
    }
}

/// 步骤2：delete_missing 解耦版。
pub(crate) async fn apply_delete_missing_step(
    pool: &DbPool,
    active_target: &ActiveTarget,
    delete_missing: &[BatchDeleteCentralSkillRequest],
    result: &mut SkillUpdateApplyResult,
) {
    if delete_missing.is_empty() {
        return;
    }
    let delete_outcome = match active_target {
        ActiveTarget::Local => {
            central_skills::delete_central_skills_impl(pool, delete_missing).await
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            central_skills::delete_central_skills_remote_impl(pool, active_target, delete_missing)
                .await
        }
    };
    match delete_outcome {
        Ok(batch) => {
            for ok in batch.succeeded {
                result.deleted_skill_ids.push(ok.skill_id);
            }
            for failure in batch.failed {
                result
                    .failures
                    .push(SkillUpdateApplyFailure::from_central_delete(failure));
            }
        }
        Err(error) => result
            .failures
            .push(SkillUpdateApplyFailure::from_central_delete_error(
                "batch", error,
            )),
    }
}

/// 步骤3a：skip_additions 解耦版。
pub(crate) async fn apply_skip_addition_step(
    pool: &DbPool,
    skip_additions: Vec<CentralRepositoryAdditionSkipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in skip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(_error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "skip_addition",
                    request.repository_id,
                ));
                continue;
            }
        };
        match db::upsert_skill_repository_sync_skip(
            pool,
            &request.repository_id,
            &source_path,
            &request.skill_id,
            &request.skill_name,
        )
        .await
        {
            Ok(saved) => {
                // pending_additions 已落 skip，删除 pending 行
                let _ = db::delete_pending_addition(pool, &saved.repository_id, &saved.source_path)
                    .await;
                result
                    .skipped_additions
                    .push(format!("{}::{}", saved.repository_id, saved.source_path));
            }
            Err(_error) => result.failures.push(SkillUpdateApplyFailure::new(
                "skip_addition",
                request.repository_id,
            )),
        }
    }
}

/// 步骤3b：unskip_additions 解耦版。
pub(crate) async fn apply_unskip_addition_step(
    pool: &DbPool,
    unskip_additions: Vec<CentralRepositoryAdditionUnskipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in unskip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(_error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "unskip_addition",
                    request.repository_id,
                ));
                continue;
            }
        };
        match db::delete_skill_repository_sync_skip(pool, &request.repository_id, &source_path)
            .await
        {
            Ok(_) => result
                .unskipped_additions
                .push(format!("{}::{}", request.repository_id, source_path)),
            Err(_error) => result.failures.push(SkillUpdateApplyFailure::new(
                "unskip_addition",
                request.repository_id,
            )),
        }
    }
}

/// 步骤6：remove_platform_duplicates 解耦版。
pub(crate) async fn apply_remove_platform_duplicates_step(
    pool: &DbPool,
    removals: Vec<PlatformDuplicateRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
) {
    for removal in removals {
        if !is_agent_allowed(&removal.agent_id, allowed_agent_ids) {
            result.failures.push(SkillUpdateApplyFailure::new(
                "remove_platform_duplicate",
                format!("{}::{}", removal.agent_id, removal.skill_id),
            ));
            continue;
        }
        let observations = match db::get_agent_skill_observations(pool, &removal.agent_id).await {
            Ok(rows) => rows,
            Err(_error) => {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "remove_platform_duplicate",
                    format!("{}::{}", removal.agent_id, removal.skill_id),
                ));
                continue;
            }
        };
        let obs_by_path = observations
            .into_iter()
            .filter(|o| o.skill_id == removal.skill_id)
            .map(|o| (o.dir_path.clone(), o))
            .collect::<HashMap<_, _>>();

        for path in removal.paths {
            let row_id = (removal.agent_id == "claude-code")
                .then(|| obs_by_path.get(&path).map(|o| o.row_id.clone()))
                .flatten();
            match uninstall_skill(
                pool,
                &InstallTransport::Local,
                &removal.skill_id,
                &removal.agent_id,
                row_id.as_deref(),
            )
            .await
            {
                Ok(()) => result.removed_platform_duplicate_paths.push(path),
                Err(error) => result
                    .failures
                    .push(SkillUpdateApplyFailure::from_central_error(
                        "remove_platform_duplicate",
                        format!("{}::{}", removal.agent_id, removal.skill_id),
                        CentralUpdateFailurePhase::DecisionApply,
                        CentralUpdatesError::Installation(error),
                    )),
            }
        }
    }
}

pub(super) fn is_agent_allowed(
    agent_id: &str,
    allowed_agent_ids: Option<&HashSet<String>>,
) -> bool {
    allowed_agent_ids.is_none_or(|ids| ids.contains(agent_id))
}
