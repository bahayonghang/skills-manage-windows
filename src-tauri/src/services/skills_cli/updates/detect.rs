//! Grouped GitHub update detection and cache load.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use chrono::Utc;

use crate::db::repos::skills_cli_updates_repo::{
    self, list_pending_update_operations, list_update_repositories, list_update_states,
    upsert_update_repository_in_transaction, upsert_update_state_in_transaction,
};
use crate::db::DbPool;
use crate::db::{
    SkillsCliUpdateOperationRow, SkillsCliUpdateRepositoryRow as PersistedRepository,
    SkillsCliUpdateStateRow as PersistedSkill,
};
use crate::fs_util::run_blocking_fs_with;
use crate::services::github_import::candidate_content_digest_from_snapshot;

use super::super::{
    check_cancel, list_global, list_global_at, SkillsCliError, SkillsCliGlobalSkill,
    SkillsCliPlacementState, SkillsCliSourceTypeBucket, SkillsCliTransport,
};
use super::capability::{apply_argv_preview, update_capability_plan};
use super::digest::digest_skill_directory;
use super::github::{GithubObserveRequest, SkillsCliUpdateGithub};
use super::source::parse_github_update_identity;
use super::status::{classify_successful_check, SkillsCliPersistedUpdateStatus};
use super::{
    map_db_error, SkillsCliPendingRecovery, SkillsCliUpdateBlocker, SkillsCliUpdateInventory,
    SkillsCliUpdateProgress, SkillsCliUpdateRepositoryRow, SkillsCliUpdateSkillRow,
    UpdateProgressEmitter,
};

struct ScopedSkill {
    skill: SkillsCliGlobalSkill,
    repository_key: Option<String>,
    normalized_source: Option<String>,
    branch: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
    skill_path: Option<String>,
    local_digest: Option<String>,
    unsupported: bool,
}

pub(crate) async fn load_update_inventory(
    pool: &DbPool,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    let repositories = list_update_repositories(pool).await.map_err(map_db_error)?;
    let skills = list_update_states(pool).await.map_err(map_db_error)?;
    let pending = list_pending_update_operations(pool)
        .await
        .map_err(map_db_error)?;
    Ok(assemble_inventory(repositories, skills, pending))
}

pub(crate) async fn check_updates(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    github: &dyn SkillsCliUpdateGithub,
    progress: &dyn UpdateProgressEmitter,
    job_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    check_cancel(cancel)?;
    let snapshot = list_global(tx, pool).await?;
    let digests = collect_installed_digests(tx, &snapshot.skills).await?;
    check_updates_from_snapshot(pool, github, progress, job_id, cancel, snapshot, digests).await
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn check_updates_at(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    github: &dyn SkillsCliUpdateGithub,
    progress: &dyn UpdateProgressEmitter,
    job_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    check_cancel(cancel)?;
    let snapshot = list_global_at(pool, canonical_root, lock_path).await?;
    let mut digests = BTreeMap::new();
    for skill in &snapshot.skills {
        let canonical = skill
            .canonical_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| canonical_root.join(&skill.name));
        if canonical.is_dir() {
            let path = canonical.clone();
            let digest = run_blocking_fs_with(
                "skills-cli-update-digest",
                move || digest_skill_directory(&path),
                SkillsCliError::task_join,
            )
            .await?;
            digests.insert(skill.name.clone(), digest);
        }
    }
    check_updates_from_snapshot(pool, github, progress, job_id, cancel, snapshot, digests).await
}

async fn collect_installed_digests(
    tx: &SkillsCliTransport,
    skills: &[SkillsCliGlobalSkill],
) -> Result<BTreeMap<String, String>, SkillsCliError> {
    if tx.is_remote() {
        let roots: Vec<String> = skills
            .iter()
            .map(|skill| {
                tx.paths()
                    .join_child(tx.paths().canonical_root(), &skill.name)
            })
            .collect();
        let by_path = tx.digest_remote_skill_dirs(&roots).await?;
        let mut by_name = BTreeMap::new();
        for skill in skills {
            let path = tx
                .paths()
                .join_child(tx.paths().canonical_root(), &skill.name);
            if let Some(digest) = by_path.get(&path) {
                by_name.insert(skill.name.clone(), digest.clone());
            }
        }
        return Ok(by_name);
    }
    let canonical_root = tx.paths().canonical_root_path();
    let mut digests = BTreeMap::new();
    for skill in skills {
        let canonical = skill
            .canonical_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| canonical_root.join(&skill.name));
        if canonical.is_dir() {
            let path = canonical.clone();
            let digest = run_blocking_fs_with(
                "skills-cli-update-digest",
                move || digest_skill_directory(&path),
                SkillsCliError::task_join,
            )
            .await?;
            digests.insert(skill.name.clone(), digest);
        }
    }
    Ok(digests)
}

async fn check_updates_from_snapshot(
    pool: &DbPool,
    github: &dyn SkillsCliUpdateGithub,
    progress: &dyn UpdateProgressEmitter,
    job_id: &str,
    cancel: Option<&AtomicBool>,
    snapshot: crate::services::skills_cli::SkillsCliGlobalSnapshot,
    digests: BTreeMap<String, String>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    let mut scoped = Vec::new();
    for skill in snapshot.skills {
        let digest = digests.get(&skill.name).cloned();
        scoped.push(scope_skill(skill, digest).await?);
    }
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, item) in scoped.iter().enumerate() {
        if let Some(key) = item.repository_key.as_ref() {
            if !item.unsupported {
                groups.entry(key.clone()).or_default().push(index);
            }
        }
    }
    let repository_total = groups.len() as u32;
    progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: job_id.to_string(),
        phase: "checking".to_string(),
        repository_total,
        repository_completed: 0,
        current_repository_key: None,
        selected_total: 0,
        selected_completed: 0,
        terminal_status: None,
    });

    let existing_states = list_update_states(pool).await.map_err(map_db_error)?;
    let existing_repos = list_update_repositories(pool).await.map_err(map_db_error)?;
    let mut repo_rows: Vec<PersistedRepository> = Vec::new();
    let mut skill_rows: Vec<(PersistedSkill, bool, bool)> = Vec::new();
    let mut rate_limited_rest = false;
    let mut completed = 0_u32;

    for (repository_key, indexes) in &groups {
        check_cancel(cancel)?;
        let sample = &scoped[indexes[0]];
        let owner = sample.owner.clone().unwrap_or_default();
        let repo = sample.repo.clone().unwrap_or_default();
        let branch = sample.branch.clone().unwrap_or_else(|| "main".to_string());
        let normalized = sample
            .normalized_source
            .clone()
            .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}"));
        progress.emit_update_progress(&SkillsCliUpdateProgress {
            job_id: job_id.to_string(),
            phase: "checking".to_string(),
            repository_total,
            repository_completed: completed,
            current_repository_key: Some(repository_key.clone()),
            selected_total: 0,
            selected_completed: 0,
            terminal_status: None,
        });

        let previous_repo = existing_repos
            .iter()
            .find(|row| row.repository_key == *repository_key);
        if rate_limited_rest {
            repo_rows.push(failed_repo_row(
                repository_key,
                &normalized,
                &branch,
                SkillsCliPersistedUpdateStatus::RateLimited,
                "skills_cli.update_rate_limited",
                previous_repo,
            ));
            for index in indexes {
                skill_rows.push(stale_skill_row(
                    &scoped[*index],
                    existing_states
                        .iter()
                        .find(|row| row.skill_name == scoped[*index].skill.name),
                    SkillsCliPersistedUpdateStatus::RateLimited,
                    Some("skills_cli.update_rate_limited"),
                ));
            }
            completed += 1;
            continue;
        }

        let observed = github
            .observe_repository(GithubObserveRequest {
                owner: owner.clone(),
                repo: repo.clone(),
                branch: branch.clone(),
                etag: previous_repo.and_then(|row| row.etag.clone()),
            })
            .await;
        match observed {
            Ok(result) => {
                if result.rate_limit_remaining == Some(0) {
                    rate_limited_rest = true;
                }
                let snapshot_digest =
                    crate::services::github_import::repository_snapshot_digest_from_local(
                        &result.snapshot,
                    );
                repo_rows.push(PersistedRepository {
                    repository_key: repository_key.clone(),
                    normalized_source: normalized.clone(),
                    branch: branch.clone(),
                    observed_revision_sha: Some(result.revision_sha.clone()),
                    repository_snapshot_digest: Some(snapshot_digest),
                    etag: result.etag.clone(),
                    status: "current".to_string(),
                    last_checked_at: Some(Utc::now().to_rfc3339()),
                    last_attempted_at: Some(Utc::now().to_rfc3339()),
                    last_error_code: None,
                    rate_limit_remaining: result.rate_limit_remaining,
                    rate_limit_reset_at: result.rate_limit_reset_at.clone(),
                    updated_at: Utc::now().to_rfc3339(),
                });
                for index in indexes {
                    let item = &scoped[*index];
                    let previous = existing_states
                        .iter()
                        .find(|row| row.skill_name == item.skill.name);
                    let skill_path = item
                        .skill_path
                        .clone()
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| item.skill.name.clone());
                    let upstream =
                        candidate_content_digest_from_snapshot(&result.snapshot, &skill_path).ok();
                    skill_rows.push(successful_skill_row(
                        item,
                        previous,
                        &result.revision_sha,
                        upstream.as_deref(),
                        repository_key,
                        &normalized,
                        &skill_path,
                    ));
                }
            }
            Err(SkillsCliError::UpdateRateLimited { reset_at }) => {
                rate_limited_rest = true;
                repo_rows.push(failed_repo_row(
                    repository_key,
                    &normalized,
                    &branch,
                    SkillsCliPersistedUpdateStatus::RateLimited,
                    "skills_cli.update_rate_limited",
                    previous_repo,
                ));
                if let Some(reset) = reset_at {
                    if let Some(last) = repo_rows.last_mut() {
                        last.rate_limit_reset_at = Some(reset);
                    }
                }
                for index in indexes {
                    skill_rows.push(stale_skill_row(
                        &scoped[*index],
                        existing_states
                            .iter()
                            .find(|row| row.skill_name == scoped[*index].skill.name),
                        SkillsCliPersistedUpdateStatus::RateLimited,
                        Some("skills_cli.update_rate_limited"),
                    ));
                }
            }
            Err(_) => {
                repo_rows.push(failed_repo_row(
                    repository_key,
                    &normalized,
                    &branch,
                    SkillsCliPersistedUpdateStatus::Failed,
                    "skills_cli.update_check_failed",
                    previous_repo,
                ));
                for index in indexes {
                    skill_rows.push(stale_skill_row(
                        &scoped[*index],
                        existing_states
                            .iter()
                            .find(|row| row.skill_name == scoped[*index].skill.name),
                        SkillsCliPersistedUpdateStatus::Failed,
                        Some("skills_cli.update_check_failed"),
                    ));
                }
            }
        }
        completed += 1;
    }

    for item in &scoped {
        if item.unsupported || item.repository_key.is_none() {
            let previous = existing_states
                .iter()
                .find(|row| row.skill_name == item.skill.name);
            skill_rows.push(unsupported_skill_row(item, previous));
        }
    }

    let mut transaction = pool.begin().await.map_err(map_db_error)?;
    for row in &repo_rows {
        upsert_update_repository_in_transaction(&mut transaction, row)
            .await
            .map_err(map_db_error)?;
    }
    for (row, overwrite_installed, clear_pending) in &skill_rows {
        upsert_update_state_in_transaction(
            &mut transaction,
            row,
            *overwrite_installed,
            *clear_pending,
        )
        .await
        .map_err(map_db_error)?;
    }
    transaction.commit().await.map_err(map_db_error)?;

    progress.emit_update_progress(&SkillsCliUpdateProgress {
        job_id: job_id.to_string(),
        phase: "completed".to_string(),
        repository_total,
        repository_completed: repository_total,
        current_repository_key: None,
        selected_total: 0,
        selected_completed: 0,
        terminal_status: Some("completed".to_string()),
    });
    load_update_inventory(pool).await
}

pub(crate) async fn verify_update_baseline(
    tx: &SkillsCliTransport,
    pool: &DbPool,
    skill_names: &[String],
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    check_cancel(cancel)?;
    let snapshot = list_global(tx, pool).await?;
    let digests = collect_installed_digests(tx, &snapshot.skills).await?;
    verify_update_baseline_from_snapshot(pool, &snapshot.skills, &digests, skill_names, cancel)
        .await
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn verify_update_baseline_at(
    pool: &DbPool,
    canonical_root: &Path,
    lock_path: &Path,
    skill_names: &[String],
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    check_cancel(cancel)?;
    let snapshot = list_global_at(pool, canonical_root, lock_path).await?;
    let mut digests = BTreeMap::new();
    for skill in &snapshot.skills {
        let canonical = skill
            .canonical_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| canonical_root.join(&skill.name));
        if canonical.is_dir() {
            let path = canonical.clone();
            let digest = run_blocking_fs_with(
                "skills-cli-verify-digest",
                move || digest_skill_directory(&path),
                SkillsCliError::task_join,
            )
            .await?;
            digests.insert(skill.name.clone(), digest);
        }
    }
    verify_update_baseline_from_snapshot(pool, &snapshot.skills, &digests, skill_names, cancel)
        .await
}

async fn verify_update_baseline_from_snapshot(
    pool: &DbPool,
    skills: &[SkillsCliGlobalSkill],
    digests: &BTreeMap<String, String>,
    skill_names: &[String],
    cancel: Option<&AtomicBool>,
) -> Result<SkillsCliUpdateInventory, SkillsCliError> {
    let mut transaction = pool.begin().await.map_err(map_db_error)?;
    for name in skill_names {
        check_cancel(cancel)?;
        let Some(_) = skills.iter().find(|skill| skill.name == *name) else {
            continue;
        };
        let local = digests.get(name).cloned();
        let existing = skills_cli_updates_repo::get_update_state(pool, name)
            .await
            .map_err(map_db_error)?;
        let Some(existing) = existing else {
            continue;
        };
        let matches = local.as_deref() == existing.observed_upstream_digest.as_deref()
            && existing.observed_revision_sha.is_some()
            && existing.observed_upstream_digest.is_some();
        if !matches {
            continue;
        }
        let now = Utc::now().to_rfc3339();
        let mut next = existing.clone();
        next.installed_revision_sha = existing.observed_revision_sha.clone();
        next.installed_upstream_digest = existing.observed_upstream_digest.clone();
        next.installed_local_digest = local.clone();
        next.installed_at = Some(now.clone());
        next.status = SkillsCliPersistedUpdateStatus::Current.as_str().to_string();
        next.last_error_code = None;
        next.is_stale = 0;
        next.updated_at = now;
        upsert_update_state_in_transaction(&mut transaction, &next, true, true)
            .await
            .map_err(map_db_error)?;
    }
    transaction.commit().await.map_err(map_db_error)?;
    load_update_inventory(pool).await
}

async fn scope_skill(
    skill: SkillsCliGlobalSkill,
    local_digest: Option<String>,
) -> Result<ScopedSkill, SkillsCliError> {
    let source = skill
        .source_url
        .as_deref()
        .or(skill.source.as_deref())
        .unwrap_or("");
    let github = skill.source_type_bucket == SkillsCliSourceTypeBucket::Github;
    if !github {
        return Ok(ScopedSkill {
            skill,
            repository_key: None,
            normalized_source: None,
            branch: None,
            owner: None,
            repo: None,
            skill_path: None,
            local_digest: None,
            unsupported: true,
        });
    }
    let parsed = parse_github_update_identity(source, None);
    let Ok(identity) = parsed else {
        return Ok(ScopedSkill {
            skill,
            repository_key: None,
            normalized_source: None,
            branch: None,
            owner: None,
            repo: None,
            skill_path: None,
            local_digest: None,
            unsupported: true,
        });
    };
    let skill_path = if identity.skill_path.is_empty() {
        skill.name.clone()
    } else {
        identity.skill_path.clone()
    };
    Ok(ScopedSkill {
        skill,
        repository_key: Some(identity.repository_key),
        normalized_source: Some(identity.normalized_source),
        branch: Some(identity.branch),
        owner: Some(identity.owner),
        repo: Some(identity.repo),
        skill_path: Some(skill_path),
        local_digest,
        unsupported: false,
    })
}

fn successful_skill_row(
    item: &ScopedSkill,
    previous: Option<&PersistedSkill>,
    observed_sha: &str,
    observed_digest: Option<&str>,
    repository_key: &str,
    normalized: &str,
    skill_path: &str,
) -> (PersistedSkill, bool, bool) {
    let Some(digest) = observed_digest else {
        return stale_skill_row(
            item,
            previous,
            SkillsCliPersistedUpdateStatus::Failed,
            Some("skills_cli.update_check_failed"),
        );
    };
    let source_changed = previous.is_some_and(|row| {
        row.repository_key.as_deref() != Some(repository_key)
            || row.skill_path.as_deref() != Some(skill_path)
    });
    let classified = classify_successful_check(
        source_changed,
        previous.and_then(|row| row.installed_revision_sha.as_deref()),
        previous.and_then(|row| row.installed_upstream_digest.as_deref()),
        previous.and_then(|row| row.installed_local_digest.as_deref()),
        item.local_digest.as_deref(),
        observed_sha,
        digest,
    );
    let now = Utc::now().to_rfc3339();
    let row = PersistedSkill {
        skill_name: item.skill.name.clone(),
        repository_key: Some(repository_key.to_string()),
        normalized_source: Some(normalized.to_string()),
        skill_path: Some(skill_path.to_string()),
        installed_revision_sha: None,
        installed_upstream_digest: None,
        installed_local_digest: None,
        installed_at: None,
        observed_revision_sha: Some(observed_sha.to_string()),
        observed_upstream_digest: Some(digest.to_string()),
        observed_at: Some(now.clone()),
        pending_revision_sha: classified.pending_revision_sha,
        pending_upstream_digest: classified.pending_upstream_digest,
        pending_detected_at: if classified.clear_pending {
            None
        } else {
            Some(now.clone())
        },
        status: classified.status.as_str().to_string(),
        last_error_code: None,
        is_stale: 0,
        updated_at: now,
    };
    (row, false, classified.clear_pending)
}

fn stale_skill_row(
    item: &ScopedSkill,
    previous: Option<&PersistedSkill>,
    status: SkillsCliPersistedUpdateStatus,
    error: Option<&str>,
) -> (PersistedSkill, bool, bool) {
    let now = Utc::now().to_rfc3339();
    let row = PersistedSkill {
        skill_name: item.skill.name.clone(),
        repository_key: item.repository_key.clone(),
        normalized_source: item.normalized_source.clone(),
        skill_path: item.skill_path.clone(),
        installed_revision_sha: None,
        installed_upstream_digest: None,
        installed_local_digest: None,
        installed_at: None,
        observed_revision_sha: None,
        observed_upstream_digest: None,
        observed_at: None,
        pending_revision_sha: None,
        pending_upstream_digest: None,
        pending_detected_at: None,
        status: status.as_str().to_string(),
        last_error_code: error.map(str::to_string),
        is_stale: 1,
        updated_at: now,
    };
    let _ = previous;
    (row, false, false)
}

fn unsupported_skill_row(
    item: &ScopedSkill,
    previous: Option<&PersistedSkill>,
) -> (PersistedSkill, bool, bool) {
    stale_skill_row(
        item,
        previous,
        SkillsCliPersistedUpdateStatus::Unsupported,
        Some("skills_cli.update_unsupported"),
    )
}

fn failed_repo_row(
    repository_key: &str,
    normalized: &str,
    branch: &str,
    status: SkillsCliPersistedUpdateStatus,
    error: &str,
    previous: Option<&PersistedRepository>,
) -> PersistedRepository {
    PersistedRepository {
        repository_key: repository_key.to_string(),
        normalized_source: normalized.to_string(),
        branch: branch.to_string(),
        observed_revision_sha: previous.and_then(|row| row.observed_revision_sha.clone()),
        repository_snapshot_digest: previous.and_then(|row| row.repository_snapshot_digest.clone()),
        etag: previous.and_then(|row| row.etag.clone()),
        status: status.as_str().to_string(),
        last_checked_at: previous.and_then(|row| row.last_checked_at.clone()),
        last_attempted_at: Some(Utc::now().to_rfc3339()),
        last_error_code: Some(error.to_string()),
        rate_limit_remaining: None,
        rate_limit_reset_at: previous.and_then(|row| row.rate_limit_reset_at.clone()),
        updated_at: Utc::now().to_rfc3339(),
    }
}

fn assemble_inventory(
    repositories: Vec<PersistedRepository>,
    skills: Vec<PersistedSkill>,
    pending: Vec<SkillsCliUpdateOperationRow>,
) -> SkillsCliUpdateInventory {
    let last_success_at = repositories
        .iter()
        .filter(|row| row.status == "current")
        .filter_map(|row| row.last_checked_at.clone())
        .max();
    let pending_recovery = pending.first().map(|row| SkillsCliPendingRecovery {
        operation_id: row.id.clone(),
        phase: row.phase.clone(),
        last_error_code: row.last_error_code.clone(),
    });
    let public_repos = repositories
        .iter()
        .map(|row| {
            let pending_count = skills
                .iter()
                .filter(|skill| {
                    skill.repository_key.as_deref() == Some(row.repository_key.as_str())
                        && skill.pending_revision_sha.is_some()
                })
                .count() as u32;
            SkillsCliUpdateRepositoryRow {
                repository_key: row.repository_key.clone(),
                normalized_source: row.normalized_source.clone(),
                branch: row.branch.clone(),
                observed_revision_sha: row.observed_revision_sha.clone(),
                status: row.status.clone(),
                last_checked_at: row.last_checked_at.clone(),
                last_error_code: row.last_error_code.clone(),
                rate_limit_reset_at: row.rate_limit_reset_at.clone(),
                pending_count,
            }
        })
        .collect();
    let public_skills = skills
        .into_iter()
        .map(|row| {
            let status = SkillsCliPersistedUpdateStatus::from_persisted(&row.status).to_public();
            SkillsCliUpdateSkillRow {
                argv_preview: apply_argv_preview(std::slice::from_ref(&row.skill_name)),
                blockers: Vec::new(),
                change_summary: Vec::new(),
                is_stale: row.is_stale != 0,
                last_error_code: row.last_error_code.clone(),
                installed_local_digest: row.installed_local_digest.clone(),
                installed_revision_sha: row.installed_revision_sha.clone(),
                observed_revision_sha: row.observed_revision_sha.clone(),
                observed_upstream_digest: row.observed_upstream_digest.clone(),
                pending_revision_sha: row.pending_revision_sha.clone(),
                pending_upstream_digest: row.pending_upstream_digest.clone(),
                repository_key: row.repository_key.clone(),
                normalized_source: row.normalized_source.clone(),
                skill_path: row.skill_path.clone(),
                skill_name: row.skill_name,
                status,
            }
        })
        .collect();
    SkillsCliUpdateInventory {
        skills: public_skills,
        repositories: public_repos,
        last_success_at,
        pending_recovery,
        capability: update_capability_plan(),
    }
}

pub fn topology_blockers(skill: &SkillsCliGlobalSkill) -> Vec<SkillsCliUpdateBlocker> {
    skill
        .placements
        .iter()
        .filter(|placement| {
            matches!(
                placement.state,
                SkillsCliPlacementState::DirectCopy | SkillsCliPlacementState::Conflict
            )
        })
        .map(|placement| SkillsCliUpdateBlocker {
            code: if placement.state == SkillsCliPlacementState::DirectCopy {
                "skills_cli.update_unsupported".to_string()
            } else {
                "skills_cli.update_topology_conflict".to_string()
            },
            skill_name: skill.name.clone(),
        })
        .collect()
}
