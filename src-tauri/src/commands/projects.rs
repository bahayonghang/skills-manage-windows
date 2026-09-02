//! Tauri IPC shells for project-level skill management.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget, ReviewedDiagnostic,
    ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::services::projects;
use crate::services::projects::{ProjectDto, ProjectSkillDto, ProjectUsingSkillDto};
use crate::AppState;

/// 后端异步扫完后向前端 emit 的事件 payload。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScannedPayload {
    pub project_id: String,
    pub skill_count: usize,
}

const PROJECT_SCANNED_EVENT: &str = "project:scanned";

/// 弹原生文件夹选择对话框，返回用户挑选的项目根绝对路径。
/// 用户取消返回 `Ok(None)`，前端据此决定是否继续 add 流程。
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn pick_project_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Option<String>> {
    let entry = crate::ipc_registry::command_policy("pick_project_folder")
        .expect("pick_project_folder must be registered");
    let CommandLogPolicy::Operation(definition) = entry.policy else {
        unreachable!("pick_project_folder must be auditable")
    };
    crate::ipc_boundary!(
        "pick_project_folder",
        async move {
            crate::observability::run_operation(
                &state,
                definition,
                OperationTarget::local(),
                |picked| match picked {
                    Some(_) => SafeOperationResult::succeeded("Project folder selected."),
                    None => SafeOperationResult::cancelled("Project folder selection cancelled."),
                },
                || async move {
                    use tauri_plugin_dialog::DialogExt;

                    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
                    app.dialog()
                        .file()
                        .set_title("选择项目根目录")
                        .pick_folder(move |chosen| {
                            let value = chosen.and_then(|p| p.as_path().map(PathBuf::from));
                            let _ = tx.send(value);
                        });

                    let picked = tauri::async_runtime::spawn_blocking(move || rx.recv())
                        .await
                        .map_err(|_| {
                            ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                        })?
                        .map_err(|_| {
                            ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                        })?;

                    Ok(picked.map(|path| path.to_string_lossy().into_owned()))
                },
            )
            .await
        }
        .await
    )
}

/// add 项目：立即返回 ProjectDto（skill_count 初始为 0），扫描在后台异步执行，
/// 完成后 emit `project:scanned`。
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn add_project(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
) -> crate::ipc_error::IpcResult<ProjectDto> {
    crate::ipc_boundary_async!("add_project", {
        let pool = state.db.clone();
        let entry = crate::ipc_registry::command_policy("add_project")
            .expect("add_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("add_project must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |project: &ProjectDto| {
                SafeOperationResult::succeeded("Project added.")
                    .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&project.id))
            },
            || async move {
                let project = projects::add_project_impl(&pool, &path)
                    .await
                    .map_err(|_| {
                        ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                    })?;
                let project_id = project.id.clone();
                let scan_pool = pool.clone();
                let scan_app = app.clone();
                tauri::async_runtime::spawn(async move {
                    match projects::rescan_project_impl(&scan_pool, &project_id).await {
                        Ok(skill_count) => {
                            let _ = scan_app.emit(
                                PROJECT_SCANNED_EVENT,
                                ProjectScannedPayload {
                                    project_id,
                                    skill_count,
                                },
                            );
                        }
                        Err(_) => {
                            tracing::warn!(
                                target: "skillport::project",
                                code = "project.background_rescan_failed",
                                phase = "filesystem",
                                "Project background rescan failed"
                            );
                            let _ = scan_app.emit(
                                PROJECT_SCANNED_EVENT,
                                ProjectScannedPayload {
                                    project_id,
                                    skill_count: 0,
                                },
                            );
                        }
                    }
                });
                Ok(ProjectDto {
                    id: project.id,
                    path: project.path,
                    name: project.name,
                    pinned: project.pinned,
                    added_at: project.added_at,
                    last_scanned_at: project.last_scanned_at,
                    skill_count: 0,
                })
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn list_projects(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<ProjectDto>> {
    crate::ipc_boundary!(
        "list_projects",
        async move {
            projects::list_projects_impl(&state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn rename_project(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("rename_project", {
        let entry = crate::ipc_registry::command_policy("rename_project")
            .expect("rename_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("rename_project must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Project, SafeIdentifier::new(&id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Project renamed."),
            || async move {
                projects::rename_project_impl(&pool, &id, &name)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn set_project_pinned(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("set_project_pinned", {
        let entry = crate::ipc_registry::command_policy("set_project_pinned")
            .expect("set_project_pinned must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("set_project_pinned must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Project, SafeIdentifier::new(&id));
        let mode = if pinned { "pinned" } else { "unpinned" };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Project pin updated.")
                    .stable(SafeDetailKey::Mode, mode)
            },
            || async move {
                projects::set_project_pinned_impl(&pool, &id, pinned)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn rescan_project(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> crate::ipc_error::IpcResult<u32> {
    crate::ipc_boundary_async!("rescan_project", {
        let entry = crate::ipc_registry::command_policy("rescan_project")
            .expect("rescan_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("rescan_project must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Project, SafeIdentifier::new(&id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |count: &u32| {
                SafeOperationResult::succeeded("Project rescanned.")
                    .count(SafeDetailKey::AffectedCount, u64::from(*count))
            },
            || async move {
                let count = projects::rescan_project_impl(&pool, &id)
                    .await
                    .map_err(|_| {
                        ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                    })?;
                let _ = app.emit(
                    PROJECT_SCANNED_EVENT,
                    ProjectScannedPayload {
                        project_id: id,
                        skill_count: count,
                    },
                );
                Ok(u32::try_from(count).unwrap_or(u32::MAX))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_project_skills(
    state: State<'_, AppState>,
    id: String,
) -> crate::ipc_error::IpcResult<Vec<ProjectSkillDto>> {
    crate::ipc_boundary!(
        "get_project_skills",
        async move {
            projects::get_project_skills_impl(&state.db, &id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn remove_project(
    state: State<'_, AppState>,
    id: String,
    uninstall_skills: bool,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("remove_project", {
        let entry = crate::ipc_registry::command_policy("remove_project")
            .expect("remove_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("remove_project must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Project, SafeIdentifier::new(&id));
        let mode = if uninstall_skills {
            "remove_and_uninstall"
        } else {
            "remove_only"
        };
        crate::observability::run_operation(
            &state,
            definition,
            context,
            move |_| {
                SafeOperationResult::succeeded("Project removed.").stable(SafeDetailKey::Mode, mode)
            },
            || async move {
                projects::remove_project_impl(&pool, &id, uninstall_skills)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// 装一个中央 skill 到项目下某个 agent 目录。
/// 返回写入的 psi 行（含真实的 link_type / symlink_target）。
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn install_skill_to_project(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
    agent_id: String,
    method: String,
) -> crate::ipc_error::IpcResult<crate::db::ProjectSkillInstallation> {
    crate::ipc_boundary_async!("install_skill_to_project", {
        let entry = crate::ipc_registry::command_policy("install_skill_to_project")
            .expect("install_skill_to_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("install_skill_to_project must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Project,
            SafeIdentifier::new(&project_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| {
                SafeOperationResult::succeeded("Skill installed to project.")
                    .count(SafeDetailKey::AffectedCount, 1)
            },
            || async move {
                projects::install_skill_to_project_impl(
                    &pool,
                    &project_id,
                    &skill_id,
                    &agent_id,
                    &method,
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// 从项目下指定 agent 目录卸载 skill。
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn uninstall_skill_from_project(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
    agent_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("uninstall_skill_from_project", {
        let entry = crate::ipc_registry::command_policy("uninstall_skill_from_project")
            .expect("uninstall_skill_from_project must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("uninstall_skill_from_project must be auditable")
        };
        let pool = state.db.clone();
        let context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Project,
            SafeIdentifier::new(&project_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| {
                SafeOperationResult::succeeded("Skill uninstalled from project.")
                    .count(SafeDetailKey::AffectedCount, 1)
            },
            || async move {
                projects::uninstall_skill_from_project_impl(
                    &pool,
                    &project_id,
                    &skill_id,
                    &agent_id,
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

/// 反查中央 skill 装在哪些项目，供详情页 sidebar 展示。
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn list_projects_using_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<Vec<ProjectUsingSkillDto>> {
    crate::ipc_boundary!(
        "list_projects_using_skill",
        async move {
            projects::list_projects_using_skill_impl(&state.db, &skill_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn project_commands_have_named_boundaries_without_path_diagnostics() {
        let source = include_str!("projects.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for command in [
            "pick_project_folder",
            "add_project",
            "list_projects",
            "rename_project",
            "set_project_pinned",
            "rescan_project",
            "get_project_skills",
            "remove_project",
            "install_skill_to_project",
            "uninstall_skill_from_project",
            "list_projects_using_skill",
        ] {
            assert!(production.contains(&format!("\"{command}\"")), "{command}");
        }
        for banned in [
            "SafeIdentifier::new(&path)",
            "error = %",
            "OperationLogEvent",
        ] {
            assert!(!production.contains(banned), "banned audit input: {banned}");
        }
        assert!(production.contains("SafeOperationResult::cancelled"));
        assert!(production.contains("crate::observability::run_operation"));
    }
}
