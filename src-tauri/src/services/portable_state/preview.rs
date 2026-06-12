use futures_util::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;

use crate::{
    db::{self, DbPool},
    secrets::SecretStore,
    services::github_import,
};

use super::error::PortableStateError;
use super::progress::{check_cancel, emit_portability_step};
use super::types::{
    CancelFlag, PortableCentralSkillSource, RemoteCatalogEntry, RemoteCatalogInvalidCandidate,
    RepoKey, SkillManifestKey, SkillPreviewStatus, SkillportStateImportPreview,
    SkillportStateImportPreviewSummary, SkillportStateImportPreviewWarning, SkillportStateManifest,
    SkillportStatePortabilityPhase, SkillportStateSkillPreview, SkillportStateSourcePreview,
    SourcePreviewStatus, EXPORT_KIND, EXPORT_VERSION, REMOTE_CATALOG_CONCURRENCY_LIMIT,
};
use super::{
    existing_registry_identities, import_source_path, normalize_registry_identity, repo_key,
    repo_url_for_source,
};

pub(crate) fn parse_manifest(json: &str) -> Result<SkillportStateManifest, PortableStateError> {
    let manifest: SkillportStateManifest =
        serde_json::from_str(json).map_err(PortableStateError::InvalidManifestJson)?;
    if manifest.kind != EXPORT_KIND {
        return Err(PortableStateError::UnsupportedExportKind);
    }
    if manifest.version != EXPORT_VERSION {
        return Err(PortableStateError::UnsupportedExportVersion(
            manifest.version,
        ));
    }
    Ok(manifest)
}

pub(crate) async fn preview_skillport_state_import_impl(
    pool: &DbPool,
    manifest: &SkillportStateManifest,
    remote_catalog: Option<&HashMap<RepoKey, RemoteCatalogEntry>>,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<SkillportStateImportPreview, PortableStateError> {
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
    let mut warnings = Vec::new();
    let mut seen_warnings = HashSet::new();
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
        } else if let Some(issue) =
            remote_catalog_issue(remote_catalog, &skill.source, &source_path)
        {
            if issue.is_warning {
                let warning_key = format!(
                    "{}\u{1f}{}\u{1f}{}",
                    issue.reason,
                    issue.repo_url.as_deref().unwrap_or_default(),
                    issue.source_path.as_deref().unwrap_or_default()
                );
                if seen_warnings.insert(warning_key) {
                    warnings.push(SkillportStateImportPreviewWarning {
                        reason: issue.reason,
                        detail: issue.detail.unwrap_or_default(),
                        source_path: issue.source_path,
                        repo_url: issue.repo_url,
                    });
                }
                if let Some(existing) = existing_skills.get(&skill.id) {
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
                }
            } else {
                (issue.status, None, Some(issue.reason), issue.detail)
            }
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
        warnings,
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

pub(crate) async fn build_remote_catalog(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    manifest: &SkillportStateManifest,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<HashMap<RepoKey, RemoteCatalogEntry>, PortableStateError> {
    check_cancel(cancel)?;
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
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
                    repo_error: Some(error.to_string()),
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

struct RemoteCatalogIssue {
    status: SkillPreviewStatus,
    reason: String,
    detail: Option<String>,
    source_path: Option<String>,
    repo_url: Option<String>,
    is_warning: bool,
}

fn remote_catalog_issue(
    remote_catalog: Option<&HashMap<RepoKey, RemoteCatalogEntry>>,
    source: &PortableCentralSkillSource,
    source_path: &str,
) -> Option<RemoteCatalogIssue> {
    let catalog = remote_catalog?;
    let entry = catalog.get(&repo_key(source))?;
    if let Some(error) = &entry.repo_error {
        return Some(RemoteCatalogIssue {
            status: SkillPreviewStatus::Ready,
            reason: "repo_unavailable".to_string(),
            detail: Some(error.clone()),
            source_path: None,
            repo_url: Some(repo_url_for_source(source)),
            is_warning: true,
        });
    }
    if let Some(invalid) = entry.invalid_candidates.get(source_path) {
        return Some(RemoteCatalogIssue {
            status: SkillPreviewStatus::Unrestorable,
            reason: invalid.reason.clone(),
            detail: Some(invalid.detail.clone()),
            source_path: None,
            repo_url: None,
            is_warning: false,
        });
    }
    if entry.valid_source_paths.contains(source_path) {
        return None;
    }

    Some(RemoteCatalogIssue {
        status: SkillPreviewStatus::Missing,
        reason: "source_missing".to_string(),
        detail: None,
        source_path: None,
        repo_url: None,
        is_warning: false,
    })
}
