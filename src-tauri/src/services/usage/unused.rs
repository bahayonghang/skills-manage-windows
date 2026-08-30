//! Read-only Central and platform projections for the unused-skills panel.

use std::collections::{HashMap, HashSet};

use chrono::Utc;

use crate::db::{self, DbPool};

use super::{aggregate, enrichment, UsageError, UsageSkillMatchStatus};

/// Build the never-used / stale report from usage facts and the active target's
/// skill inventory. `usage_pool` owns target-scoped usage history while
/// `skills_pool` owns Central, installation, observation, and recovery state.
pub async fn build_unused_report(
    usage_pool: &DbPool,
    skills_pool: &DbPool,
    target_id: &str,
    source: Option<&str>,
    threshold_days: u32,
) -> Result<aggregate::UnusedSkillsReport, UsageError> {
    let now_ms = Utc::now().timestamp_millis();
    let central_skills = db::get_central_skills(skills_pool).await?;
    let resolved_aggregates: HashMap<String, db::ResolvedCallAggregateRow> =
        db::list_resolved_call_aggregates(usage_pool, target_id, source)
            .await?
            .into_iter()
            .map(|row| (row.resolved_skill_id.clone(), row))
            .collect();
    let central_ids = central_skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let pending_skill_ids = db::list_pending_fs_db_operations(skills_pool, target_id)
        .await?
        .into_iter()
        .map(|operation| operation.skill_id)
        .collect::<HashSet<_>>();
    let mut installations_by_skill =
        db::get_skill_installations_for_skills(skills_pool, &central_ids).await?;

    let mut central = Vec::new();
    for skill in central_skills {
        let stats = resolved_aggregates.get(&skill.id);
        let call_count = stats.map(|row| row.call_count).unwrap_or(0);
        let last_used_ms = stats.and_then(|row| row.last_used_ms);
        let Some(status) =
            aggregate::unused_skill_status(call_count, last_used_ms, now_ms, threshold_days)
        else {
            continue;
        };
        let mut agents = installations_by_skill
            .remove(&skill.id)
            .unwrap_or_default()
            .into_iter()
            .map(|installation| aggregate::UnusedAgentInstall {
                agent_id: installation.agent_id,
                link_type: installation.link_type,
                installed_path: installation.installed_path,
                has_pending_recovery: pending_skill_ids.contains(&skill.id),
            })
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        central.push(aggregate::UnusedSkillEntry {
            skill_id: Some(skill.id.clone()),
            name: skill.name.clone(),
            match_status: if stats.is_some() {
                UsageSkillMatchStatus::Matched
            } else {
                UsageSkillMatchStatus::Unmatched
            },
            origin: aggregate::UnusedSkillOrigin::Central,
            agents,
            installs: Vec::new(),
            installed_path: skill.canonical_path.clone(),
            call_count,
            last_used_ms,
            static_token_estimate: stats.and_then(|row| row.static_token_estimate),
            static_byte_count: stats.and_then(|row| row.static_byte_count),
            status,
        });
    }

    let observations = db::list_platform_skill_observations(skills_pool).await?;
    let call_aggregates: HashMap<String, db::NormalizedCallAggregateRow> =
        db::list_normalized_call_aggregates(usage_pool, target_id, source)
            .await?
            .into_iter()
            .map(|row| (row.normalized_skill.clone(), row))
            .collect();
    let mut metadata_by_normalized: HashMap<String, db::SkillUsageMetadataRow> = HashMap::new();
    for row in db::list_usage_metadata(usage_pool, target_id).await? {
        let key = enrichment::normalize_identity(&row.skill);
        let replace = metadata_by_normalized
            .get(&key)
            .map(|existing| row.match_status == "matched" && existing.match_status != "matched")
            .unwrap_or(true);
        if replace {
            metadata_by_normalized.insert(key, row);
        }
    }

    struct PlatformGroup {
        name: String,
        dir_path: String,
        installs: Vec<aggregate::UnusedPlatformInstall>,
    }
    let mut groups: HashMap<String, PlatformGroup> = HashMap::new();
    for observation in observations {
        let key = enrichment::normalize_identity(&observation.name);
        let install = aggregate::UnusedPlatformInstall {
            agent_id: observation.agent_id.clone(),
            row_id: Some(observation.row_id.clone()),
            skill_id: observation.skill_id.clone(),
            link_type: observation.link_type.clone(),
            source_kind: Some(observation.source_kind.clone()),
            is_read_only: observation.is_read_only,
            installed_path: observation.dir_path.clone(),
            has_pending_recovery: pending_skill_ids.contains(&observation.skill_id),
        };
        let group = groups.entry(key).or_insert_with(|| PlatformGroup {
            name: observation.name.clone(),
            dir_path: observation.dir_path.clone(),
            installs: Vec::new(),
        });
        group.installs.push(install);
    }

    let mut platforms = Vec::new();
    for (normalized, group) in groups {
        let stats = call_aggregates.get(&normalized);
        let call_count = stats.map(|row| row.call_count).unwrap_or(0);
        let last_used_ms = stats.and_then(|row| row.last_used_ms);
        let Some(status) =
            aggregate::unused_skill_status(call_count, last_used_ms, now_ms, threshold_days)
        else {
            continue;
        };
        let metadata = metadata_by_normalized.get(&normalized);
        platforms.push(aggregate::UnusedSkillEntry {
            skill_id: metadata.and_then(|row| row.resolved_skill_id.clone()),
            name: group.name,
            match_status: metadata
                .map(|row| UsageSkillMatchStatus::from_db(&row.match_status))
                .unwrap_or(UsageSkillMatchStatus::Unmatched),
            origin: aggregate::UnusedSkillOrigin::Platform,
            agents: Vec::new(),
            installs: group.installs,
            installed_path: Some(group.dir_path),
            call_count,
            last_used_ms,
            static_token_estimate: metadata.and_then(|row| row.static_token_estimate),
            static_byte_count: metadata.and_then(|row| row.static_byte_count),
            status,
        });
    }

    let by_name = |a: &aggregate::UnusedSkillEntry, b: &aggregate::UnusedSkillEntry| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    };
    central.sort_by(by_name);
    platforms.sort_by(by_name);

    Ok(aggregate::UnusedSkillsReport { central, platforms })
}
