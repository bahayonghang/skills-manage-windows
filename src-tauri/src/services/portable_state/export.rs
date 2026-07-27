use chrono::Utc;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;

use crate::db::{self, DbPool};

use super::error::PortableStateError;
use super::progress::{check_cancel, emit_portability_step};
use super::types::{
    CancelFlag, ExportedFrom, PortableCentralSkill, PortableCentralSkillSource,
    PortableGithubSource, PortableSkillTag, PortableStateTargetContext, PortableUnrestorableSkill,
    SkillportStateManifest, SkillportStatePortabilityPhase, EXPORT_KIND, EXPORT_VERSION,
};
use super::{export_source_path, normalize_registry_identity};

pub(crate) async fn export_skillport_state_impl(
    pool: &DbPool,
    target: Option<&PortableStateTargetContext>,
    job_id: &str,
    app: Option<&AppHandle>,
    cancel: Option<&CancelFlag>,
) -> Result<String, PortableStateError> {
    check_cancel(cancel)?;
    let github_sources = export_github_sources(pool).await?;
    emit_portability_step(
        app,
        job_id,
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
        job_id,
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
            job_id,
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
                uid: Some(skill.uid.clone()),
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
            target: target.map(Into::into),
        },
        github_sources,
        central_skills,
        unrestorable_skills,
    };

    emit_portability_step(
        app,
        job_id,
        SkillportStatePortabilityPhase::Finalizing,
        total_export_steps,
        total_export_steps,
        Some("Serializing SkillPort state JSON"),
        None,
    );
    serde_json::to_string_pretty(&manifest).map_err(PortableStateError::Json)
}

async fn export_github_sources(
    pool: &DbPool,
) -> Result<Vec<PortableGithubSource>, PortableStateError> {
    let registry_rows = sqlx::query(
        "SELECT url, is_enabled
         FROM skill_registries
         WHERE source_type = 'github'",
    )
    .fetch_all(pool)
    .await?;
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
    .await?;

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
