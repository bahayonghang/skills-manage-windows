use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::{
    db::{self, DbPool, Skill, SkillRepositoryAssignment, SkillUpdateState},
    targets::ActiveTarget,
    AppState,
};

use super::{
    github_import::{self, GitHubRepoRef, GitHubRepoSnapshot, RemoteSkillCandidate},
    linker,
};

const UPDATE_PROGRESS_EVENT: &str = "central://skill-update-progress";
const STATUS_UP_TO_DATE: &str = "up_to_date";
const STATUS_UPDATE_AVAILABLE: &str = "update_available";
const STATUS_UNSUPPORTED: &str = "unsupported";
const STATUS_ERROR: &str = "error";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateProgressPayload {
    pub phase: String,
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateFailure {
    pub skill_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateSkip {
    pub skill_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralSkillUpdateResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<CentralSkillUpdateFailure>,
    pub skipped: Vec<CentralSkillUpdateSkip>,
    pub states: Vec<SkillUpdateState>,
}

#[derive(Debug, Clone)]
struct GitHubUpdateSource {
    repo: GitHubRepoRef,
    source_path: String,
}

#[derive(Debug, Clone)]
struct RemoteSkillFile {
    repo_path: String,
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct RemoteSkillContent {
    source: GitHubUpdateSource,
    candidate: RemoteSkillCandidate,
    files: Vec<RemoteSkillFile>,
    remote_hash: String,
    local_hash: String,
    target_dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct UpdateCounters {
    completed: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
}

#[tauri::command]
pub async fn get_central_skill_update_states(
    state: State<'_, AppState>,
) -> Result<Vec<SkillUpdateState>, String> {
    let pool = state.active_db().await?;
    db::get_skill_update_states(&pool).await
}

#[tauri::command]
pub async fn check_central_skill_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Option<Vec<String>>,
) -> Result<Vec<SkillUpdateState>, String> {
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Central update checks are not supported in this version.".to_string());
    }
    let skills = load_selected_central_skills(&state.db, skill_ids.as_deref()).await?;
    let auth = github_import::github_direct_auth_from_settings(&state.db).await?;
    let client = github_import::github_client()?;
    let mut snapshots = HashMap::new();
    let mut counters = UpdateCounters::default();
    let mut states = Vec::with_capacity(skills.len());
    let total = skills.len();

    emit_update_progress(&app, "checking", "started", total, &counters, None, None);

    for skill in skills {
        emit_update_progress(
            &app,
            "checking",
            "running",
            total,
            &counters,
            Some(&skill),
            None,
        );

        let state_result = match load_remote_skill_content(
            &state.db,
            &skill,
            auth.as_deref(),
            &client,
            &mut snapshots,
        )
        .await
        {
            Ok(Some(remote)) => state_from_remote(&skill, &remote, false),
            Ok(None) => unsupported_state(&state.db, &skill, None).await?,
            Err(error) => error_state(&state.db, &skill, &error).await?,
        };

        db::upsert_skill_update_state(&state.db, &state_result).await?;
        update_counters_for_state(&mut counters, &state_result);
        emit_update_progress(
            &app,
            "checking",
            &state_result.status,
            total,
            &counters,
            Some(&skill),
            state_result.error.as_deref(),
        );
        states.push(state_result);
    }

    emit_update_progress(&app, "checking", "completed", total, &counters, None, None);

    Ok(states)
}

#[tauri::command]
pub async fn update_central_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<CentralSkillUpdateResult, String> {
    if skill_ids.is_empty() {
        return Err("Select at least one Central skill to update.".to_string());
    }
    if matches!(state.active_target().await?, ActiveTarget::Ssh(_)) {
        return Err("Remote Central updates are not supported in this version.".to_string());
    }

    let skills = load_selected_central_skills(&state.db, Some(&skill_ids)).await?;
    let auth = github_import::github_direct_auth_from_settings(&state.db).await?;
    let client = github_import::github_client()?;
    let mut snapshots = HashMap::new();
    let mut counters = UpdateCounters::default();
    let total = skills.len();
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut states = Vec::new();

    emit_update_progress(&app, "updating", "started", total, &counters, None, None);

    for skill in skills {
        emit_update_progress(
            &app,
            "updating",
            "running",
            total,
            &counters,
            Some(&skill),
            None,
        );

        match load_remote_skill_content(&state.db, &skill, auth.as_deref(), &client, &mut snapshots)
            .await
        {
            Ok(Some(remote)) if remote.remote_hash == remote.local_hash => {
                let state_result = state_from_remote(&skill, &remote, false);
                db::upsert_skill_update_state(&state.db, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: "Already up to date".to_string(),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_UP_TO_DATE,
                    total,
                    &counters,
                    Some(&skill),
                    None,
                );
                states.push(state_result);
            }
            Ok(Some(remote)) => match update_one_skill(&state.db, &skill, remote).await {
                Ok(state_result) => {
                    db::upsert_skill_update_state(&state.db, &state_result).await?;
                    counters.completed += 1;
                    counters.succeeded += 1;
                    succeeded.push(skill.id.clone());
                    emit_update_progress(
                        &app,
                        "updating",
                        STATUS_UP_TO_DATE,
                        total,
                        &counters,
                        Some(&skill),
                        None,
                    );
                    states.push(state_result);
                }
                Err(error) => {
                    let state_result = error_state(&state.db, &skill, &error).await?;
                    db::upsert_skill_update_state(&state.db, &state_result).await?;
                    counters.completed += 1;
                    counters.failed += 1;
                    failed.push(CentralSkillUpdateFailure {
                        skill_id: skill.id.clone(),
                        error: error.clone(),
                    });
                    emit_update_progress(
                        &app,
                        "updating",
                        STATUS_ERROR,
                        total,
                        &counters,
                        Some(&skill),
                        Some(&error),
                    );
                    states.push(state_result);
                }
            },
            Ok(None) => {
                let state_result = unsupported_state(&state.db, &skill, None).await?;
                db::upsert_skill_update_state(&state.db, &state_result).await?;
                counters.completed += 1;
                counters.skipped += 1;
                skipped.push(CentralSkillUpdateSkip {
                    skill_id: skill.id.clone(),
                    reason: state_result
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unsupported source".to_string()),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_UNSUPPORTED,
                    total,
                    &counters,
                    Some(&skill),
                    state_result.error.as_deref(),
                );
                states.push(state_result);
            }
            Err(error) => {
                let state_result = error_state(&state.db, &skill, &error).await?;
                db::upsert_skill_update_state(&state.db, &state_result).await?;
                counters.completed += 1;
                counters.failed += 1;
                failed.push(CentralSkillUpdateFailure {
                    skill_id: skill.id.clone(),
                    error: error.clone(),
                });
                emit_update_progress(
                    &app,
                    "updating",
                    STATUS_ERROR,
                    total,
                    &counters,
                    Some(&skill),
                    Some(&error),
                );
                states.push(state_result);
            }
        }
    }

    emit_update_progress(&app, "updating", "completed", total, &counters, None, None);

    Ok(CentralSkillUpdateResult {
        succeeded,
        failed,
        skipped,
        states,
    })
}

async fn load_selected_central_skills(
    pool: &DbPool,
    skill_ids: Option<&[String]>,
) -> Result<Vec<Skill>, String> {
    let mut skills = db::get_central_skills(pool).await?;
    if let Some(skill_ids) = skill_ids {
        let requested = skill_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        skills.retain(|skill| requested.contains(&skill.id));
    }
    Ok(skills)
}

async fn load_remote_skill_content(
    pool: &DbPool,
    skill: &Skill,
    auth_token: Option<&str>,
    client: &reqwest::Client,
    snapshots: &mut HashMap<String, GitHubRepoSnapshot>,
) -> Result<Option<RemoteSkillContent>, String> {
    let Some(source) = resolve_github_update_source(pool, skill, auth_token).await? else {
        return Ok(None);
    };

    ensure_snapshot(client, &source.repo, auth_token, snapshots).await?;
    let snapshot = snapshots
        .get(&repo_cache_key(&source.repo))
        .ok_or_else(|| "GitHub repository snapshot is unavailable.".to_string())?;

    let candidates = github_import::build_repo_skill_candidates_from_snapshot_at_path(
        &source.repo,
        snapshot,
        Some(&source.source_path),
    )?;
    let candidate = candidates
        .into_iter()
        .find(|candidate| {
            normalize_repo_path(&candidate.source_path)
                .map(|path| path == source.source_path)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!(
                "Skill source path '{}' no longer contains an importable skill.",
                source.source_path
            )
        })?;

    let files = collect_remote_skill_files(snapshot, &source.source_path)?;
    ensure_remote_skill_manifest(&files)?;

    let remote_hash = hash_remote_files(snapshot, &files)?;
    let target_dir = skill_target_dir(skill)?;
    let local_hash = hash_local_directory(&target_dir)?;

    Ok(Some(RemoteSkillContent {
        source,
        candidate,
        files,
        remote_hash,
        local_hash,
        target_dir,
    }))
}

async fn resolve_github_update_source(
    pool: &DbPool,
    skill: &Skill,
    auth_token: Option<&str>,
) -> Result<Option<GitHubUpdateSource>, String> {
    let assignment = db::get_skill_repository_assignment(pool, &skill.id).await?;
    if assignment.is_source_unknown || assignment.repository.is_unknown {
        return Ok(None);
    }
    if assignment.repository.source_type != "github" {
        return Ok(None);
    }

    let Some(source_path) = assignment
        .source_path
        .as_deref()
        .map(normalize_repo_path)
        .transpose()?
        .filter(|path| !path.is_empty())
    else {
        return Ok(None);
    };

    let repo = if let (Some(owner), Some(repo), Some(branch)) = (
        assignment.repository.owner.as_ref(),
        assignment.repository.repo.as_ref(),
        assignment.repository.branch.as_ref(),
    ) {
        GitHubRepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            branch: branch.clone(),
            normalized_url: assignment
                .repository
                .url
                .clone()
                .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}")),
        }
    } else {
        let url = repository_url(&assignment).ok_or_else(|| {
            format!(
                "GitHub source for skill '{}' is missing repository URL.",
                skill.id
            )
        })?;
        github_import::resolve_repo_source(&url, auth_token)
            .await?
            .repo
    };

    Ok(Some(GitHubUpdateSource { repo, source_path }))
}

fn repository_url(assignment: &SkillRepositoryAssignment) -> Option<String> {
    assignment.repository.url.clone().or_else(|| {
        match (&assignment.repository.owner, &assignment.repository.repo) {
            (Some(owner), Some(repo)) => Some(format!("https://github.com/{owner}/{repo}")),
            _ => None,
        }
    })
}

async fn ensure_snapshot(
    client: &reqwest::Client,
    repo: &GitHubRepoRef,
    auth_token: Option<&str>,
    snapshots: &mut HashMap<String, GitHubRepoSnapshot>,
) -> Result<(), String> {
    let key = repo_cache_key(repo);
    if snapshots.contains_key(&key) {
        return Ok(());
    }

    let snapshot = github_import::download_repo_snapshot(client, repo, auth_token).await?;
    snapshots.insert(key, snapshot);
    Ok(())
}

fn repo_cache_key(repo: &GitHubRepoRef) -> String {
    format!("{}/{}/{}", repo.owner, repo.repo, repo.branch)
}

fn state_from_remote(
    skill: &Skill,
    remote: &RemoteSkillContent,
    updated: bool,
) -> SkillUpdateState {
    let now = Utc::now().to_rfc3339();
    let status = if remote.remote_hash == remote.local_hash {
        STATUS_UP_TO_DATE
    } else {
        STATUS_UPDATE_AVAILABLE
    };

    SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type: "github".to_string(),
        source_url: Some(remote.source.repo.normalized_url.clone()),
        ref_name: Some(remote.source.repo.branch.clone()),
        source_path: Some(remote.source.source_path.clone()),
        last_remote_hash: Some(if updated {
            remote.remote_hash.clone()
        } else {
            remote.local_hash.clone()
        }),
        latest_remote_hash: Some(remote.remote_hash.clone()),
        last_checked_at: Some(now.clone()),
        last_updated_at: if updated { Some(now) } else { None },
        status: if updated {
            STATUS_UP_TO_DATE.to_string()
        } else {
            status.to_string()
        },
        error: None,
    }
}

async fn unsupported_state(
    pool: &DbPool,
    skill: &Skill,
    reason: Option<&str>,
) -> Result<SkillUpdateState, String> {
    let assignment = db::get_skill_repository_assignment(pool, &skill.id).await?;
    let source_url = repository_url(&assignment);
    let source_type = assignment.repository.source_type.clone();
    let ref_name = assignment.repository.branch.clone();
    let source_path = assignment.source_path.clone();
    let reason = reason
        .map(str::to_string)
        .unwrap_or_else(|| unsupported_reason(&assignment));

    Ok(SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type,
        source_url,
        ref_name,
        source_path,
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: STATUS_UNSUPPORTED.to_string(),
        error: Some(reason),
    })
}

async fn error_state(
    pool: &DbPool,
    skill: &Skill,
    error: &str,
) -> Result<SkillUpdateState, String> {
    let assignment = db::get_skill_repository_assignment(pool, &skill.id).await?;
    let source_url = repository_url(&assignment);
    Ok(SkillUpdateState {
        skill_id: skill.id.clone(),
        source_type: assignment.repository.source_type,
        source_url,
        ref_name: assignment.repository.branch,
        source_path: assignment.source_path,
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: STATUS_ERROR.to_string(),
        error: Some(error.to_string()),
    })
}

fn unsupported_reason(assignment: &SkillRepositoryAssignment) -> String {
    if assignment.is_source_unknown || assignment.repository.is_unknown {
        return "Source is unknown or manually assigned.".to_string();
    }
    if assignment.repository.source_type != "github" {
        return format!(
            "Source type '{}' is not supported for automatic updates.",
            assignment.repository.source_type
        );
    }
    if assignment
        .source_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return "GitHub source path is missing.".to_string();
    }
    "Automatic update is not supported for this source.".to_string()
}

async fn update_one_skill(
    pool: &DbPool,
    skill: &Skill,
    remote: RemoteSkillContent,
) -> Result<SkillUpdateState, String> {
    let parent = remote
        .target_dir
        .parent()
        .ok_or_else(|| format!("Skill '{}' target directory has no parent.", skill.id))?;
    std::fs::create_dir_all(parent).map_err(|e| {
        format!(
            "Failed to create parent directory '{}': {}",
            parent.display(),
            e
        )
    })?;

    let temp_dir = parent.join(format!(".skillport-update-{}-{}", skill.id, Uuid::new_v4()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).map_err(|e| {
            format!(
                "Failed to clear stale update directory '{}': {}",
                temp_dir.display(),
                e
            )
        })?;
    }
    write_remote_skill_files(&remote, &temp_dir)?;

    let backup_dir = parent.join(format!(".skillport-backup-{}-{}", skill.id, Uuid::new_v4()));
    replace_target_dir(&remote.target_dir, &temp_dir, &backup_dir)?;

    let skill_md_path = remote.target_dir.join("SKILL.md");
    let updated_skill = Skill {
        id: skill.id.clone(),
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
    refresh_copy_installations(pool, &skill.id, &remote.target_dir).await?;

    Ok(state_from_remote(skill, &remote, true))
}

fn collect_remote_skill_files(
    snapshot: &GitHubRepoSnapshot,
    source_path: &str,
) -> Result<Vec<RemoteSkillFile>, String> {
    let mut files = snapshot
        .files
        .iter()
        .filter_map(|(repo_path, bytes)| {
            let relative_path = if source_path == "." {
                if repo_path.contains('/') {
                    return None;
                }
                repo_path.clone()
            } else {
                let prefix = format!("{}/", source_path.trim_matches('/'));
                let relative = repo_path.strip_prefix(&prefix)?;
                if relative.is_empty() {
                    return None;
                }
                relative.to_string()
            };

            Some(RemoteSkillFile {
                repo_path: repo_path.clone(),
                relative_path,
                bytes: bytes.clone(),
            })
        })
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.repo_path.cmp(&right.repo_path));
    if files.is_empty() {
        return Err(format!(
            "Repository path '{}' is no longer available.",
            source_path
        ));
    }
    Ok(files)
}

fn ensure_remote_skill_manifest(files: &[RemoteSkillFile]) -> Result<(), String> {
    let has_manifest = files
        .iter()
        .any(|file| file.relative_path.eq_ignore_ascii_case("SKILL.md"));
    if has_manifest {
        Ok(())
    } else {
        Err("Remote skill no longer contains SKILL.md.".to_string())
    }
}

fn write_remote_skill_files(remote: &RemoteSkillContent, target_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Failed to create update staging directory '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    for file in &remote.files {
        if !is_safe_relative_path(&file.relative_path) {
            return Err(format!(
                "Repository contains an unsupported path '{}'.",
                file.repo_path
            ));
        }

        let destination = target_dir.join(&file.relative_path);
        let parent = destination.parent().ok_or_else(|| {
            format!(
                "Failed to determine parent directory for '{}'.",
                destination.display()
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create update file parent '{}': {}",
                parent.display(),
                e
            )
        })?;
        std::fs::write(&destination, &file.bytes).map_err(|e| {
            format!(
                "Failed to write update file '{}': {}",
                destination.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn replace_target_dir(target_dir: &Path, temp_dir: &Path, backup_dir: &Path) -> Result<(), String> {
    let had_target = std::fs::symlink_metadata(target_dir).is_ok();
    if had_target {
        std::fs::rename(target_dir, backup_dir).map_err(|e| {
            format!(
                "Failed to stage existing skill directory '{}' for replacement: {}",
                target_dir.display(),
                e
            )
        })?;
    }

    if let Err(error) = std::fs::rename(temp_dir, target_dir) {
        if had_target {
            let _ = std::fs::rename(backup_dir, target_dir);
        }
        return Err(format!(
            "Failed to replace skill directory '{}': {}",
            target_dir.display(),
            error
        ));
    }

    if had_target {
        remove_path(backup_dir).map_err(|e| {
            format!(
                "Updated skill directory, but failed to remove backup '{}': {}",
                backup_dir.display(),
                e
            )
        })?;
    }

    Ok(())
}

async fn refresh_copy_installations(
    pool: &DbPool,
    skill_id: &str,
    source_dir: &Path,
) -> Result<(), String> {
    let installations = db::get_skill_installations(pool, skill_id).await?;
    for installation in installations {
        if installation.link_type != "copy" {
            continue;
        }
        let target = PathBuf::from(&installation.installed_path);
        if target.file_name().and_then(|value| value.to_str()) != Some(skill_id) {
            return Err(format!(
                "Refusing to refresh copy install outside expected skill directory '{}'.",
                target.display()
            ));
        }
        if std::fs::symlink_metadata(&target).is_ok() {
            remove_path(&target)?;
        }
        linker::copy_dir_all(source_dir, &target)?;
    }
    Ok(())
}

fn skill_target_dir(skill: &Skill) -> Result<PathBuf, String> {
    skill
        .canonical_path
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Path::new(&skill.file_path).parent().map(Path::to_path_buf))
        .ok_or_else(|| format!("Skill '{}' has no canonical directory.", skill.id))
}

fn hash_remote_files(
    _snapshot: &GitHubRepoSnapshot,
    files: &[RemoteSkillFile],
) -> Result<String, String> {
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        entries.push((file.relative_path.clone(), file.bytes.clone()));
    }
    Ok(hash_entries(entries))
}

fn hash_local_directory(root: &Path) -> Result<String, String> {
    let mut entries = Vec::new();
    collect_local_hash_entries(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(hash_entries(entries))
}

fn collect_local_hash_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current).map_err(|e| {
        format!(
            "Failed to read local skill directory '{}': {}",
            current.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| format!("Failed to read local skill entry: {}", e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect local skill entry '{}': {}",
                path.display(),
                e
            )
        })?;
        if file_type.is_dir() {
            collect_local_hash_entries(root, &path, entries)?;
        } else if file_type.is_file() {
            let relative_path = relative_path_string(root, &path)?;
            let bytes = std::fs::read(&path).map_err(|e| {
                format!(
                    "Failed to read local skill file '{}': {}",
                    path.display(),
                    e
                )
            })?;
            entries.push((relative_path, bytes));
        }
    }
    Ok(())
}

fn hash_entries(mut entries: Vec<(String, Vec<u8>)>) -> String {
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = 0xcbf29ce484222325u64;
    for (path, bytes) in entries {
        hash = fnv1a(hash, path.as_bytes());
        hash = fnv1a(hash, &[0xff]);
        hash = fnv1a(hash, &bytes);
        hash = fnv1a(hash, &[0xfe]);
    }
    format!("fnv1a64:{hash:016x}")
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|e| {
        format!(
            "Failed to compute relative path for '{}': {}",
            path.display(),
            e
        )
    })?;
    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err(format!(
                "Local skill path '{}' contains unsupported components.",
                path.display()
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join("/"))
}

fn normalize_repo_path(path: &str) -> Result<String, String> {
    let normalized = path.trim().trim_matches('/').replace('\\', "/");
    let normalized = if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    };
    if !is_safe_repo_path(&normalized) {
        return Err(format!("Repository path '{}' is not supported.", path));
    }
    Ok(normalized)
}

fn is_safe_repo_path(path: &str) -> bool {
    path == "." || is_safe_relative_path(path)
}

fn is_safe_relative_path(path: &str) -> bool {
    let relative = Path::new(path);
    !relative.is_absolute()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn remove_path(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_symlink_path(path),
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory '{}': {}", path.display(), e)),
        Ok(_) => std::fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file '{}': {}", path.display(), e)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to inspect '{}': {}", path.display(), error)),
    }
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_dir(path)
        .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e))
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path)
        .map_err(|e| format!("Failed to remove symlink '{}': {}", path.display(), e))
}

fn update_counters_for_state(counters: &mut UpdateCounters, state: &SkillUpdateState) {
    counters.completed += 1;
    match state.status.as_str() {
        STATUS_UP_TO_DATE | STATUS_UPDATE_AVAILABLE => counters.succeeded += 1,
        STATUS_UNSUPPORTED => counters.skipped += 1,
        STATUS_ERROR => counters.failed += 1,
        _ => {}
    }
}

fn emit_update_progress(
    app: &AppHandle,
    phase: &str,
    status: &str,
    total: usize,
    counters: &UpdateCounters,
    skill: Option<&Skill>,
    error: Option<&str>,
) {
    let payload = CentralSkillUpdateProgressPayload {
        phase: phase.to_string(),
        status: status.to_string(),
        total,
        completed: counters.completed,
        succeeded: counters.succeeded,
        failed: counters.failed,
        skipped: counters.skipped,
        skill_id: skill.map(|skill| skill.id.clone()),
        skill_name: skill.map(|skill| skill.name.clone()),
        error: error.map(str::to_string),
    };
    let _ = app.emit(UPDATE_PROGRESS_EVENT, payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn hash_entries_is_stable_across_input_order() {
        let left = hash_entries(vec![
            ("b.txt".to_string(), b"two".to_vec()),
            ("a.txt".to_string(), b"one".to_vec()),
        ]);
        let right = hash_entries(vec![
            ("a.txt".to_string(), b"one".to_vec()),
            ("b.txt".to_string(), b"two".to_vec()),
        ]);

        assert_eq!(left, right);
    }

    #[test]
    fn local_hash_changes_when_file_content_changes() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("demo");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();

        let first = hash_local_directory(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), b"two").unwrap();
        let second = hash_local_directory(&skill_dir).unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn collect_remote_skill_files_requires_source_path() {
        let snapshot = GitHubRepoSnapshot {
            files: HashMap::from([(
                "skills/demo/SKILL.md".to_string(),
                b"---\nname: Demo\n---".to_vec(),
            )]),
        };

        let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "SKILL.md");
    }
}
