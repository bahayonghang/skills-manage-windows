//! 项目级 skill 管理服务层。
//!
//! 替代 `services/discovery` 的「全盘扫描 → 临时 cache」模型：用户手动 add 项目，
//! 后端做单点扫描 + 装/卸 + reconcile。
//!
//! 模块布局：
//! - `types`：DTO（前端可见的结构）
//! - `crud` ：add/list/rename/pin/remove + 扫描入口
//! - `scan` ：遍历已启用 agent，对每个项目内 skill 目录扫盘，UPSERT psi
//!
//! 命令层在 `crate::commands::projects`。

pub mod crud;
pub mod error;
pub mod scan;
pub mod types;

#[cfg(test)]
mod tests;

pub use crud::{
    add_project_impl, get_project_skills_impl, install_skill_to_project_impl, list_projects_impl,
    list_projects_using_skill_impl, normalize_project_path, project_id_from_path,
    remove_project_impl, rename_project_impl, rescan_project_impl, set_project_pinned_impl,
    uninstall_skill_from_project_impl,
};
pub use error::ProjectsError;
pub use types::{ProjectDto, ProjectSkillDto, ProjectUsingSkillDto};
