//! Central skills service layer.
//!
//! Owns Central Skill listing, detail hydration, deletion previews/deletes,
//! repository deletion orchestration, and file/content helpers. Tauri IPC
//! shells live in `crate::commands::skills`.

mod common;
mod delete;
mod files;
mod query;
mod types;

#[cfg(test)]
mod tests;

pub use delete::{
    delete_central_skill_impl, delete_central_skill_ssh_impl, delete_central_skills_impl,
    delete_central_skills_ssh_impl, delete_skill_repository_impl, delete_skill_repository_ssh_impl,
    preview_delete_central_skills_impl, preview_delete_central_skills_ssh_impl,
    preview_delete_skill_repository_impl, preview_delete_skill_repository_ssh_impl,
};
pub use files::{
    open_in_file_manager_for_target_impl, read_file_by_path_for_target_impl,
    read_skill_content_for_target_impl,
};
pub use query::{
    get_central_skills_impl, get_skill_detail_with_row_impl, get_skills_by_agent_impl,
};
pub use types::{
    BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest,
    BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess, DeleteCentralSkillPreview,
    DeleteCentralSkillResult, DeleteSkillRepositoryPreview, DeleteSkillRepositoryResult,
    FailedCentralSkillDelete, SkillDetail, SkillInstallationDetail, SkillWithLinks,
};
