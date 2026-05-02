use std::collections::HashMap;
use std::path::Path;

use crate::db::{self, DbPool};
use crate::paths;

use super::types::{DiscoveredProject, DiscoveredSkill, DiscoveredSummary};

pub async fn get_discovered_summary_impl(pool: &DbPool) -> Result<DiscoveredSummary, String> {
    let total_skills_found = db::get_discovered_skill_count(pool).await?;
    let total_projects_found = db::get_discovered_project_count(pool).await?.max(0) as usize;

    Ok(DiscoveredSummary {
        total_skills_found,
        total_projects_found,
    })
}

/// Load previously discovered skills from the database, grouped by project.
pub async fn get_discovered_skills_impl(pool: &DbPool) -> Result<Vec<DiscoveredProject>, String> {
    let rows = db::get_all_discovered_skills(pool).await?;

    let central_dir = paths::central_skills_dir();
    let platform_names: HashMap<String, String> = db::builtin_agents()
        .into_iter()
        .map(|agent| (agent.id, agent.display_name))
        .collect();

    let skills: Vec<DiscoveredSkill> = rows
        .into_iter()
        .map(|row| {
            let skill_dir_name = Path::new(&row.dir_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let is_already_central = central_dir.join(skill_dir_name).exists();
            let platform_id = row.platform_id.clone();

            DiscoveredSkill {
                id: row.id,
                name: row.name,
                description: row.description,
                file_path: row.file_path,
                dir_path: row.dir_path,
                platform_id: platform_id.clone(),
                platform_name: platform_names
                    .get(&platform_id)
                    .cloned()
                    .unwrap_or_else(|| platform_id.clone()),
                project_path: row.project_path,
                project_name: row.project_name,
                is_already_central,
            }
        })
        .collect();

    let mut by_project: HashMap<String, Vec<DiscoveredSkill>> = HashMap::new();
    let mut project_names: HashMap<String, String> = HashMap::new();

    for skill in skills {
        project_names.insert(skill.project_path.clone(), skill.project_name.clone());
        by_project
            .entry(skill.project_path.clone())
            .or_default()
            .push(skill);
    }

    let mut projects: Vec<DiscoveredProject> = by_project
        .into_iter()
        .map(|(path, skills)| DiscoveredProject {
            project_path: path.clone(),
            project_name: project_names.get(&path).cloned().unwrap_or_default(),
            skills,
        })
        .collect();

    projects.sort_by(|a, b| a.project_name.cmp(&b.project_name));

    Ok(projects)
}

/// Clear all discovered skills from the database.
pub async fn clear_discovered_skills_impl(pool: &DbPool) -> Result<(), String> {
    db::clear_all_discovered_skills(pool).await
}
