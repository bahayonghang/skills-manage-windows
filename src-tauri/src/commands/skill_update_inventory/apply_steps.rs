use std::collections::HashMap;

use crate::commands::central_updates::{
    self, keep_remote_missing_central_skills_impl,
};
use crate::commands::central_updates_fs::normalize_repo_path;
use crate::db::{self, DbPool};
use crate::services::central_skills::{
    self, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult,
};
use crate::services::installation::uninstall_skill_from_agent_with_row_impl;
use crate::targets::ActiveTarget;

use super::{
    PlatformDuplicateRemoval, SkillUpdateApplyFailure, SkillUpdateApplyResult,
};

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
        Err(error) => result.failures.push(SkillUpdateApplyFailure {
            step: "keep_missing".to_string(),
            identifier: keep_missing.join(","),
            error,
        }),
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
    let delete_outcome: Result<BatchDeleteCentralSkillResult, String> = match active_target {
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
                result.failures.push(SkillUpdateApplyFailure {
                    step: "delete_missing".to_string(),
                    identifier: failure.skill_id,
                    error: failure.error,
                });
            }
        }
        Err(error) => result.failures.push(SkillUpdateApplyFailure {
            step: "delete_missing".to_string(),
            identifier: String::new(),
            error,
        }),
    }
}

/// 步骤3a：skip_additions 解耦版。
pub(crate) async fn apply_skip_addition_step(
    pool: &DbPool,
    skip_additions: Vec<central_updates::CentralRepositoryAdditionSkipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in skip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "skip_addition".to_string(),
                    identifier: format!("{}::{}", request.repository_id, request.source_path),
                    error,
                });
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
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "skip_addition".to_string(),
                identifier: format!("{}::{}", request.repository_id, source_path),
                error,
            }),
        }
    }
}

/// 步骤3b：unskip_additions 解耦版。
pub(crate) async fn apply_unskip_addition_step(
    pool: &DbPool,
    unskip_additions: Vec<central_updates::CentralRepositoryAdditionUnskipRequest>,
    result: &mut SkillUpdateApplyResult,
) {
    for request in unskip_additions {
        let source_path = match normalize_repo_path(&request.source_path) {
            Ok(p) => p,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "unskip_addition".to_string(),
                    identifier: format!("{}::{}", request.repository_id, request.source_path),
                    error,
                });
                continue;
            }
        };
        match db::delete_skill_repository_sync_skip(pool, &request.repository_id, &source_path)
            .await
        {
            Ok(_) => result
                .unskipped_additions
                .push(format!("{}::{}", request.repository_id, source_path)),
            Err(error) => result.failures.push(SkillUpdateApplyFailure {
                step: "unskip_addition".to_string(),
                identifier: format!("{}::{}", request.repository_id, source_path),
                error,
            }),
        }
    }
}

/// 步骤6：remove_platform_duplicates 解耦版。
pub(crate) async fn apply_remove_platform_duplicates_step(
    pool: &DbPool,
    removals: Vec<PlatformDuplicateRemoval>,
    result: &mut SkillUpdateApplyResult,
) {
    for removal in removals {
        let observations = match db::get_agent_skill_observations(pool, &removal.agent_id).await {
            Ok(rows) => rows,
            Err(error) => {
                result.failures.push(SkillUpdateApplyFailure {
                    step: "remove_platform_duplicate".to_string(),
                    identifier: format!("{}::{}", removal.agent_id, removal.skill_id),
                    error,
                });
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
            match uninstall_skill_from_agent_with_row_impl(
                pool,
                &removal.skill_id,
                &removal.agent_id,
                row_id.as_deref(),
            )
            .await
            {
                Ok(()) => result.removed_platform_duplicate_paths.push(path),
                Err(error) => result.failures.push(SkillUpdateApplyFailure {
                    step: "remove_platform_duplicate".to_string(),
                    identifier: format!("{}::{}::{}", removal.agent_id, removal.skill_id, path),
                    error,
                }),
            }
        }
    }
}
