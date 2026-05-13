use chrono::Utc;
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;
use uuid::Uuid;

use crate::{
    db::{self, DbPool},
    secrets::SecretStore,
    services::{
        github_import,
        github_import::{DuplicateResolution, GitHubSkillImportSelection},
    },
};

use super::progress::{check_cancel, emit_portability_step};
use super::types::{
    CancelFlag, ImportGroup, PortableCentralSkill, PortableGithubSource, PortableSkillTag, RepoKey,
    SkillManifestKey, SkillportStateImportFailure, SkillportStateImportResolution,
    SkillportStateImportResult, SkillportStateImportedSkill, SkillportStateManifest,
    SkillportStatePortabilityPhase,
};
use super::{
    existing_registry_identities, export_source_path, import_source_path,
    normalize_registry_identity, repo_key, repo_url_for_source,
};

pub(crate) async fn import_skillport_state_impl(
    pool: &DbPool,
    secrets: &dyn SecretStore,
    manifest: &SkillportStateManifest,
    resolutions: Vec<SkillportStateImportResolution>,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<SkillportStateImportResult, String> {
    if check_cancel(cancel).is_err() {
        return Ok(cancelled_import_result(manifest, 0, 0));
    }
    let auth = github_import::github_direct_auth_from_secret_store(pool, secrets).await?;
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

pub(crate) async fn ensure_github_sources(
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

pub(crate) async fn build_import_groups(
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

pub(crate) async fn restore_skill_tags(
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
