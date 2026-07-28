use serde::{Deserialize, Serialize};

pub const OBSIDIAN_PLATFORM_ID: &str = "obsidian";
pub const OBSIDIAN_PLATFORM_NAME: &str = "Obsidian";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianVault {
    pub id: String,
    pub name: String,
    pub path: String,
    pub skill_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianSkill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub platform_id: String,
    pub platform_name: String,
    pub project_path: String,
    pub project_name: String,
    pub is_already_central: bool,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianImportResult {
    pub skill_id: String,
    pub target: String,
}
