//! Tauri IPC shells for project-level skill management.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

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
pub async fn pick_project_folder(app: AppHandle) -> crate::ipc_error::IpcResult<Option<String>> {
    crate::ipc_boundary!(
        async move {
            use tauri_plugin_dialog::DialogExt;

            let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
            app.dialog()
                .file()
                .set_title("选择项目根目录")
                .pick_folder(move |chosen| {
                    let value = chosen.and_then(|p| p.as_path().map(PathBuf::from));
                    let _ = tx.send(value);
                });

            let picked = tauri::async_runtime::spawn_blocking(move || {
                rx.recv()
                    .map_err(|e| format!("Dialog channel closed: {}", e))
            })
            .await
            .map_err(|e| format!("Failed to await folder pick: {}", e))??;

            Ok(picked.map(|p| p.to_string_lossy().into_owned()))
        }
        .await
    )
}

/// add 项目：立即返回 ProjectDto（skill_count 初始为 0），扫描在后台异步执行，
/// 完成后 emit `project:scanned`。
#[tauri::command]
pub async fn add_project(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
) -> crate::ipc_error::IpcResult<ProjectDto> {
    crate::ipc_boundary!(
        async move {
            let pool = state.db.clone();
            let project = projects::add_project_impl(&pool, &path)
                .await
                .map_err(|e| e.to_string())?;

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
                    Err(error) => {
                        tracing::warn!(error = %error, "Project rescan after add failed");
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
        }
        .await
    )
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<ProjectDto>> {
    crate::ipc_boundary!(
        async move {
            projects::list_projects_impl(&state.db)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            projects::rename_project_impl(&state.db, &id, &name)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn set_project_pinned(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            projects::set_project_pinned_impl(&state.db, &id, pinned)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn rescan_project(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> crate::ipc_error::IpcResult<usize> {
    crate::ipc_boundary!(
        async move {
            let count = projects::rescan_project_impl(&state.db, &id)
                .await
                .map_err(|e| e.to_string())?;
            let _ = app.emit(
                PROJECT_SCANNED_EVENT,
                ProjectScannedPayload {
                    project_id: id,
                    skill_count: count,
                },
            );
            Ok(count)
        }
        .await
    )
}

#[tauri::command]
pub async fn get_project_skills(
    state: State<'_, AppState>,
    id: String,
) -> crate::ipc_error::IpcResult<Vec<ProjectSkillDto>> {
    crate::ipc_boundary!(
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
    crate::ipc_boundary!(
        async move {
            projects::remove_project_impl(&state.db, &id, uninstall_skills)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
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
    crate::ipc_boundary!(
        async move {
            projects::install_skill_to_project_impl(
                &state.db,
                &project_id,
                &skill_id,
                &agent_id,
                &method,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
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
    crate::ipc_boundary!(
        async move {
            projects::uninstall_skill_from_project_impl(
                &state.db,
                &project_id,
                &skill_id,
                &agent_id,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

/// 反查中央 skill 装在哪些项目，供详情页 sidebar 展示。
#[tauri::command]
pub async fn list_projects_using_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<Vec<ProjectUsingSkillDto>> {
    crate::ipc_boundary!(
        async move {
            projects::list_projects_using_skill_impl(&state.db, &skill_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}
