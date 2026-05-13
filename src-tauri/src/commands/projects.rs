//! Tauri IPC shells for project-level skill management.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::db::Project;
use crate::services::projects;
use crate::services::projects::{ProjectDto, ProjectSkillDto};
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
pub async fn pick_project_folder(app: AppHandle) -> Result<Option<String>, String> {
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
        rx.recv().map_err(|e| format!("Dialog channel closed: {}", e))
    })
    .await
    .map_err(|e| format!("Failed to await folder pick: {}", e))??;

    Ok(picked.map(|p| p.to_string_lossy().into_owned()))
}

/// add 项目：立即返回 Project，扫描在后台异步执行，完成后 emit `project:scanned`。
#[tauri::command]
pub async fn add_project(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
) -> Result<Project, String> {
    let pool = state.db.clone();
    let project = projects::add_project_impl(&pool, &path).await?;

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

    Ok(project)
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectDto>, String> {
    projects::list_projects_impl(&state.db).await
}

#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), String> {
    projects::rename_project_impl(&state.db, &id, &name).await
}

#[tauri::command]
pub async fn set_project_pinned(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    projects::set_project_pinned_impl(&state.db, &id, pinned).await
}

#[tauri::command]
pub async fn rescan_project(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> Result<usize, String> {
    let count = projects::rescan_project_impl(&state.db, &id).await?;
    let _ = app.emit(
        PROJECT_SCANNED_EVENT,
        ProjectScannedPayload {
            project_id: id,
            skill_count: count,
        },
    );
    Ok(count)
}

#[tauri::command]
pub async fn get_project_skills(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ProjectSkillDto>, String> {
    projects::get_project_skills_impl(&state.db, &id).await
}

#[tauri::command]
pub async fn remove_project(
    state: State<'_, AppState>,
    id: String,
    uninstall_skills: bool,
) -> Result<(), String> {
    projects::remove_project_impl(&state.db, &id, uninstall_skills).await
}
