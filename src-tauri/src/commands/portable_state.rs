use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tauri::State;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillPreviewStatus {
    Ready,
    Conflict,
    Missing,
    Unrestorable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportPreviewSummary {
    pub sources_to_add: usize,
    pub sources_existing: usize,
    pub ready: usize,
    pub conflicts: usize,
    pub missing: usize,
    pub unrestorable: usize,
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

#[tauri::command]
pub async fn export_skillport_state(
    state: State<'_, AppState>,
    _options: Option<SkillportStateExportOptions>,
) -> Result<String, String> {
    let started_at = Instant::now();
    let result = export_skillport_state_impl(&state.db).await;
    match &result {
        Ok(payload) => {
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
    state: State<'_, AppState>,
    json: String,
) -> Result<SkillportStateImportPreview, String> {
    let started_at = Instant::now();
    let result = match parse_manifest(&json) {
        Ok(manifest) => match build_remote_catalog(&state.db, &manifest).await {
            Ok(remote_catalog) => {
                preview_skillport_state_import_impl(&state.db, &manifest, Some(&remote_catalog))
                    .await
            }
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    match &result {
        Ok(preview) => {
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
    state: State<'_, AppState>,
    json: String,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<SkillportStateImportResult, String> {
    let started_at = Instant::now();
    let result = match parse_manifest(&json) {
        Ok(manifest) => import_skillport_state_impl(&state.db, &manifest, resolutions).await,
        Err(error) => Err(error),
    };
    match &result {
        Ok(import_result) => {
            let status = match (
                import_result.imported_skills.len() + import_result.sources_added,
                import_result.failed_skills.len(),
            ) {
                (_, 0) => "succeeded",
                (0, _) => "failed",
                _ => "partial",
            };
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
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
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

async fn export_skillport_state_impl(pool: &DbPool) -> Result<String, String> {
    let github_sources = export_github_sources(pool).await?;
    let skills = db::get_central_skills(pool).await?;
    let mut central_skills = Vec::new();
    let mut unrestorable_skills = Vec::new();

    for skill in skills {
        let assignment = db::get_skill_repository_assignment(pool, &skill.id).await?;
        if let Some(source) = exportable_skill_source(&assignment) {
            central_skills.push(PortableCentralSkill {
                id: skill.id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                source,
                tags: export_skill_tags(pool, &skill.id).await?,
            });
        } else {
            unrestorable_skills.push(PortableUnrestorableSkill {
                id: skill.id,
                name: skill.name,
                reason: "source_unknown".to_string(),
            });
        }
    }

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

async fn export_skill_tags(pool: &DbPool, skill_id: &str) -> Result<Vec<PortableSkillTag>, String> {
    let tags = db::get_skill_tags_for_skill(pool, skill_id).await?;
    Ok(tags
        .into_iter()
        .map(|tag| PortableSkillTag {
            name: tag.name,
            description: tag.description,
            color: tag.color,
        })
        .collect())
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
    remote_catalog: Option<&HashMap<RepoKey, HashSet<String>>>,
) -> Result<SkillportStateImportPreview, String> {
    let existing_sources = existing_registry_identities(pool).await?;
    let mut summary = SkillportStateImportPreviewSummary::default();
    let github_sources = manifest
        .github_sources
        .iter()
        .filter(|source| source.source_type == "github")
        .map(|source| {
            let identity = normalize_registry_identity(&source.url);
            let status = if existing_sources.contains(&identity) {
                summary.sources_existing += 1;
                SourcePreviewStatus::Exists
            } else {
                summary.sources_to_add += 1;
                SourcePreviewStatus::WillAdd
            };
            SkillportStateSourcePreview {
                name: source.name.clone(),
                url: source.url.clone(),
                status,
            }
        })
        .collect();

    let mut skills =
        Vec::with_capacity(manifest.central_skills.len() + manifest.unrestorable_skills.len());
    for skill in &manifest.central_skills {
        let source_path = import_source_path(&skill.source.source_path);
        let (status, existing_skill_id, reason) = if skill.source.source_type != "github" {
            (
                SkillPreviewStatus::Unrestorable,
                None,
                Some("source_unknown".to_string()),
            )
        } else if !remote_catalog_contains(remote_catalog, &skill.source, &source_path) {
            (
                SkillPreviewStatus::Missing,
                None,
                Some("source_missing".to_string()),
            )
        } else if let Some(existing) = db::get_skill_by_id(pool, &skill.id).await? {
            if existing.is_central {
                (
                    SkillPreviewStatus::Conflict,
                    Some(existing.id),
                    Some("central_skill_exists".to_string()),
                )
            } else {
                (
                    SkillPreviewStatus::Unrestorable,
                    Some(existing.id),
                    Some("non_central_conflict".to_string()),
                )
            }
        } else {
            (SkillPreviewStatus::Ready, None, None)
        };
        increment_skill_summary(&mut summary, &status);
        skills.push(SkillportStateSkillPreview {
            id: skill.id.clone(),
            name: skill.name.clone(),
            source_path: Some(skill.source.source_path.clone()),
            status,
            existing_skill_id,
            reason,
        });
    }

    for skill in &manifest.unrestorable_skills {
        summary.unrestorable += 1;
        skills.push(SkillportStateSkillPreview {
            id: skill.id.clone(),
            name: skill.name.clone(),
            source_path: None,
            status: SkillPreviewStatus::Unrestorable,
            existing_skill_id: None,
            reason: Some(skill.reason.clone()),
        });
    }

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
    }
}

async fn build_remote_catalog(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
) -> Result<HashMap<RepoKey, HashSet<String>>, String> {
    let mut catalog = HashMap::new();
    for source in manifest.central_skills.iter().map(|skill| &skill.source) {
        if source.source_type != "github" {
            continue;
        }
        let key = repo_key(source);
        if catalog.contains_key(&key) {
            continue;
        }
        let preview =
            github_import::preview_github_repo_import_impl(pool, &repo_url_for_source(source))
                .await?;
        let source_paths = preview
            .skills
            .into_iter()
            .map(|skill| import_source_path(&skill.source_path))
            .collect::<HashSet<_>>();
        catalog.insert(key, source_paths);
    }
    Ok(catalog)
}

fn remote_catalog_contains(
    remote_catalog: Option<&HashMap<RepoKey, HashSet<String>>>,
    source: &PortableCentralSkillSource,
    source_path: &str,
) -> bool {
    let Some(catalog) = remote_catalog else {
        return true;
    };
    catalog
        .get(&repo_key(source))
        .map(|paths| paths.contains(source_path))
        .unwrap_or(false)
}

async fn import_skillport_state_impl(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<SkillportStateImportResult, String> {
    let (sources_added, sources_skipped) =
        ensure_github_sources(pool, &manifest.github_sources).await?;
    let (groups, mut result) = build_import_groups(pool, manifest, resolutions).await?;
    result.sources_added = sources_added;
    result.sources_skipped = sources_skipped;

    let skill_by_source_path = manifest
        .central_skills
        .iter()
        .map(|skill| (import_source_path(&skill.source.source_path), skill))
        .collect::<HashMap<_, _>>();

    for group in groups {
        let selected_paths = group
            .selections
            .iter()
            .map(|selection| selection.source_path.clone())
            .collect::<Vec<_>>();
        match github_import::import_github_repo_skills_impl(
            pool,
            &group.repo_url,
            group.selections,
            None,
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
                        source_path: skill.source_path,
                        imported_skill_id: skill.imported_skill_id,
                        skill_name: skill.skill_name,
                    });
                }
            }
            Err(error) => {
                for source_path in selected_paths {
                    let skill = skill_by_source_path.get(&source_path);
                    result.failed_skills.push(SkillportStateImportFailure {
                        skill_id: skill
                            .map(|skill| skill.id.clone())
                            .unwrap_or_else(|| source_path.clone()),
                        source_path: Some(export_source_path(&source_path)),
                        error: error.clone(),
                    });
                }
            }
        }
    }

    Ok(result)
}

async fn ensure_github_sources(
    pool: &DbPool,
    sources: &[PortableGithubSource],
) -> Result<(usize, usize), String> {
    let mut existing = existing_registry_identities(pool).await?;
    let mut added = 0;
    let mut skipped = 0;

    for source in sources
        .iter()
        .filter(|source| source.source_type == "github")
    {
        let identity = normalize_registry_identity(&source.url);
        if existing.contains(&identity) {
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
        .map(|resolution| (resolution.skill_id.clone(), resolution))
        .collect::<HashMap<_, _>>();
    let mut grouped = HashMap::<RepoKey, ImportGroup>::new();
    let mut result = SkillportStateImportResult::default();

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
        let resolution = resolution_for_skill(pool, skill, &resolution_map).await?;
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

async fn resolution_for_skill(
    pool: &DbPool,
    skill: &PortableCentralSkill,
    resolutions: &HashMap<String, SkillportStateImportResolution>,
) -> Result<SkillportStateImportResolution, String> {
    if let Some(resolution) = resolutions.get(&skill.id) {
        return Ok(resolution.clone());
    }

    let resolution = if db::get_skill_by_id(pool, &skill.id).await?.is_some() {
        DuplicateResolution::Skip
    } else {
        DuplicateResolution::Overwrite
    };

    Ok(SkillportStateImportResolution {
        skill_id: skill.id.clone(),
        source_path: Some(skill.source.source_path.clone()),
        resolution,
        renamed_skill_id: None,
    })
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
        PortableCentralSkillSource {
            source_type: "github".to_string(),
            owner: "openai".to_string(),
            repo: "skills".to_string(),
            branch: "main".to_string(),
            url: "https://github.com/openai/skills".to_string(),
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
        let json = export_skillport_state_impl(&pool).await.unwrap();
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

        let manifest = parse_manifest(&export_skillport_state_impl(&pool).await.unwrap()).unwrap();

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

        let manifest = parse_manifest(&export_skillport_state_impl(&pool).await.unwrap()).unwrap();
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
            paths,
        );

        let preview = preview_skillport_state_import_impl(&pool, &manifest, Some(&catalog))
            .await
            .unwrap();

        assert_eq!(preview.summary.ready, 1);
        assert_eq!(preview.summary.conflicts, 1);
        assert_eq!(preview.summary.missing, 1);
        assert_eq!(preview.summary.unrestorable, 1);
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
