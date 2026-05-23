use std::collections::{HashMap, HashSet};

use crate::db::{self, AgentSkillObservation, DbPool};

use super::PlatformDuplicateGroup;

/// 内核版本：接受 pool 直接传入，便于单元测试。
pub(crate) async fn scan_platform_duplicate_skills_with_pool(
    pool: &DbPool,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, String> {
    /*
     * 步骤1：拿 agents
     * 步骤2：对每个 agent 拿 observations
     * 步骤3：同 skill_id 分组，分 writable / plugin readonly
     *        只保留两类都存在的组
     */
    let agents = db::get_all_agents(pool).await?;
    let target_agent_ids: HashSet<String> = match agent_ids {
        Some(ids) => ids.into_iter().collect(),
        None => agents.iter().map(|a| a.id.clone()).collect(),
    };

    let mut groups = Vec::new();
    for agent in agents {
        if !target_agent_ids.contains(&agent.id) {
            continue;
        }
        let observations = db::get_agent_skill_observations(pool, &agent.id).await?;
        groups.extend(group_platform_duplicate_skills(&agent.id, &observations));
    }
    // 稳定排序，便于前端展示
    groups.sort_by(|a, b| {
        a.agent_id
            .cmp(&b.agent_id)
            .then_with(|| a.skill_id.cmp(&b.skill_id))
    });
    Ok(groups)
}

/// 纯函数：从某 agent 的 observations 中分出 writable + plugin readonly 同时存在的组。
/// 抽出来方便单元测试避开 DB。
pub(crate) fn group_platform_duplicate_skills(
    agent_id: &str,
    observations: &[AgentSkillObservation],
) -> Vec<PlatformDuplicateGroup> {
    let mut by_skill: HashMap<String, (Vec<String>, Vec<String>, String)> = HashMap::new();
    for obs in observations {
        let entry = by_skill
            .entry(obs.skill_id.clone())
            .or_insert_with(|| (Vec::new(), Vec::new(), obs.name.clone()));
        let is_plugin = obs.source_kind == "plugin" || obs.is_read_only;
        if is_plugin {
            entry.1.push(obs.dir_path.clone());
        } else {
            entry.0.push(obs.dir_path.clone());
        }
    }
    let mut groups = Vec::new();
    for (skill_id, (writable, plugin, name)) in by_skill {
        if writable.is_empty() || plugin.is_empty() {
            continue;
        }
        groups.push(PlatformDuplicateGroup {
            agent_id: agent_id.to_string(),
            skill_id,
            skill_name: name,
            writable_paths: writable,
            plugin_paths: plugin,
        });
    }
    groups
}
