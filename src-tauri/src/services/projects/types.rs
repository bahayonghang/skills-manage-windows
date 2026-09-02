//! 前端可见的 DTO。后端 `crate::db::types::Project` 是纯表行，DTO 在它基础上
//! 补充 `skill_count` 等扫描期才有的信息。

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub pinned: bool,
    pub added_at: String,
    pub last_scanned_at: Option<String>,
    pub skill_count: u32,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillDto {
    pub project_id: String,
    pub skill_id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    /// `'central'` | `'project'`
    pub source_origin: String,
    pub agent_id: String,
    pub agent_display_name: String,
    pub installed_path: String,
    /// `'symlink'` | `'copy'`
    pub link_type: String,
    pub symlink_target: Option<String>,
}

/// 反向视图：一个 skill 装在哪些项目下、走哪个 agent、用哪种 link_type。
/// 用于中央 skill 详情页 sidebar 显示「装在哪些项目」。
#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsingSkillDto {
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub agent_id: String,
    pub agent_display_name: String,
    pub installed_path: String,
    pub link_type: String,
}
