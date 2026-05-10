use chrono::Utc;
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use super::github_import::{self, DuplicateResolution, GitHubSkillImportSelection};
use crate::operation_log::{
    local_target_context, record_operation_log_best_effort, OperationLogEvent,
};
use crate::{
    db::{self, DbPool},
    AppState,
};

const EXPORT_KIND: &str = "skillport/state-export";
const EXPORT_VERSION: u32 = 1;
const REMOTE_CATALOG_CONCURRENCY_LIMIT: usize = 4;
const PORTABILITY_PROGRESS_EVENT: &str = "central://state-portability-progress";
const STATUS_CANCELLED: &str = "cancelled";
const PORTABILITY_CANCELLED_MESSAGE: &str = "SkillPort state portability cancelled";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateExportOptions {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateManifest {
    pub kind: String,
    pub version: u32,
    pub exported_at: String,
    pub exported_from: ExportedFrom,
    pub github_sources: Vec<PortableGithubSource>,
    pub central_skills: Vec<PortableCentralSkill>,
    pub unrestorable_skills: Vec<PortableUnrestorableSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFrom {
    pub app: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableGithubSource {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableCentralSkill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: PortableCentralSkillSource,
    pub tags: Vec<PortableSkillTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableCentralSkillSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub url: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableSkillTag {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableUnrestorableSkill {
    pub id: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreview {
    pub github_sources: Vec<SkillportStateSourcePreview>,
    pub skills: Vec<SkillportStateSkillPreview>,
    pub summary: SkillportStateImportPreviewSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateSourcePreview {
    pub name: String,
    pub url: String,
    pub status: SourcePreviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePreviewStatus {
    Exists,
    WillAdd,
    Duplicate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateSkillPreview {
    pub id: String,
    pub name: String,
    pub source_path: Option<String>,
    pub status: SkillPreviewStatus,
    pub existing_skill_id: Option<String>,
    pub reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillPreviewStatus {
    Ready,
    Conflict,
    Missing,
    Unrestorable,
    DuplicateSkipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreviewSummary {
    pub sources_to_add: usize,
    pub sources_existing: usize,
    pub sources_duplicate: usize,
    pub ready: usize,
    pub conflicts: usize,
    pub missing: usize,
    pub unrestorable: usize,
    pub duplicate_skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportResolution {
    pub skill_id: String,
    pub source_path: Option<String>,
    pub resolution: DuplicateResolution,
    pub renamed_skill_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportResult {
    pub sources_added: usize,
    pub sources_skipped: usize,
    pub imported_skills: Vec<SkillportStateImportedSkill>,
    pub skipped_skills: Vec<String>,
    pub failed_skills: Vec<SkillportStateImportFailure>,
    pub tags_restored: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportedSkill {
    pub source_path: String,
    pub imported_skill_id: String,
    pub skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportFailure {
    pub skill_id: String,
    pub source_path: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillportStatePortabilityPhase {
    Exporting,
    Previewing,
    Importing,
    Finalizing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillportStatePortabilityStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelling,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStatePortabilityProgressPayload {
    pub phase: SkillportStatePortabilityPhase,
    pub status: SkillportStatePortabilityStatus,
    pub total: usize,
    pub completed: usize,
    pub message: Option<String>,
    pub current_item: Option<String>,
    pub error: Option<String>,
}

type CancelFlag = Arc<AtomicBool>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RepoKey {
    owner: String,
    repo: String,
    branch: String,
}

#[derive(Debug, Clone)]
struct ImportGroup {
    repo_url: String,
    selections: Vec<GitHubSkillImportSelection>,
}

impl ImportGroup {
    fn selected_paths(&self) -> Vec<String> {
        self.selections
            .iter()
            .map(|selection| selection.source_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
struct RemoteCatalogEntry {
    valid_source_paths: HashSet<String>,
    invalid_candidates: HashMap<String, RemoteCatalogInvalidCandidate>,
    repo_error: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteCatalogInvalidCandidate {
    reason: String,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SkillManifestKey {
    id: String,
    source_path: String,
}

#[derive(Debug, Clone)]
struct PortabilityProgressUpdate<'a> {
    phase: SkillportStatePortabilityPhase,
    status: SkillportStatePortabilityStatus,
    total: usize,
    completed: usize,
    message: Option<&'a str>,
    current_item: Option<&'a str>,
    error: Option<&'a str>,
}

#[tauri::command]
pub async fn export_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    _options: Option<SkillportStateExportOptions>,
) -> Result<String, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
        PortabilityProgressUpdate {
            phase: SkillportStatePortabilityPhase::Exporting,
            status: SkillportStatePortabilityStatus::Running,
            total: 1,
            completed: 0,
            message: Some("Preparing portable SkillPort state export"),
            current_item: None,
            error: None,
        },
    );
    let result = export_skillport_state_impl(&state.db, Some(&app), Some(&cancel)).await;
    match &result {
        Ok(payload) => {
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Exporting,
                    status: SkillportStatePortabilityStatus::Completed,
                    total: 1,
                    completed: 1,
                    message: Some("Portable SkillPort state export completed"),
                    current_item: None,
                    error: None,
                },
            );
            let manifest = serde_json::from_str::<SkillportStateManifest>(payload).ok();
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.export",
                    "succeeded",
                    "Exported portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "githubSources": manifest.as_ref().map(|item| item.github_sources.len()),
                    "centralSkills": manifest.as_ref().map(|item| item.central_skills.len()),
                    "unrestorableSkills": manifest.as_ref().map(|item| item.unrestorable_skills.len()),
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Exporting,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.export",
                    "failed",
                    "Failed to export portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn preview_skillport_state_import(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<SkillportStateImportPreview, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
        PortabilityProgressUpdate {
            phase: SkillportStatePortabilityPhase::Previewing,
            status: SkillportStatePortabilityStatus::Running,
            total: 3,
            completed: 0,
            message: Some("Parsing SkillPort state JSON"),
            current_item: None,
            error: None,
        },
    );
    let result = match parse_manifest(&json) {
        Ok(manifest) => {
            match build_remote_catalog(&state.db, &manifest, Some(&app), Some(&cancel)).await {
                Ok(remote_catalog) => match preview_skillport_state_import_impl(
                    &state.db,
                    &manifest,
                    Some(&remote_catalog),
                    Some(&app),
                    Some(&cancel),
                )
                .await
                {
                    Ok(preview) => Ok(preview),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    };
    match &result {
        Ok(preview) => {
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Previewing,
                    status: SkillportStatePortabilityStatus::Completed,
                    total: 3,
                    completed: 3,
                    message: Some("SkillPort state import preview completed"),
                    current_item: None,
                    error: None,
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.preview_import",
                    "succeeded",
                    "Previewed portable SkillPort state import",
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "summary": &preview.summary,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Previewing,
                    status,
                    total: 3,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.preview_import",
                    "failed",
                    "Failed to preview portable SkillPort state import",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn import_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<SkillportStateImportResult, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
        PortabilityProgressUpdate {
            phase: SkillportStatePortabilityPhase::Importing,
            status: SkillportStatePortabilityStatus::Running,
            total: 1,
            completed: 0,
            message: Some("Preparing SkillPort state import"),
            current_item: None,
            error: None,
        },
    );
    let result = match parse_manifest(&json) {
        Ok(manifest) => {
            import_skillport_state_impl(
                &state.db,
                &manifest,
                resolutions,
                Some(&app),
                Some(&cancel),
            )
            .await
        }
        Err(error) => Err(error),
    };
    match &result {
        Ok(import_result) => {
            let status = if import_result.cancelled {
                "cancelled"
            } else {
                match (
                    import_result.imported_skills.len() + import_result.sources_added,
                    import_result.failed_skills.len(),
                ) {
                    (_, 0) => "succeeded",
                    (0, _) => "failed",
                    _ => "partial",
                }
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Importing,
                    status: if import_result.cancelled {
                        SkillportStatePortabilityStatus::Cancelled
                    } else if import_result.failed_skills.is_empty() {
                        SkillportStatePortabilityStatus::Completed
                    } else {
                        SkillportStatePortabilityStatus::Failed
                    },
                    total: import_result.imported_skills.len()
                        + import_result.failed_skills.len()
                        + import_result.skipped_skills.len(),
                    completed: import_result.imported_skills.len()
                        + import_result.failed_skills.len()
                        + import_result.skipped_skills.len(),
                    message: Some("SkillPort state import finished"),
                    current_item: None,
                    error: None,
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.import",
                    status,
                    format!(
                        "Imported {} skill(s), {} failed",
                        import_result.imported_skills.len(),
                        import_result.failed_skills.len()
                    ),
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "sourcesAdded": import_result.sources_added,
                    "sourcesSkipped": import_result.sources_skipped,
                    "importedSkills": &import_result.imported_skills,
                    "skippedSkills": &import_result.skipped_skills,
                    "failedSkills": &import_result.failed_skills,
                    "tagsRestored": import_result.tags_restored,
                    "cancelled": import_result.cancelled,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Importing,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.import",
                    "failed",
                    "Failed to import portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn cancel_skillport_state_portability(state: State<'_, AppState>) -> Result<(), String> {
    state.portable_state_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

async fn export_skillport_state_impl(
    pool: &DbPool,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<String, String> {
    check_cancel(cancel)?;
    let github_sources = export_github_sources(pool).await?;
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Exporting,
        3,
        1,
        Some("Collected GitHub sources"),
        None,
    );
    check_cancel(cancel)?;
    let skills = db::get_central_skills(pool).await?;
    let skill_ids = skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let total_export_steps = skill_ids.len() + 3;
    let mut assignments = db::get_skill_repository_assignments_for_skills(pool, &skill_ids).await?;
    let mut tags_by_skill = db::get_skill_tags_for_skills(pool, &skill_ids).await?;
    let unknown_repository = db::get_local_unknown_repository(pool).await?;
    let mut central_skills = Vec::new();
    let mut unrestorable_skills = Vec::new();
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Exporting,
        total_export_steps,
        2,
        Some("Loaded Central skill metadata"),
        None,
    );

    for (index, skill) in skills.into_iter().enumerate() {
        check_cancel(cancel)?;
        emit_portability_step(
            app,
            SkillportStatePortabilityPhase::Exporting,
            total_export_steps,
            index + 2,
            Some("Exporting Central skill"),
            Some(&skill.name),
        );
        let assignment =
            assignments
                .remove(&skill.id)
                .unwrap_or_else(|| db::SkillRepositoryAssignment {
                    repository: unknown_repository.clone(),
                    source_path: None,
                    is_source_unknown: true,
                });
        if let Some(source) = exportable_skill_source(&assignment) {
            central_skills.push(PortableCentralSkill {
                id: skill.id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                source,
                tags: tags_by_skill
                    .remove(&skill.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tag| PortableSkillTag {
                        name: tag.name,
                        description: tag.description,
                        color: tag.color,
                    })
                    .collect(),
            });
        } else {
            unrestorable_skills.push(PortableUnrestorableSkill {
                id: skill.id,
                name: skill.name,
                reason: "source_unknown".to_string(),
            });
        }
    }
    check_cancel(cancel)?;

    let manifest = SkillportStateManifest {
        kind: EXPORT_KIND.to_string(),
        version: EXPORT_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        exported_from: ExportedFrom {
            app: "SkillPort".to_string(),
        },
        github_sources,
        central_skills,
        unrestorable_skills,
    };

    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Finalizing,
        total_export_steps,
        total_export_steps,
        Some("Serializing SkillPort state JSON"),
        None,
    );
    serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())
}

async fn export_github_sources(pool: &DbPool) -> Result<Vec<PortableGithubSource>, String> {
    let registry_rows = sqlx::query(
        "SELECT url, is_enabled
         FROM skill_registries
         WHERE source_type = 'github'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let registry_enabled_by_identity = registry_rows
        .iter()
        .map(|row| {
            (
                normalize_registry_identity(row.get::<String, _>("url").as_str()),
                row.get::<bool, _>("is_enabled"),
            )
        })
        .collect::<HashMap<_, _>>();

    let rows = sqlx::query(
        "SELECT DISTINCT r.name, r.source_type, r.url
         FROM skill_repositories r
         JOIN skill_repository_members m ON m.repository_id = r.id
         JOIN skills s ON s.id = m.skill_id
         WHERE s.is_central = 1
           AND r.source_type = 'github'
           AND r.is_unknown = 0
           AND r.url IS NOT NULL
           AND TRIM(r.url) <> ''
         ORDER BY lower(r.name)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut seen = HashSet::new();
    let mut sources = Vec::with_capacity(rows.len());
    for row in rows {
        let url: String = row.get("url");
        let identity = normalize_registry_identity(&url);
        if !seen.insert(identity.clone()) {
            continue;
        }
        sources.push(PortableGithubSource {
            name: row.get("name"),
            source_type: row.get("source_type"),
            url,
            is_enabled: registry_enabled_by_identity
                .get(&identity)
                .copied()
                .unwrap_or(true),
        });
    }

    Ok(sources)
}

fn exportable_skill_source(
    assignment: &db::SkillRepositoryAssignment,
) -> Option<PortableCentralSkillSource> {
    let repository = &assignment.repository;
    if repository.source_type != "github" || repository.is_unknown {
        return None;
    }

    Some(PortableCentralSkillSource {
        source_type: "github".to_string(),
        owner: repository.owner.clone()?,
        repo: repository.repo.clone()?,
        branch: repository
            .branch
            .clone()
            .filter(|branch| !branch.trim().is_empty())
            .unwrap_or_else(|| "main".to_string()),
        url: repository.url.clone()?,
        source_path: export_source_path(assignment.source_path.as_deref()?),
    })
}

fn parse_manifest(json: &str) -> Result<SkillportStateManifest, String> {
    let manifest: SkillportStateManifest =
        serde_json::from_str(json).map_err(|e| format!("Invalid SkillPort state JSON: {e}"))?;
    if manifest.kind != EXPORT_KIND {
        return Err("Unsupported SkillPort state export kind".to_string());
    }
    if manifest.version != EXPORT_VERSION {
        return Err(format!(
            "Unsupported SkillPort state export version: {}",
            manifest.version
        ));
    }
    Ok(manifest)
}

async fn preview_skillport_state_import_impl(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    remote_catalog: Option<&HashMap<RepoKey, RemoteCatalogEntry>>,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<SkillportStateImportPreview, String> {
    check_cancel(cancel)?;
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Previewing,
        3,
        2,
        Some("Checking existing Central skills and sources"),
        None,
    );
    let existing_sources = existing_registry_identities(pool).await?;
    let existing_skills = db::get_skills_by_ids(
        pool,
        &manifest
            .central_skills
            .iter()
            .map(|skill| skill.id.clone())
            .collect::<Vec<_>>(),
    )
    .await?;
    let mut summary = SkillportStateImportPreviewSummary::default();
    let mut seen_source_identities = HashSet::new();
    let mut github_sources = Vec::new();
    for source in manifest
        .github_sources
        .iter()
        .filter(|source| source.source_type == "github")
    {
        check_cancel(cancel)?;
        let identity = normalize_registry_identity(&source.url);
        let status = if !seen_source_identities.insert(identity.clone()) {
            summary.sources_duplicate += 1;
            SourcePreviewStatus::Duplicate
        } else if existing_sources.contains(&identity) {
            summary.sources_existing += 1;
            SourcePreviewStatus::Exists
        } else {
            summary.sources_to_add += 1;
            SourcePreviewStatus::WillAdd
        };
        github_sources.push(SkillportStateSourcePreview {
            name: source.name.clone(),
            url: source.url.clone(),
            status,
        });
    }

    let mut skills =
        Vec::with_capacity(manifest.central_skills.len() + manifest.unrestorable_skills.len());
    let mut seen_skill_keys = HashSet::new();
    let mut seen_skill_ids = HashMap::<String, String>::new();
    for skill in &manifest.central_skills {
        check_cancel(cancel)?;
        let source_path = import_source_path(&skill.source.source_path);
        let key = SkillManifestKey {
            id: skill.id.clone(),
            source_path: source_path.clone(),
        };
        let id_duplicate_with_other_path = seen_skill_ids
            .get(&skill.id)
            .is_some_and(|previous_path| previous_path != &source_path);
        let (status, existing_skill_id, reason, detail) = if !seen_skill_keys.insert(key) {
            (
                SkillPreviewStatus::DuplicateSkipped,
                None,
                Some("duplicate_in_json".to_string()),
                Some("A skill with the same id and sourcePath already appeared earlier in this JSON.".to_string()),
            )
        } else if id_duplicate_with_other_path {
            (
                SkillPreviewStatus::Conflict,
                None,
                Some("duplicate_skill_id_different_source".to_string()),
                Some(
                    "The JSON contains the same skill id with different sourcePath values."
                        .to_string(),
                ),
            )
        } else if skill.source.source_type != "github" {
            (
                SkillPreviewStatus::Unrestorable,
                None,
                Some("source_unknown".to_string()),
                None,
            )
        } else if let Some((status, reason, detail)) =
            remote_catalog_issue(remote_catalog, &skill.source, &source_path)
        {
            (status, None, Some(reason), detail)
        } else if let Some(existing) = existing_skills.get(&skill.id) {
            if existing.is_central {
                (
                    SkillPreviewStatus::Conflict,
                    Some(existing.id.clone()),
                    Some("central_skill_exists".to_string()),
                    None,
                )
            } else {
                (
                    SkillPreviewStatus::Unrestorable,
                    Some(existing.id.clone()),
                    Some("non_central_conflict".to_string()),
                    None,
                )
            }
        } else {
            (SkillPreviewStatus::Ready, None, None, None)
        };
        seen_skill_ids
            .entry(skill.id.clone())
            .or_insert_with(|| source_path.clone());
        increment_skill_summary(&mut summary, &status);
        skills.push(SkillportStateSkillPreview {
            id: skill.id.clone(),
            name: skill.name.clone(),
            source_path: Some(skill.source.source_path.clone()),
            status,
            existing_skill_id,
            reason,
            detail,
        });
    }

    for skill in &manifest.unrestorable_skills {
        check_cancel(cancel)?;
        summary.unrestorable += 1;
        skills.push(SkillportStateSkillPreview {
            id: skill.id.clone(),
            name: skill.name.clone(),
            source_path: None,
            status: SkillPreviewStatus::Unrestorable,
            existing_skill_id: None,
            reason: Some(skill.reason.clone()),
            detail: None,
        });
    }
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Previewing,
        3,
        3,
        Some("Classified SkillPort state import preview"),
        None,
    );

    Ok(SkillportStateImportPreview {
        github_sources,
        skills,
        summary,
    })
}

fn increment_skill_summary(
    summary: &mut SkillportStateImportPreviewSummary,
    status: &SkillPreviewStatus,
) {
    match status {
        SkillPreviewStatus::Ready => summary.ready += 1,
        SkillPreviewStatus::Conflict => summary.conflicts += 1,
        SkillPreviewStatus::Missing => summary.missing += 1,
        SkillPreviewStatus::Unrestorable => summary.unrestorable += 1,
        SkillPreviewStatus::DuplicateSkipped => summary.duplicate_skipped += 1,
    }
}

async fn build_remote_catalog(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<HashMap<RepoKey, RemoteCatalogEntry>, String> {
    check_cancel(cancel)?;
    let auth = github_import::github_direct_auth_from_settings(pool).await?;
    let mut repo_urls = HashMap::<RepoKey, String>::new();
    for source in manifest.central_skills.iter().map(|skill| &skill.source) {
        if source.source_type != "github" {
            continue;
        }
        let key = repo_key(source);
        repo_urls
            .entry(key)
            .or_insert_with(|| repo_url_for_source(source));
    }
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Previewing,
        3,
        1,
        Some("Checking GitHub source catalogs"),
        None,
    );

    let entries = stream::iter(repo_urls.into_iter().map(|(key, repo_url)| {
        let auth = auth.clone();
        async move {
            let entry = match github_import::inspect_github_repo_skills_with_auth(
                &repo_url,
                auth.as_deref(),
            )
            .await
            {
                Ok(inspected) => RemoteCatalogEntry {
                    valid_source_paths: inspected
                        .valid_candidates
                        .into_iter()
                        .map(|candidate| import_source_path(&candidate.source_path))
                        .collect(),
                    invalid_candidates: inspected
                        .invalid_candidates
                        .into_iter()
                        .map(|candidate| {
                            (
                                import_source_path(&candidate.source_path),
                                RemoteCatalogInvalidCandidate {
                                    reason: candidate.reason,
                                    detail: candidate.detail,
                                },
                            )
                        })
                        .collect(),
                    repo_error: None,
                },
                Err(error) => RemoteCatalogEntry {
                    valid_source_paths: HashSet::new(),
                    invalid_candidates: HashMap::new(),
                    repo_error: Some(error),
                },
            };
            (key, entry)
        }
    }))
    .buffer_unordered(REMOTE_CATALOG_CONCURRENCY_LIMIT)
    .collect::<Vec<_>>()
    .await;
    check_cancel(cancel)?;

    Ok(entries.into_iter().collect())
}

fn remote_catalog_issue(
    remote_catalog: Option<&HashMap<RepoKey, RemoteCatalogEntry>>,
    source: &PortableCentralSkillSource,
    source_path: &str,
) -> Option<(SkillPreviewStatus, String, Option<String>)> {
    let catalog = remote_catalog?;
    let entry = catalog.get(&repo_key(source))?;
    if let Some(error) = &entry.repo_error {
        return Some((
            SkillPreviewStatus::Unrestorable,
            "repo_unavailable".to_string(),
            Some(error.clone()),
        ));
    }
    if let Some(invalid) = entry.invalid_candidates.get(source_path) {
        return Some((
            SkillPreviewStatus::Unrestorable,
            invalid.reason.clone(),
            Some(invalid.detail.clone()),
        ));
    }
    if entry.valid_source_paths.contains(source_path) {
        return None;
    }

    Some((
        SkillPreviewStatus::Missing,
        "source_missing".to_string(),
        None,
    ))
}

async fn import_skillport_state_impl(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    resolutions: Vec<SkillportStateImportResolution>,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<SkillportStateImportResult, String> {
    if check_cancel(cancel).is_err() {
        return Ok(cancelled_import_result(manifest, 0, 0));
    }
    let auth = github_import::github_direct_auth_from_settings(pool).await?;
    if check_cancel(cancel).is_err() {
        return Ok(cancelled_import_result(manifest, 0, 0));
    }
    let (sources_added, sources_skipped) =
        ensure_github_sources(pool, &manifest.github_sources).await?;
    emit_portability_step(
        app,
        SkillportStatePortabilityPhase::Importing,
        manifest.central_skills.len().max(1),
        0,
        Some("Registered GitHub sources"),
        None,
    );
    if check_cancel(cancel).is_err() {
        return Ok(cancelled_import_result(
            manifest,
            sources_added,
            sources_skipped,
        ));
    }
    let (groups, mut result) = build_import_groups(pool, manifest, resolutions).await?;
    result.sources_added = sources_added;
    result.sources_skipped = sources_skipped;
    let total_import_items = groups
        .iter()
        .map(|group| group.selections.len())
        .sum::<usize>()
        + result.skipped_skills.len()
        + result.failed_skills.len();
    let total_import_items = total_import_items.max(1);
    let mut completed_import_items = result.skipped_skills.len() + result.failed_skills.len();

    let skill_by_source_path = manifest
        .central_skills
        .iter()
        .map(|skill| (import_source_path(&skill.source.source_path), skill))
        .collect::<HashMap<_, _>>();

    for group_index in 0..groups.len() {
        let group = groups[group_index].clone();
        if check_cancel(cancel).is_err() {
            for pending_group in &groups[group_index..] {
                mark_group_cancelled(&mut result, pending_group, &skill_by_source_path);
            }
            result.cancelled = true;
            completed_import_items = total_import_items;
            emit_portability_step(
                app,
                SkillportStatePortabilityPhase::Importing,
                total_import_items,
                completed_import_items,
                Some("SkillPort state import cancelled"),
                None,
            );
            break;
        }

        let selected_paths = group.selected_paths();
        emit_portability_step(
            app,
            SkillportStatePortabilityPhase::Importing,
            total_import_items,
            completed_import_items,
            Some("Importing GitHub-backed skills"),
            Some(&group.repo_url),
        );
        match github_import::import_github_repo_skills_partially_with_auth(
            pool,
            &group.repo_url,
            group.selections,
            None,
            auth.as_deref(),
        )
        .await
        {
            Ok(imported) => {
                result.skipped_skills.extend(imported.skipped_skills);
                for skill in imported.imported_skills {
                    if let Some(exported) =
                        skill_by_source_path.get(&import_source_path(&skill.source_path))
                    {
                        result.tags_restored +=
                            restore_skill_tags(pool, &skill.imported_skill_id, &exported.tags)
                                .await?;
                    }
                    result.imported_skills.push(SkillportStateImportedSkill {
                        source_path: export_source_path(&skill.source_path),
                        imported_skill_id: skill.imported_skill_id,
                        skill_name: skill.skill_name,
                    });
                }
                for failure in imported.failed_skills {
                    let skill = skill_by_source_path.get(&failure.source_path);
                    result.failed_skills.push(SkillportStateImportFailure {
                        skill_id: skill
                            .map(|skill| skill.id.clone())
                            .unwrap_or_else(|| failure.source_path.clone()),
                        source_path: Some(export_source_path(&failure.source_path)),
                        error: failure.error,
                    });
                }
            }
            Err(error) => {
                for source_path in &selected_paths {
                    let skill = skill_by_source_path.get(source_path);
                    result.failed_skills.push(SkillportStateImportFailure {
                        skill_id: skill
                            .map(|skill| skill.id.clone())
                            .unwrap_or_else(|| source_path.to_string()),
                        source_path: Some(export_source_path(source_path)),
                        error: error.clone(),
                    });
                }
            }
        }
        completed_import_items =
            (completed_import_items + selected_paths.len()).min(total_import_items);
        emit_portability_step(
            app,
            SkillportStatePortabilityPhase::Importing,
            total_import_items,
            completed_import_items,
            Some("Imported GitHub-backed skill group"),
            Some(&group.repo_url),
        );
    }

    Ok(result)
}

fn cancelled_import_result(
    manifest: &SkillportStateManifest,
    sources_added: usize,
    sources_skipped: usize,
) -> SkillportStateImportResult {
    let mut seen = HashSet::new();
    SkillportStateImportResult {
        sources_added,
        sources_skipped,
        skipped_skills: manifest
            .central_skills
            .iter()
            .filter_map(|skill| {
                if seen.insert(SkillManifestKey {
                    id: skill.id.clone(),
                    source_path: import_source_path(&skill.source.source_path),
                }) {
                    Some(skill.id.clone())
                } else {
                    None
                }
            })
            .collect(),
        cancelled: true,
        ..SkillportStateImportResult::default()
    }
}

fn mark_group_cancelled(
    result: &mut SkillportStateImportResult,
    group: &ImportGroup,
    skill_by_source_path: &HashMap<String, &PortableCentralSkill>,
) {
    for source_path in group.selected_paths() {
        let skill = skill_by_source_path.get(&source_path);
        result.skipped_skills.push(
            skill
                .map(|skill| skill.id.clone())
                .unwrap_or_else(|| source_path.clone()),
        );
    }
}

async fn ensure_github_sources(
    pool: &DbPool,
    sources: &[PortableGithubSource],
) -> Result<(usize, usize), String> {
    let mut existing = existing_registry_identities(pool).await?;
    let mut seen_import_identities = HashSet::new();
    let mut added = 0;
    let mut skipped = 0;

    for source in sources
        .iter()
        .filter(|source| source.source_type == "github")
    {
        let identity = normalize_registry_identity(&source.url);
        if !seen_import_identities.insert(identity.clone()) || existing.contains(&identity) {
            skipped += 1;
            continue;
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO skill_registries
             (id, name, source_type, url, is_builtin, is_enabled, last_sync_status, created_at)
             VALUES (?, ?, ?, ?, 0, ?, 'never', ?)",
        )
        .bind(id)
        .bind(&source.name)
        .bind(&source.source_type)
        .bind(&source.url)
        .bind(source.is_enabled)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        existing.insert(identity);
        added += 1;
    }

    Ok((added, skipped))
}

async fn existing_registry_identities(pool: &DbPool) -> Result<HashSet<String>, String> {
    let rows = sqlx::query("SELECT url FROM skill_registries WHERE source_type = 'github'")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|row| normalize_registry_identity(row.get::<String, _>("url").as_str()))
        .collect())
}

async fn build_import_groups(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<(Vec<ImportGroup>, SkillportStateImportResult), String> {
    let resolution_map = resolutions
        .into_iter()
        .map(|resolution| {
            (
                resolution_key(&resolution.skill_id, resolution.source_path.as_deref()),
                resolution,
            )
        })
        .collect::<HashMap<_, _>>();
    let existing_skills = db::get_skills_by_ids(
        pool,
        &manifest
            .central_skills
            .iter()
            .filter(|skill| !resolution_map.contains_key(&skill.id))
            .map(|skill| skill.id.clone())
            .collect::<Vec<_>>(),
    )
    .await?;
    let mut grouped = HashMap::<RepoKey, ImportGroup>::new();
    let mut result = SkillportStateImportResult::default();
    let mut seen_skill_keys = HashSet::<SkillManifestKey>::new();
    let mut seen_skill_ids = HashMap::<String, String>::new();

    for skill in &manifest.central_skills {
        if skill.source.source_type != "github" {
            result.failed_skills.push(SkillportStateImportFailure {
                skill_id: skill.id.clone(),
                source_path: Some(skill.source.source_path.clone()),
                error: "Only GitHub-backed central skills can be restored".to_string(),
            });
            continue;
        }

        let source_path = import_source_path(&skill.source.source_path);
        let key = SkillManifestKey {
            id: skill.id.clone(),
            source_path: source_path.clone(),
        };
        if !seen_skill_keys.insert(key) {
            result.skipped_skills.push(skill.id.clone());
            continue;
        }
        let duplicate_id_with_other_path = seen_skill_ids
            .get(&skill.id)
            .is_some_and(|previous_path| previous_path != &source_path);
        seen_skill_ids
            .entry(skill.id.clone())
            .or_insert_with(|| source_path.clone());
        let resolution = resolution_for_skill(skill, &resolution_map, &existing_skills);
        if duplicate_id_with_other_path && resolution.resolution != DuplicateResolution::Rename {
            result.skipped_skills.push(skill.id.clone());
            continue;
        }
        if resolution.resolution == DuplicateResolution::Skip {
            result.skipped_skills.push(skill.id.clone());
            continue;
        }

        let group = grouped
            .entry(repo_key(&skill.source))
            .or_insert_with(|| ImportGroup {
                repo_url: repo_url_for_source(&skill.source),
                selections: Vec::new(),
            });
        group.selections.push(GitHubSkillImportSelection {
            source_path,
            resolution: resolution.resolution,
            renamed_skill_id: resolution.renamed_skill_id,
        });
    }

    Ok((grouped.into_values().collect(), result))
}

fn resolution_for_skill(
    skill: &PortableCentralSkill,
    resolutions: &HashMap<String, SkillportStateImportResolution>,
    existing_skills: &HashMap<String, db::Skill>,
) -> SkillportStateImportResolution {
    if let Some(resolution) = resolutions
        .get(&resolution_key(&skill.id, Some(&skill.source.source_path)))
        .or_else(|| resolutions.get(&resolution_key(&skill.id, None)))
    {
        return resolution.clone();
    }

    let resolution = if existing_skills.contains_key(&skill.id) {
        DuplicateResolution::Skip
    } else {
        DuplicateResolution::Overwrite
    };

    SkillportStateImportResolution {
        skill_id: skill.id.clone(),
        source_path: Some(skill.source.source_path.clone()),
        resolution,
        renamed_skill_id: None,
    }
}

fn resolution_key(skill_id: &str, source_path: Option<&str>) -> String {
    format!(
        "{}\u{1f}{}",
        skill_id,
        source_path.map(import_source_path).unwrap_or_default()
    )
}

async fn restore_skill_tags(
    pool: &DbPool,
    skill_id: &str,
    tags: &[PortableSkillTag],
) -> Result<usize, String> {
    let mut tag_ids = Vec::new();
    for tag in tags {
        let created = db::create_skill_tag(
            pool,
            &tag.name,
            tag.description.as_deref(),
            tag.color.as_deref(),
        )
        .await?;
        tag_ids.push(created.id);
    }

    if tag_ids.is_empty() {
        return Ok(0);
    }

    db::assign_skill_tags(
        pool,
        &[skill_id.to_string()],
        &tag_ids,
        "manual",
        None,
        None,
    )
    .await?;
    Ok(tag_ids.len())
}

fn repo_key(source: &PortableCentralSkillSource) -> RepoKey {
    RepoKey {
        owner: source.owner.to_ascii_lowercase(),
        repo: source.repo.to_ascii_lowercase(),
        branch: source.branch.to_ascii_lowercase(),
    }
}

fn repo_url_for_source(source: &PortableCentralSkillSource) -> String {
    if !source.url.trim().is_empty() && source.url.contains("/tree/") {
        source.url.clone()
    } else {
        format!(
            "https://github.com/{}/{}/tree/{}",
            source.owner, source.repo, source.branch
        )
    }
}

fn normalize_registry_identity(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let without_www = without_scheme
        .strip_prefix("www.")
        .unwrap_or(without_scheme);
    let lower = without_www.to_ascii_lowercase();
    if let Some(path) = lower.strip_prefix("github.com/") {
        let mut parts = path.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            return format!("github:{owner}/{repo}");
        }
    }
    lower
}

fn export_source_path(source_path: &str) -> String {
    let normalized = import_source_path(source_path);
    if normalized == "." {
        "SKILL.md".to_string()
    } else if normalized.to_ascii_lowercase().ends_with("/skill.md") {
        normalized
    } else {
        format!("{normalized}/SKILL.md")
    }
}

fn import_source_path(source_path: &str) -> String {
    let normalized = source_path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("SKILL.md") {
        return ".".to_string();
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.ends_with("/skill.md") {
        normalized[..normalized.len() - "/SKILL.md".len()].to_string()
    } else {
        normalized
    }
}

fn check_cancel(cancel: Option<&CancelFlag>) -> Result<(), String> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::SeqCst)) {
        Err(PORTABILITY_CANCELLED_MESSAGE.to_string())
    } else {
        Ok(())
    }
}

fn is_cancelled_error(error: &str) -> bool {
    error.contains(PORTABILITY_CANCELLED_MESSAGE) || error == STATUS_CANCELLED
}

fn emit_portability_step(
    app: Option<&AppHandle>,
    phase: SkillportStatePortabilityPhase,
    total: usize,
    completed: usize,
    message: Option<&str>,
    current_item: Option<&str>,
) {
    if let Some(app) = app {
        emit_portability_progress(
            app,
            PortabilityProgressUpdate {
                phase,
                status: SkillportStatePortabilityStatus::Running,
                total,
                completed,
                message,
                current_item,
                error: None,
            },
        );
    }
}

fn emit_portability_progress(app: &AppHandle, update: PortabilityProgressUpdate<'_>) {
    let payload = SkillportStatePortabilityProgressPayload {
        phase: update.phase,
        status: update.status,
        total: update.total,
        completed: update.completed,
        message: update.message.map(str::to_string),
        current_item: update.current_item.map(str::to_string),
        error: update.error.map(str::to_string),
    };
    let _ = app.emit(PORTABILITY_PROGRESS_EVENT, payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Skill;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    fn github_source(path: &str) -> PortableCentralSkillSource {
        github_source_for_repo("openai", "skills", "main", path)
    }

    fn github_source_for_repo(
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> PortableCentralSkillSource {
        PortableCentralSkillSource {
            source_type: "github".to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            url: format!("https://github.com/{owner}/{repo}"),
            source_path: path.to_string(),
        }
    }

    fn manifest_with_skill(id: &str, path: &str) -> SkillportStateManifest {
        SkillportStateManifest {
            kind: EXPORT_KIND.to_string(),
            version: EXPORT_VERSION,
            exported_at: "2026-04-25T00:00:00Z".to_string(),
            exported_from: ExportedFrom {
                app: "SkillPort".to_string(),
            },
            github_sources: vec![PortableGithubSource {
                name: "OpenAI Skills".to_string(),
                source_type: "github".to_string(),
                url: "https://github.com/openai/skills".to_string(),
                is_enabled: true,
            }],
            central_skills: vec![PortableCentralSkill {
                id: id.to_string(),
                name: id.to_string(),
                description: Some("demo".to_string()),
                source: github_source(path),
                tags: vec![PortableSkillTag {
                    name: "Docs".to_string(),
                    description: None,
                    color: Some("#111111".to_string()),
                }],
            }],
            unrestorable_skills: Vec::new(),
        }
    }

    #[tokio::test]
    async fn export_empty_state_produces_manifest() {
        let pool = setup_test_db().await;
        let json = export_skillport_state_impl(&pool, None, None)
            .await
            .unwrap();
        let manifest = parse_manifest(&json).unwrap();
        assert_eq!(manifest.kind, EXPORT_KIND);
        assert_eq!(manifest.version, EXPORT_VERSION);
        assert!(manifest.github_sources.is_empty());
    }

    #[tokio::test]
    async fn export_includes_github_skill_and_unrestorable_local_skill() {
        let pool = setup_test_db().await;
        let github = Skill {
            id: "openai-docs".to_string(),
            name: "openai-docs".to_string(),
            description: Some("docs".to_string()),
            file_path: "/tmp/openai-docs/SKILL.md".to_string(),
            canonical_path: Some("/tmp/openai-docs".to_string()),
            is_central: true,
            source: Some("github:openai/skills".to_string()),
            content: None,
            scanned_at: "2026-04-25T00:00:00Z".to_string(),
        };
        db::upsert_skill(&pool, &github).await.unwrap();
        db::assign_github_repository_to_skill(
            &pool,
            "openai",
            "skills",
            "main",
            "https://github.com/openai/skills",
            "openai-docs",
            "skills/openai-docs",
        )
        .await
        .unwrap();
        let tag = db::create_skill_tag(&pool, "Docs", None, Some("#111111"))
            .await
            .unwrap();
        db::assign_skill_tags(
            &pool,
            &["openai-docs".to_string()],
            &[tag.id],
            "manual",
            None,
            None,
        )
        .await
        .unwrap();
        let local = Skill {
            id: "local-skill".to_string(),
            name: "local-skill".to_string(),
            description: None,
            file_path: "/tmp/local-skill/SKILL.md".to_string(),
            canonical_path: Some("/tmp/local-skill".to_string()),
            is_central: true,
            source: None,
            content: None,
            scanned_at: "2026-04-25T00:00:00Z".to_string(),
        };
        db::upsert_skill(&pool, &local).await.unwrap();

        let manifest = parse_manifest(
            &export_skillport_state_impl(&pool, None, None)
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(manifest.central_skills.len(), 1);
        assert_eq!(manifest.github_sources.len(), 1);
        assert_eq!(manifest.github_sources[0].name, "openai/skills");
        assert_eq!(
            manifest.github_sources[0].url,
            "https://github.com/openai/skills"
        );
        assert_eq!(
            manifest.central_skills[0].source.source_path,
            "skills/openai-docs/SKILL.md"
        );
        assert_eq!(manifest.central_skills[0].tags[0].name, "Docs");
        assert_eq!(manifest.unrestorable_skills.len(), 1);
    }

    #[tokio::test]
    async fn export_counts_distinct_github_repositories_backing_central_skills() {
        let pool = setup_test_db().await;
        for (id, name) in [
            ("alpha-one", "Alpha One"),
            ("alpha-two", "Alpha Two"),
            ("beta-one", "Beta One"),
        ] {
            db::upsert_skill(
                &pool,
                &Skill {
                    id: id.to_string(),
                    name: name.to_string(),
                    description: None,
                    file_path: format!("/tmp/{id}/SKILL.md"),
                    canonical_path: Some(format!("/tmp/{id}")),
                    is_central: true,
                    source: Some("github:test/source".to_string()),
                    content: None,
                    scanned_at: "2026-04-25T00:00:00Z".to_string(),
                },
            )
            .await
            .unwrap();
        }
        db::assign_github_repository_to_skill(
            &pool,
            "example",
            "alpha-skills",
            "main",
            "https://github.com/example/alpha-skills",
            "alpha-one",
            "skills/alpha-one",
        )
        .await
        .unwrap();
        db::assign_github_repository_to_skill(
            &pool,
            "example",
            "alpha-skills",
            "main",
            "https://github.com/example/alpha-skills",
            "alpha-two",
            "skills/alpha-two",
        )
        .await
        .unwrap();
        db::assign_github_repository_to_skill(
            &pool,
            "example",
            "beta-skills",
            "main",
            "https://github.com/example/beta-skills",
            "beta-one",
            "skills/beta-one",
        )
        .await
        .unwrap();

        let manifest = parse_manifest(
            &export_skillport_state_impl(&pool, None, None)
                .await
                .unwrap(),
        )
        .unwrap();
        let urls = manifest
            .github_sources
            .iter()
            .map(|source| source.url.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            urls,
            vec![
                "https://github.com/example/alpha-skills",
                "https://github.com/example/beta-skills",
            ]
        );
        assert_eq!(manifest.central_skills.len(), 3);
    }

    #[test]
    fn parse_manifest_rejects_invalid_kind_and_version() {
        let invalid_kind = r#"{"kind":"other","version":1,"exportedAt":"","exportedFrom":{"app":"SkillPort"},"githubSources":[],"centralSkills":[],"unrestorableSkills":[]}"#;
        assert!(parse_manifest(invalid_kind).unwrap_err().contains("kind"));

        let invalid_version = r#"{"kind":"skillport/state-export","version":2,"exportedAt":"","exportedFrom":{"app":"SkillPort"},"githubSources":[],"centralSkills":[],"unrestorableSkills":[]}"#;
        assert!(parse_manifest(invalid_version)
            .unwrap_err()
            .contains("version"));
    }

    #[tokio::test]
    async fn ensure_github_sources_skips_duplicates() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("openai-docs", "skills/openai-docs/SKILL.md");
        manifest.github_sources[0].url = "https://github.com/example/portable-skills".to_string();

        let first = ensure_github_sources(&pool, &manifest.github_sources)
            .await
            .unwrap();
        let second = ensure_github_sources(&pool, &manifest.github_sources)
            .await
            .unwrap();

        assert_eq!(first, (1, 0));
        assert_eq!(second, (0, 1));
    }

    #[tokio::test]
    async fn ensure_github_sources_skips_duplicate_sources_in_same_manifest() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("openai-docs", "skills/openai-docs/SKILL.md");
        manifest.github_sources[0].url = "https://github.com/example/portable-skills".to_string();
        let mut duplicate_source = manifest.github_sources[0].clone();
        duplicate_source.name = "OpenAI Skills Duplicate".to_string();
        duplicate_source.url = "https://github.com/example/portable-skills.git".to_string();
        manifest.github_sources.push(duplicate_source);

        let result = ensure_github_sources(&pool, &manifest.github_sources)
            .await
            .unwrap();

        assert_eq!(result, (1, 1));
    }

    #[tokio::test]
    async fn preview_reports_ready_conflict_missing_and_unrestorable() {
        let pool = setup_test_db().await;
        let existing = Skill {
            id: "conflict-skill".to_string(),
            name: "conflict-skill".to_string(),
            description: None,
            file_path: "/tmp/conflict-skill/SKILL.md".to_string(),
            canonical_path: Some("/tmp/conflict-skill".to_string()),
            is_central: true,
            source: None,
            content: None,
            scanned_at: "2026-04-25T00:00:00Z".to_string(),
        };
        db::upsert_skill(&pool, &existing).await.unwrap();

        let mut manifest = manifest_with_skill("ready-skill", "skills/ready-skill/SKILL.md");
        manifest.central_skills.push(PortableCentralSkill {
            id: "conflict-skill".to_string(),
            name: "conflict-skill".to_string(),
            description: None,
            source: github_source("skills/conflict-skill/SKILL.md"),
            tags: Vec::new(),
        });
        manifest.central_skills.push(PortableCentralSkill {
            id: "missing-skill".to_string(),
            name: "missing-skill".to_string(),
            description: None,
            source: github_source("skills/missing-skill/SKILL.md"),
            tags: Vec::new(),
        });
        manifest
            .unrestorable_skills
            .push(PortableUnrestorableSkill {
                id: "local-only".to_string(),
                name: "local-only".to_string(),
                reason: "source_unknown".to_string(),
            });

        let mut paths = HashSet::new();
        paths.insert("skills/ready-skill".to_string());
        paths.insert("skills/conflict-skill".to_string());
        let mut catalog = HashMap::new();
        catalog.insert(
            repo_key(&github_source("skills/ready-skill/SKILL.md")),
            RemoteCatalogEntry {
                valid_source_paths: paths,
                invalid_candidates: HashMap::new(),
                repo_error: None,
            },
        );

        let preview =
            preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
                .await
                .unwrap();

        assert_eq!(preview.summary.ready, 1);
        assert_eq!(preview.summary.conflicts, 1);
        assert_eq!(preview.summary.missing, 1);
        assert_eq!(preview.summary.unrestorable, 1);
    }

    #[tokio::test]
    async fn preview_reports_internal_duplicate_skills_and_sources() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
        manifest.github_sources.push(PortableGithubSource {
            name: "OpenAI Skills Duplicate".to_string(),
            source_type: "github".to_string(),
            url: "https://github.com/openai/skills.git".to_string(),
            is_enabled: true,
        });
        manifest
            .central_skills
            .push(manifest.central_skills[0].clone());
        manifest.central_skills.push(PortableCentralSkill {
            id: "dup-skill".to_string(),
            name: "dup-skill-alt".to_string(),
            description: None,
            source: github_source("skills/dup-skill-alt/SKILL.md"),
            tags: Vec::new(),
        });

        let mut paths = HashSet::new();
        paths.insert("skills/dup-skill".to_string());
        paths.insert("skills/dup-skill-alt".to_string());
        let mut catalog = HashMap::new();
        catalog.insert(
            repo_key(&github_source("skills/dup-skill/SKILL.md")),
            RemoteCatalogEntry {
                valid_source_paths: paths,
                invalid_candidates: HashMap::new(),
                repo_error: None,
            },
        );

        let preview =
            preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
                .await
                .unwrap();

        assert_eq!(preview.summary.sources_duplicate, 1);
        assert_eq!(preview.summary.duplicate_skipped, 1);
        assert_eq!(preview.summary.conflicts, 1);
        assert!(preview.skills.iter().any(|skill| {
            skill.status == SkillPreviewStatus::DuplicateSkipped
                && skill.reason.as_deref() == Some("duplicate_in_json")
        }));
        assert!(preview.skills.iter().any(|skill| {
            skill.status == SkillPreviewStatus::Conflict
                && skill.reason.as_deref() == Some("duplicate_skill_id_different_source")
        }));
    }

    #[tokio::test]
    async fn preview_reports_invalid_remote_skill_and_repo_unavailable_as_unrestorable() {
        let pool = setup_test_db().await;
        let invalid_source = github_source_for_repo(
            "openai",
            "skills",
            "main",
            "skills/bad-frontmatter/SKILL.md",
        );
        let repo_error_source =
            github_source_for_repo("other", "skills", "main", "skills/network-error/SKILL.md");
        let manifest = SkillportStateManifest {
            kind: EXPORT_KIND.to_string(),
            version: EXPORT_VERSION,
            exported_at: "2026-04-25T00:00:00Z".to_string(),
            exported_from: ExportedFrom {
                app: "SkillPort".to_string(),
            },
            github_sources: vec![],
            central_skills: vec![
                PortableCentralSkill {
                    id: "bad-frontmatter".to_string(),
                    name: "bad-frontmatter".to_string(),
                    description: None,
                    source: invalid_source.clone(),
                    tags: Vec::new(),
                },
                PortableCentralSkill {
                    id: "network-error".to_string(),
                    name: "network-error".to_string(),
                    description: None,
                    source: repo_error_source.clone(),
                    tags: Vec::new(),
                },
            ],
            unrestorable_skills: Vec::new(),
        };

        let mut catalog = HashMap::new();
        catalog.insert(
            repo_key(&invalid_source),
            RemoteCatalogEntry {
                valid_source_paths: HashSet::new(),
                invalid_candidates: HashMap::from([(
                    "skills/bad-frontmatter".to_string(),
                    RemoteCatalogInvalidCandidate {
                        reason: "invalid_frontmatter".to_string(),
                        detail: "Skill 'skills/bad-frontmatter' is missing valid frontmatter."
                            .to_string(),
                    },
                )]),
                repo_error: None,
            },
        );
        catalog.insert(
            repo_key(&repo_error_source),
            RemoteCatalogEntry {
                valid_source_paths: HashSet::new(),
                invalid_candidates: HashMap::new(),
                repo_error: Some("GitHub rate limit was exceeded".to_string()),
            },
        );

        let preview =
            preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog), None, None)
                .await
                .unwrap();

        let invalid = preview
            .skills
            .iter()
            .find(|skill| skill.id == "bad-frontmatter")
            .expect("invalid skill");
        assert_eq!(invalid.status, SkillPreviewStatus::Unrestorable);
        assert_eq!(invalid.reason.as_deref(), Some("invalid_frontmatter"));
        assert_eq!(
            invalid.detail.as_deref(),
            Some("Skill 'skills/bad-frontmatter' is missing valid frontmatter.")
        );

        let repo_failure = preview
            .skills
            .iter()
            .find(|skill| skill.id == "network-error")
            .expect("repo failure skill");
        assert_eq!(repo_failure.status, SkillPreviewStatus::Unrestorable);
        assert_eq!(repo_failure.reason.as_deref(), Some("repo_unavailable"));
        assert_eq!(
            repo_failure.detail.as_deref(),
            Some("GitHub rate limit was exceeded")
        );
        assert_eq!(preview.summary.unrestorable, 2);
    }

    #[tokio::test]
    async fn build_import_groups_applies_skip_overwrite_and_rename() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("new-skill", "skills/new-skill/SKILL.md");
        manifest.central_skills.push(PortableCentralSkill {
            id: "renamed-skill".to_string(),
            name: "renamed-skill".to_string(),
            description: None,
            source: github_source("skills/renamed-skill/SKILL.md"),
            tags: Vec::new(),
        });
        manifest.central_skills.push(PortableCentralSkill {
            id: "skipped-skill".to_string(),
            name: "skipped-skill".to_string(),
            description: None,
            source: github_source("skills/skipped-skill/SKILL.md"),
            tags: Vec::new(),
        });

        let (groups, result) = build_import_groups(
            &pool,
            &manifest,
            vec![
                SkillportStateImportResolution {
                    skill_id: "renamed-skill".to_string(),
                    source_path: None,
                    resolution: DuplicateResolution::Rename,
                    renamed_skill_id: Some("renamed-skill-copy".to_string()),
                },
                SkillportStateImportResolution {
                    skill_id: "skipped-skill".to_string(),
                    source_path: None,
                    resolution: DuplicateResolution::Skip,
                    renamed_skill_id: None,
                },
            ],
        )
        .await
        .unwrap();

        assert_eq!(result.skipped_skills, vec!["skipped-skill"]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].selections.len(), 2);
        assert_eq!(
            groups[0].selections[0].resolution,
            DuplicateResolution::Overwrite
        );
        assert_eq!(
            groups[0].selections[1].resolution,
            DuplicateResolution::Rename
        );
    }

    #[tokio::test]
    async fn build_import_groups_skips_exact_duplicate_entries() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
        manifest
            .central_skills
            .push(manifest.central_skills[0].clone());

        let (groups, result) = build_import_groups(&pool, &manifest, Vec::new())
            .await
            .unwrap();

        assert_eq!(result.skipped_skills, vec!["dup-skill"]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].selections.len(), 1);
        assert_eq!(groups[0].selections[0].source_path, "skills/dup-skill");
    }

    #[tokio::test]
    async fn build_import_groups_requires_resolution_for_duplicate_id_with_different_source() {
        let pool = setup_test_db().await;
        let mut manifest = manifest_with_skill("dup-skill", "skills/dup-skill/SKILL.md");
        manifest.central_skills.push(PortableCentralSkill {
            id: "dup-skill".to_string(),
            name: "dup-skill-alt".to_string(),
            description: None,
            source: github_source("skills/dup-skill-alt/SKILL.md"),
            tags: Vec::new(),
        });

        let (groups, result) = build_import_groups(&pool, &manifest, Vec::new())
            .await
            .unwrap();

        assert_eq!(result.skipped_skills, vec!["dup-skill"]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].selections.len(), 1);
        assert_eq!(groups[0].selections[0].source_path, "skills/dup-skill");

        let (groups, result) = build_import_groups(
            &pool,
            &manifest,
            vec![SkillportStateImportResolution {
                skill_id: "dup-skill".to_string(),
                source_path: Some("skills/dup-skill-alt/SKILL.md".to_string()),
                resolution: DuplicateResolution::Rename,
                renamed_skill_id: Some("dup-skill-alt-copy".to_string()),
            }],
        )
        .await
        .unwrap();

        assert!(result.skipped_skills.is_empty());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].selections.len(), 2);
        assert_eq!(
            groups[0].selections[1].renamed_skill_id.as_deref(),
            Some("dup-skill-alt-copy")
        );
    }

    #[tokio::test]
    async fn import_cancelled_before_groups_returns_partial_cancelled_result() {
        let pool = setup_test_db().await;
        let manifest = manifest_with_skill("cancelled-skill", "skills/cancelled-skill/SKILL.md");
        let cancel = Arc::new(AtomicBool::new(true));

        let result = import_skillport_state_impl(&pool, &manifest, Vec::new(), None, Some(&cancel))
            .await
            .unwrap();

        assert!(result.cancelled);
        assert_eq!(result.skipped_skills, vec!["cancelled-skill"]);
        assert!(result.failed_skills.is_empty());
    }

    #[tokio::test]
    async fn restore_skill_tags_creates_and_assigns_tags() {
        let pool = setup_test_db().await;
        let skill = Skill {
            id: "tagged".to_string(),
            name: "tagged".to_string(),
            description: None,
            file_path: "/tmp/tagged/SKILL.md".to_string(),
            canonical_path: Some("/tmp/tagged".to_string()),
            is_central: true,
            source: None,
            content: None,
            scanned_at: "2026-04-25T00:00:00Z".to_string(),
        };
        db::upsert_skill(&pool, &skill).await.unwrap();

        let count = restore_skill_tags(
            &pool,
            "tagged",
            &[PortableSkillTag {
                name: "Portable".to_string(),
                description: None,
                color: None,
            }],
        )
        .await
        .unwrap();
        let tags = db::get_skill_tags_for_skill(&pool, "tagged").await.unwrap();

        assert_eq!(count, 1);
        assert!(tags.iter().any(|tag| tag.name == "Portable"));
    }
}
