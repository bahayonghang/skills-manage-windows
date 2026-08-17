use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::db::{self, Agent, DbPool};
use crate::services::central_updates::{CentralUpdateFailurePhase, CentralUpdatesError};
use crate::services::installation::{uninstall_skill, InstallTransport};
use crate::targets::{connect_remote_target, ActiveTarget, ConnectedRemoteTarget};

use super::apply_steps::is_agent_allowed;
use super::{DeletedPlatformCopyRemoval, SkillUpdateApplyFailure, SkillUpdateApplyResult};

pub(crate) const REMOTE_LEFTOVER_DELETE_CHUNK_SIZE: usize = 256;

pub(crate) const REMOTE_LEFTOVER_DELETE_SCRIPT: &str = r#"set -u
index=0
for path in "$@"; do
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    printf 'MISSING\t%d\n' "$index"
  elif rm -rf -- "$path"; then
    printf 'OK\t%d\n' "$index"
  else
    printf 'ERR\t%d\n' "$index"
  fi
  index=$((index + 1))
done
"#;

#[derive(Clone, Debug)]
struct RemoteLeftoverRequest {
    agent_id: String,
    skill_id: String,
    requested_path: String,
}

#[derive(Debug, Default)]
struct RemoteLeftoverPlan {
    unique_paths: Vec<String>,
    requests_by_path: HashMap<String, Vec<RemoteLeftoverRequest>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteLeftoverPathStatus {
    Ok,
    Missing,
    Err,
}

/// 步骤7：remove_deleted_platform_copies 解耦版。
pub(crate) async fn apply_remove_deleted_platform_copies_step(
    pool: &DbPool,
    active_target: &ActiveTarget,
    removals: Vec<DeletedPlatformCopyRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
    cancel: Option<&AtomicBool>,
) {
    match active_target {
        ActiveTarget::Local => {
            apply_remove_deleted_platform_copies_local(pool, removals, result, allowed_agent_ids)
                .await;
        }
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
            apply_remove_deleted_platform_copies_remote(
                pool,
                active_target,
                removals,
                result,
                allowed_agent_ids,
                cancel,
            )
            .await;
        }
    }
}

async fn apply_remove_deleted_platform_copies_local(
    pool: &DbPool,
    removals: Vec<DeletedPlatformCopyRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
) {
    for removal in removals {
        if !is_agent_allowed(&removal.agent_id, allowed_agent_ids) {
            result.failures.push(SkillUpdateApplyFailure::new(
                "remove_deleted_platform_copy",
                format!("{}::{}", removal.agent_id, removal.skill_id),
            ));
            continue;
        }
        for path in &removal.paths {
            match remove_deleted_platform_copy_local_item(pool, &removal, path).await {
                Ok(()) => result
                    .removed_deleted_platform_copy_paths
                    .push(path.clone()),
                Err(error) => {
                    push_leftover_failure(result, &removal.agent_id, &removal.skill_id, error)
                }
            }
        }
    }
}

async fn apply_remove_deleted_platform_copies_remote(
    pool: &DbPool,
    active_target: &ActiveTarget,
    removals: Vec<DeletedPlatformCopyRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
    cancel: Option<&AtomicBool>,
) {
    let plan = plan_remote_leftover_deletes(pool, removals, allowed_agent_ids, result).await;
    if plan.unique_paths.is_empty() {
        return;
    }
    if is_leftover_cancel_requested(cancel) {
        fail_remote_leftover_paths(&plan, plan.unique_paths.iter(), result);
        return;
    }
    let connection = match connect_remote_target(active_target).await {
        Ok(connection) => connection,
        Err(error) => {
            fail_remote_leftover_paths_with(
                &plan,
                plan.unique_paths.iter(),
                result,
                CentralUpdatesError::Remote(error.to_string()),
            );
            return;
        }
    };
    execute_remote_leftover_deletes(pool, &connection, plan, result, cancel).await;
}

#[cfg(test)]
pub(crate) async fn apply_remove_deleted_platform_copies_on_connection(
    pool: &DbPool,
    connection: &ConnectedRemoteTarget,
    removals: Vec<DeletedPlatformCopyRemoval>,
    result: &mut SkillUpdateApplyResult,
    allowed_agent_ids: Option<&HashSet<String>>,
    cancel: Option<&AtomicBool>,
) {
    let plan = plan_remote_leftover_deletes(pool, removals, allowed_agent_ids, result).await;
    if plan.unique_paths.is_empty() {
        return;
    }
    if is_leftover_cancel_requested(cancel) {
        fail_remote_leftover_paths(&plan, plan.unique_paths.iter(), result);
        return;
    }
    execute_remote_leftover_deletes(pool, connection, plan, result, cancel).await;
}

fn is_leftover_cancel_requested(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|flag| flag.load(Ordering::SeqCst))
}

fn push_leftover_failure(
    result: &mut SkillUpdateApplyResult,
    agent_id: &str,
    skill_id: &str,
    error: CentralUpdatesError,
) {
    result
        .failures
        .push(SkillUpdateApplyFailure::from_central_error(
            "remove_deleted_platform_copy",
            format!("{agent_id}::{skill_id}"),
            CentralUpdateFailurePhase::DecisionApply,
            error,
        ));
}

fn leftover_cancel_error() -> CentralUpdatesError {
    CentralUpdatesError::BatchCancelled
}

async fn remove_deleted_platform_copy_local_item(
    pool: &DbPool,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), CentralUpdatesError> {
    ensure_central_still_missing(pool, &removal.skill_id).await?;
    let agent = db::get_agent_by_id(pool, &removal.agent_id)
        .await?
        .ok_or_else(|| CentralUpdatesError::AgentNotFound(removal.agent_id.clone()))?;
    if removal.agent_id == "central" {
        return Err(CentralUpdatesError::CentralAgentPlatformCopy);
    }
    remove_deleted_platform_copy_local(pool, &agent, removal, path).await
}

async fn plan_remote_leftover_deletes(
    pool: &DbPool,
    removals: Vec<DeletedPlatformCopyRemoval>,
    allowed_agent_ids: Option<&HashSet<String>>,
    result: &mut SkillUpdateApplyResult,
) -> RemoteLeftoverPlan {
    let agents = match db::get_all_agents(pool).await {
        Ok(agents) => agents
            .into_iter()
            .map(|agent| (agent.id.clone(), agent))
            .collect::<HashMap<_, _>>(),
        Err(_error) => {
            for removal in &removals {
                result.failures.push(SkillUpdateApplyFailure::new(
                    "remove_deleted_platform_copy",
                    format!("{}::{}", removal.agent_id, removal.skill_id),
                ));
            }
            return RemoteLeftoverPlan::default();
        }
    };

    let mut central_missing = HashMap::new();
    let mut validated = Vec::new();
    for removal in removals {
        if !is_agent_allowed(&removal.agent_id, allowed_agent_ids) {
            result.failures.push(SkillUpdateApplyFailure::new(
                "remove_deleted_platform_copy",
                format!("{}::{}", removal.agent_id, removal.skill_id),
            ));
            continue;
        }
        if removal.agent_id == "central" {
            push_leftover_failure(
                result,
                &removal.agent_id,
                &removal.skill_id,
                CentralUpdatesError::CentralAgentPlatformCopy,
            );
            continue;
        }
        let Some(agent) = agents.get(&removal.agent_id) else {
            push_leftover_failure(
                result,
                &removal.agent_id,
                &removal.skill_id,
                CentralUpdatesError::AgentNotFound(removal.agent_id.clone()),
            );
            continue;
        };
        if let Err(error) =
            cached_central_still_missing(pool, &mut central_missing, &removal.skill_id).await
        {
            let path_count = removal.paths.len().max(1);
            for _ in 0..path_count {
                push_leftover_failure(
                    result,
                    &removal.agent_id,
                    &removal.skill_id,
                    leftover_plan_error(&error),
                );
            }
            continue;
        }
        for path in removal.paths {
            match validate_remote_leftover_path(agent, &removal.skill_id, &path) {
                Ok(normalized) => validated.push((
                    normalized,
                    RemoteLeftoverRequest {
                        agent_id: removal.agent_id.clone(),
                        skill_id: removal.skill_id.clone(),
                        requested_path: path,
                    },
                )),
                Err(error) => {
                    push_leftover_failure(result, &removal.agent_id, &removal.skill_id, error)
                }
            }
        }
    }
    group_remote_leftover_requests(validated)
}

fn validate_remote_leftover_path(
    agent: &Agent,
    skill_id: &str,
    path: &str,
) -> Result<String, CentralUpdatesError> {
    if path != crate::targets::remote_join(&agent.global_skills_dir, skill_id) {
        return Err(CentralUpdatesError::NotManagedRemoteInstallPath {
            path: path.to_string(),
            skill_id: skill_id.to_string(),
            agent_id: agent.id.clone(),
        });
    }
    ensure_remote_child_path(&agent.global_skills_dir, path, &agent.id)
}

fn group_remote_leftover_requests(
    validated: Vec<(String, RemoteLeftoverRequest)>,
) -> RemoteLeftoverPlan {
    let mut plan = RemoteLeftoverPlan::default();
    for (normalized, request) in validated {
        if !plan.requests_by_path.contains_key(&normalized) {
            plan.unique_paths.push(normalized.clone());
        }
        plan.requests_by_path
            .entry(normalized)
            .or_default()
            .push(request);
    }
    plan
}

pub(crate) fn leftover_remote_chunk_count(unique_path_count: usize) -> usize {
    unique_path_count.div_ceil(REMOTE_LEFTOVER_DELETE_CHUNK_SIZE)
}

#[tracing::instrument(
    skip_all,
    fields(
        target_kind = connection_target_kind(connection),
        removal_count = plan.request_count(),
        unique_path_count = plan.unique_paths.len(),
        remote_chunks = leftover_remote_chunk_count(plan.unique_paths.len())
    )
)]
async fn execute_remote_leftover_deletes(
    pool: &DbPool,
    connection: &ConnectedRemoteTarget,
    plan: RemoteLeftoverPlan,
    result: &mut SkillUpdateApplyResult,
    cancel: Option<&AtomicBool>,
) {
    let mut offset = 0;
    while offset < plan.unique_paths.len() {
        if is_leftover_cancel_requested(cancel) {
            fail_remote_leftover_paths(&plan, plan.unique_paths[offset..].iter(), result);
            return;
        }
        let end = (offset + REMOTE_LEFTOVER_DELETE_CHUNK_SIZE).min(plan.unique_paths.len());
        let chunk = &plan.unique_paths[offset..end];
        let args = chunk.iter().map(String::as_str).collect::<Vec<_>>();
        match connection
            .run_script_cancellable(REMOTE_LEFTOVER_DELETE_SCRIPT, &args, cancel)
            .await
        {
            Ok(stdout) => match parse_remote_leftover_delete_stdout(&stdout, chunk.len()) {
                Ok(statuses) => {
                    settle_remote_leftover_chunk(pool, &plan, chunk, &statuses, result).await;
                }
                Err(error) => {
                    fail_remote_leftover_paths_with(&plan, chunk.iter(), result, error);
                }
            },
            Err(crate::targets::TargetsError::ProcessCancelled(_)) => {
                fail_remote_leftover_paths(&plan, plan.unique_paths[offset..].iter(), result);
                return;
            }
            Err(error) => {
                fail_remote_leftover_paths_with(
                    &plan,
                    chunk.iter(),
                    result,
                    CentralUpdatesError::Remote(error.to_string()),
                );
            }
        }
        offset = end;
    }
}

async fn settle_remote_leftover_chunk(
    pool: &DbPool,
    plan: &RemoteLeftoverPlan,
    chunk: &[String],
    statuses: &[RemoteLeftoverPathStatus],
    result: &mut SkillUpdateApplyResult,
) {
    for (path, status) in chunk.iter().zip(statuses) {
        let requests = plan
            .requests_by_path
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        match status {
            RemoteLeftoverPathStatus::Ok | RemoteLeftoverPathStatus::Missing => {
                if let Err(_error) = cleanup_remote_leftover_path(pool, path, requests).await {
                    for request in requests {
                        push_leftover_failure(
                            result,
                            &request.agent_id,
                            &request.skill_id,
                            CentralUpdatesError::LeftoverRecordCleanupFailed,
                        );
                    }
                    continue;
                }
                for request in requests {
                    result
                        .removed_deleted_platform_copy_paths
                        .push(request.requested_path.clone());
                }
            }
            RemoteLeftoverPathStatus::Err => {
                for request in requests {
                    push_leftover_failure(
                        result,
                        &request.agent_id,
                        &request.skill_id,
                        CentralUpdatesError::RemoteLeftoverDeleteFailed,
                    );
                }
            }
        }
    }
}

async fn cleanup_remote_leftover_path(
    pool: &DbPool,
    normalized_path: &str,
    requests: &[RemoteLeftoverRequest],
) -> Result<(), CentralUpdatesError> {
    let mut path_aliases = vec![normalized_path.to_string()];
    let mut payload_pairs = Vec::new();
    for request in requests {
        if !path_aliases
            .iter()
            .any(|path| path == &request.requested_path)
        {
            path_aliases.push(request.requested_path.clone());
        }
        payload_pairs.push((request.skill_id.clone(), request.agent_id.clone()));
    }
    db::delete_leftover_installations_and_observations_for_paths(
        pool,
        &path_aliases,
        &payload_pairs,
    )
    .await?;
    Ok(())
}

fn fail_remote_leftover_paths<'a>(
    plan: &RemoteLeftoverPlan,
    paths: impl IntoIterator<Item = &'a String>,
    result: &mut SkillUpdateApplyResult,
) {
    fail_remote_leftover_paths_with(plan, paths, result, leftover_cancel_error());
}

fn fail_remote_leftover_paths_with<'a>(
    plan: &RemoteLeftoverPlan,
    paths: impl IntoIterator<Item = &'a String>,
    result: &mut SkillUpdateApplyResult,
    error: CentralUpdatesError,
) {
    for path in paths {
        if let Some(requests) = plan.requests_by_path.get(path) {
            for request in requests {
                push_leftover_failure(
                    result,
                    &request.agent_id,
                    &request.skill_id,
                    leftover_plan_error(&error),
                );
            }
        }
    }
}

fn leftover_plan_error(error: &CentralUpdatesError) -> CentralUpdatesError {
    match error {
        CentralUpdatesError::BatchCancelled => CentralUpdatesError::BatchCancelled,
        CentralUpdatesError::RemoteLeftoverDeleteFailed => {
            CentralUpdatesError::RemoteLeftoverDeleteFailed
        }
        CentralUpdatesError::RemoteLeftoverProtocol => CentralUpdatesError::RemoteLeftoverProtocol,
        CentralUpdatesError::LeftoverRecordCleanupFailed => {
            CentralUpdatesError::LeftoverRecordCleanupFailed
        }
        CentralUpdatesError::Remote(message) => CentralUpdatesError::Remote(message.clone()),
        CentralUpdatesError::CentralSkillReappeared(skill_id) => {
            CentralUpdatesError::CentralSkillReappeared(skill_id.clone())
        }
        CentralUpdatesError::CentralAgentPlatformCopy => {
            CentralUpdatesError::CentralAgentPlatformCopy
        }
        CentralUpdatesError::AgentNotFound(agent_id) => {
            CentralUpdatesError::AgentNotFound(agent_id.clone())
        }
        CentralUpdatesError::NotManagedRemoteInstallPath {
            path,
            skill_id,
            agent_id,
        } => CentralUpdatesError::NotManagedRemoteInstallPath {
            path: path.clone(),
            skill_id: skill_id.clone(),
            agent_id: agent_id.clone(),
        },
        CentralUpdatesError::PlatformRootDeletion(label) => {
            CentralUpdatesError::PlatformRootDeletion(label.clone())
        }
        CentralUpdatesError::RemoteRootDeletionScope(label) => {
            CentralUpdatesError::RemoteRootDeletionScope(label.clone())
        }
        CentralUpdatesError::RemoteRootDeletion { root, label } => {
            CentralUpdatesError::RemoteRootDeletion {
                root: root.clone(),
                label: label.clone(),
            }
        }
        CentralUpdatesError::OutsideRemoteRoot { child, root } => {
            CentralUpdatesError::OutsideRemoteRoot {
                child: child.clone(),
                root: root.clone(),
            }
        }
        CentralUpdatesError::InvalidRemotePath(path) => {
            CentralUpdatesError::InvalidRemotePath(path.clone())
        }
        CentralUpdatesError::RemotePathTraversal(path) => {
            CentralUpdatesError::RemotePathTraversal(path.clone())
        }
        _ => CentralUpdatesError::RemoteLeftoverDeleteFailed,
    }
}

fn connection_target_kind(connection: &ConnectedRemoteTarget) -> &'static str {
    match connection.active_target() {
        ActiveTarget::Local => "local",
        ActiveTarget::Ssh(_) => "ssh",
        ActiveTarget::Wsl(_) => "wsl",
    }
}

impl RemoteLeftoverPlan {
    fn request_count(&self) -> usize {
        self.requests_by_path.values().map(Vec::len).sum()
    }
}

async fn cached_central_still_missing(
    pool: &DbPool,
    cache: &mut HashMap<String, bool>,
    skill_id: &str,
) -> Result<(), CentralUpdatesError> {
    if let Some(&missing) = cache.get(skill_id) {
        return if missing {
            Ok(())
        } else {
            Err(CentralUpdatesError::CentralSkillReappeared(
                skill_id.to_string(),
            ))
        };
    }
    match ensure_central_still_missing(pool, skill_id).await {
        Ok(()) => {
            cache.insert(skill_id.to_string(), true);
            Ok(())
        }
        Err(CentralUpdatesError::CentralSkillReappeared(id)) => {
            cache.insert(id.clone(), false);
            Err(CentralUpdatesError::CentralSkillReappeared(id))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn parse_remote_leftover_delete_stdout(
    stdout: &str,
    path_count: usize,
) -> Result<Vec<RemoteLeftoverPathStatus>, CentralUpdatesError> {
    let mut statuses = vec![None; path_count];
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((status, index_text)) = line.split_once('\t') else {
            return Err(CentralUpdatesError::RemoteLeftoverProtocol);
        };
        if !index_text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(CentralUpdatesError::RemoteLeftoverProtocol);
        }
        let Ok(index) = index_text.parse::<usize>() else {
            return Err(CentralUpdatesError::RemoteLeftoverProtocol);
        };
        if index >= path_count || statuses[index].is_some() {
            return Err(CentralUpdatesError::RemoteLeftoverProtocol);
        }
        statuses[index] = Some(match status {
            "OK" => RemoteLeftoverPathStatus::Ok,
            "MISSING" => RemoteLeftoverPathStatus::Missing,
            "ERR" => RemoteLeftoverPathStatus::Err,
            _ => return Err(CentralUpdatesError::RemoteLeftoverProtocol),
        });
    }
    statuses
        .into_iter()
        .map(|status| status.ok_or(CentralUpdatesError::RemoteLeftoverProtocol))
        .collect()
}

async fn ensure_central_still_missing(
    pool: &DbPool,
    skill_id: &str,
) -> Result<(), CentralUpdatesError> {
    if db::get_central_skills_by_ids(pool, &[skill_id.to_string()])
        .await?
        .is_empty()
    {
        Ok(())
    } else {
        Err(CentralUpdatesError::CentralSkillReappeared(
            skill_id.to_string(),
        ))
    }
}

async fn remove_deleted_platform_copy_local(
    pool: &DbPool,
    agent: &Agent,
    removal: &DeletedPlatformCopyRemoval,
    path: &str,
) -> Result<(), CentralUpdatesError> {
    let root = Path::new(&agent.global_skills_dir);
    let target = Path::new(path);
    ensure_local_child_path(root, target, &removal.agent_id)?;

    if removal.agent_id == "claude-code" {
        let observations = db::get_agent_skill_observations(pool, &removal.agent_id).await?;
        if let Some(obs) = observations.iter().find(|obs| {
            obs.skill_id == removal.skill_id && paths_equivalent_str(&obs.dir_path, path)
        }) {
            if obs.is_read_only || obs.source_kind == "plugin" {
                return Err(CentralUpdatesError::ReadOnlyPluginCopy);
            }
            uninstall_skill(
                pool,
                &InstallTransport::Local,
                &removal.skill_id,
                &removal.agent_id,
                Some(&obs.row_id),
            )
            .await?;
            return Ok(());
        }
    }

    let expected = root.join(&removal.skill_id);
    if !paths_equivalent_path(&expected, target) {
        return Err(CentralUpdatesError::NotManagedInstallPath {
            path: path.to_string(),
            skill_id: removal.skill_id.clone(),
            agent_id: removal.agent_id.clone(),
        });
    }

    uninstall_skill(
        pool,
        &InstallTransport::Local,
        &removal.skill_id,
        &removal.agent_id,
        None,
    )
    .await?;
    Ok(())
}

fn ensure_local_child_path(
    root: &Path,
    child: &Path,
    label: &str,
) -> Result<(), CentralUpdatesError> {
    if crate::paths::paths_equivalent(root, child) {
        return Err(CentralUpdatesError::PlatformRootDeletion(label.to_string()));
    }

    let root_cmp = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let child_parent = child
        .parent()
        .ok_or_else(|| CentralUpdatesError::PathNoParent(child.display().to_string()))?;
    let child_parent_cmp = child_parent
        .canonicalize()
        .unwrap_or_else(|_| child_parent.to_path_buf());
    if !child_parent_cmp.starts_with(&root_cmp) {
        return Err(CentralUpdatesError::OutsidePlatformRoot {
            child: child.display().to_string(),
            root: root.display().to_string(),
        });
    }
    Ok(())
}

fn ensure_remote_child_path(
    root: &str,
    child: &str,
    label: &str,
) -> Result<String, CentralUpdatesError> {
    let root_cmp = normalize_remote_path(root)?;
    let child_cmp = normalize_remote_path(child)?;
    if root_cmp == "/" {
        return Err(CentralUpdatesError::RemoteRootDeletionScope(
            label.to_string(),
        ));
    }
    if root_cmp == child_cmp {
        return Err(CentralUpdatesError::RemoteRootDeletion {
            root: root_cmp,
            label: label.to_string(),
        });
    }
    let prefix = format!("{}/", root_cmp.trim_end_matches('/'));
    if !child_cmp.starts_with(&prefix) {
        return Err(CentralUpdatesError::OutsideRemoteRoot {
            child: child.to_string(),
            root: root.to_string(),
        });
    }
    Ok(child_cmp)
}

fn normalize_remote_path(path: &str) -> Result<String, CentralUpdatesError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') || trimmed.contains('\0') {
        return Err(CentralUpdatesError::InvalidRemotePath(path.to_string()));
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err(CentralUpdatesError::RemotePathTraversal(path.to_string())),
            value => segments.push(value),
        }
    }
    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

fn paths_equivalent_str(left: &str, right: &str) -> bool {
    paths_equivalent_path(Path::new(left), Path::new(right))
}

fn paths_equivalent_path(left: &Path, right: &Path) -> bool {
    crate::paths::paths_equivalent(left, right)
}
