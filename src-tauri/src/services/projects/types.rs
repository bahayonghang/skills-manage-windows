//! 前端可见的 DTO。后端 `crate::db::types::Project` 是纯表行，DTO 在它基础上
//! 补充 `skill_count` 等扫描期才有的信息。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub pinned: bool,
    pub added_at: String,
    pub last_scanned_at: Option<String>,
    pub skill_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillDto {
    pub project_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub agent_id: String,
    pub agent_display_name: String,
    pub installed_path: String,
    /// `'symlink'` | `'copy'`
    pub link_type: String,
    pub symlink_target: Option<String>,
}
