pub mod central_migration;
pub mod cli_api;
pub mod commands;
pub mod db;
pub mod fs_util;
#[cfg(feature = "ipc-codegen")]
pub mod ipc_codegen;
pub mod ipc_error;
pub mod ipc_registry;
pub mod logging;
pub mod observability;
pub mod operation_log;
pub mod paths;
pub mod redaction;
pub mod secrets;
pub mod services;
pub mod skill_time;
pub mod targets;

#[cfg(test)]
pub mod test_support;

use db::DbPool;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// Event name emitted on the main webview when the legacy central skills
/// migration progresses through start → completed/failed states. Front-end
/// subscribers can `listen("system://migration-progress", ...)` to react.
/// The migration is best-effort and never blocks IPC availability.
const MIGRATION_PROGRESS_EVENT: &str = "system://migration-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
enum MigrationProgress {
    Started,
    Completed {
        copied: usize,
        skipped: usize,
        failed: usize,
    },
    Failed {
        error: String,
    },
}

/// Application state shared across Tauri commands.
pub struct AppState {
    pub db: DbPool,
    pub ai_tag_jobs: AiTagJobRegistry,
    pub central_update_jobs: services::exclusive_job::ExclusiveJobRegistry,
    /// Short-lived GitHub repository snapshots shared by Central update check
    /// and update commands. This lets "check, then update" reuse the archive
    /// that was just downloaded without copying credentials into target DBs.
    pub central_update_snapshots: CentralUpdateSnapshotCache,
    pub portable_state_jobs: services::exclusive_job::ExclusiveJobRegistry,
    /// Skills CLI global add/remove family: exclusive within itself for
    /// cancel/progress only; filesystem mutual exclusion comes from the
    /// Local target mutation guard.
    pub skills_cli_jobs: services::exclusive_job::ExclusiveJobRegistry,
    /// Commands receive this injectable store from AppState so unit tests do
    /// not need to touch the real OS credential vault.
    pub secrets: Arc<dyn secrets::SecretStore>,
    pub targets: targets::TargetRegistry,
}

impl AppState {
    pub async fn resolve_target_context(&self) -> Result<targets::TargetContext, String> {
        self.targets
            .resolve_active_context(&self.db)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn active_db(&self) -> Result<DbPool, String> {
        Ok(self.resolve_target_context().await?.db().clone())
    }

    pub async fn active_target(&self) -> Result<targets::ActiveTarget, String> {
        self.targets
            .active_target(&self.db)
            .await
            .map_err(|e| e.to_string())
    }
}

pub use services::central_updates::CentralUpdateSnapshotCache;

#[derive(Default)]
pub struct AiTagJobRegistry {
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl AiTagJobRegistry {
    pub fn register(&self, job_id: &str) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        match self.jobs.lock() {
            Ok(mut jobs) => {
                jobs.insert(job_id.to_string(), Arc::clone(&cancel_flag));
            }
            Err(_error) => {
                tracing::warn!("AI tag job registry lock is poisoned during register");
            }
        }
        cancel_flag
    }

    pub fn cancel(&self, job_id: &str) -> bool {
        let Ok(jobs) = self.jobs.lock() else {
            tracing::warn!("AI tag job registry lock is poisoned during cancel");
            return false;
        };
        let Some(cancel_flag) = jobs.get(job_id) else {
            return false;
        };
        cancel_flag.store(true, Ordering::SeqCst);
        true
    }

    pub fn finish(&self, job_id: &str) {
        match self.jobs.lock() {
            Ok(mut jobs) => {
                jobs.remove(job_id);
            }
            Err(_error) => {
                tracing::warn!("AI tag job registry lock is poisoned during finish");
            }
        }
    }
}

fn focus_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!(
            code = "main_window_unavailable",
            "Could not focus the main window"
        );
        return;
    };

    if window.is_minimized().unwrap_or(false) && window.unminimize().is_err() {
        tracing::warn!(
            code = "main_window_unminimize_failed",
            "Could not restore the main window"
        );
    }
    if window.show().is_err() {
        tracing::warn!(
            code = "main_window_show_failed",
            "Could not show the main window"
        );
    }
    if window.set_focus().is_err() {
        tracing::warn!(
            code = "main_window_focus_failed",
            "Could not focus the main window"
        );
    }
    #[cfg(target_os = "windows")]
    if !force_windows_foreground(&window) {
        tracing::warn!(
            code = "main_window_foreground_failed",
            "Could not bring the main window to the foreground"
        );
    }
}

#[cfg(target_os = "windows")]
fn force_windows_foreground(window: &tauri::WebviewWindow) -> bool {
    unsafe extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(window: isize, process_id: *mut u32) -> u32;
        fn AttachThreadInput(attach: u32, attach_to: u32, value: i32) -> i32;
        fn BringWindowToTop(window: isize) -> i32;
        fn SetForegroundWindow(window: isize) -> i32;
        fn SetFocus(window: isize) -> isize;
        fn GetCurrentThreadId() -> u32;
    }

    let Ok(handle) = window.hwnd() else {
        return false;
    };
    let target = handle.0 as isize;

    // Windows normally rejects focus stealing across input queues. Temporarily
    // attach to the foreground queue so a user-initiated URI activation can
    // restore the already-running primary window.
    unsafe {
        let foreground = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, 1) != 0;
        let brought_to_top = BringWindowToTop(target) != 0;
        let focused = SetForegroundWindow(target) != 0;
        SetFocus(target);
        if attached {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
        brought_to_top && focused && GetForegroundWindow() == target
    }
}

async fn install_ready_state(
    app: &tauri::AppHandle,
    pool: DbPool,
) -> Result<(), services::startup::StartupError> {
    if let Err(error) = services::central_operation::recover_pending_operations(
        &pool,
        &targets::ActiveTarget::Local,
    )
    .await
    {
        tracing::warn!(
            code = error.code(),
            "Local Central operation recovery remains pending"
        );
    }
    if targets::recover_target_config(&pool).await.is_err() {
        tracing::warn!(
            code = "target_config_recovery_failed",
            "Target configuration recovery remains pending"
        );
    }

    let secrets: Arc<dyn secrets::SecretStore> = Arc::new(secrets::SystemSecretStore::default());
    if !app.manage(AppState {
        db: pool.clone(),
        ai_tag_jobs: AiTagJobRegistry::default(),
        central_update_jobs: services::exclusive_job::ExclusiveJobRegistry::new(
            "job.central_update_busy",
            "A Central update job is already running.",
        ),
        central_update_snapshots: CentralUpdateSnapshotCache::default(),
        portable_state_jobs: services::exclusive_job::ExclusiveJobRegistry::new(
            "job.portability_busy",
            "A portability job is already running.",
        ),
        skills_cli_jobs: services::exclusive_job::ExclusiveJobRegistry::new(
            "job.skills_cli_busy",
            "A Skills CLI job is already running.",
        ),
        secrets: Arc::clone(&secrets),
        targets: targets::TargetRegistry::default(),
    }) {
        return Err(services::startup::StartupError::StateAlreadyInstalled);
    }

    // `AppHandle::manage` is the process-local once boundary: only the first
    // successful state installation may sweep rows left by an older process.
    // A retry after state installation therefore cannot relabel a current
    // process operation as interrupted.
    observability::mark_interrupted_operations_best_effort(&pool).await;

    let github_pat_migration_pool = pool.clone();
    let github_pat_migration_secrets = Arc::clone(&secrets);
    tauri::async_runtime::spawn(async move {
        if services::github_import::migrate_github_pat_on_startup(
            &github_pat_migration_pool,
            github_pat_migration_secrets.as_ref(),
        )
        .await
        .is_err()
        {
            tracing::warn!("Failed to run GitHub token secure-storage migration");
        }
    });

    let ai_api_key_migration_pool = pool.clone();
    let ai_api_key_migration_secrets = Arc::clone(&secrets);
    tauri::async_runtime::spawn(async move {
        if services::ai_provider::migrate_ai_api_key_on_startup(
            &ai_api_key_migration_pool,
            ai_api_key_migration_secrets.as_ref(),
        )
        .await
        .is_err()
        {
            tracing::warn!("Failed to run AI API key secure-storage migration");
        }
    });

    let migration_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = migration_handle.emit(MIGRATION_PROGRESS_EVENT, MigrationProgress::Started);
        match central_migration::migrate_legacy_central_skills_to_private_store(&pool).await {
            Ok(summary) => {
                let _ = migration_handle.emit(
                    MIGRATION_PROGRESS_EVENT,
                    MigrationProgress::Completed {
                        copied: summary.copied,
                        skipped: summary.skipped_existing,
                        failed: summary.failed,
                    },
                );
            }
            Err(error) => {
                tracing::error!("Failed to migrate legacy Central Skills store");
                let _ = migration_handle.emit(
                    MIGRATION_PROGRESS_EVENT,
                    MigrationProgress::Failed {
                        error: error.to_string(),
                    },
                );
            }
        }
    });
    Ok(())
}

pub(crate) async fn run_startup_attempt(
    app: &tauri::AppHandle,
    coordinator: &services::startup::StartupCoordinator,
    backup_created: bool,
) -> services::startup::StartupStatus {
    coordinator.set_status(services::startup::StartupStatus::Checking);
    let status = match services::startup::attempt_startup(coordinator.db_path()).await {
        Ok(pool) => match install_ready_state(app, pool).await {
            Ok(()) => services::startup::StartupStatus::Ready,
            Err(_error) => {
                tracing::error!(
                    code = services::startup::StartupIssue::DatabaseRecoveryFailed.code(),
                    "Startup application state installation failed"
                );
                services::startup::StartupStatus::Fatal {
                    issue: services::startup::StartupIssue::DatabaseRecoveryFailed,
                }
            }
        },
        Err(failure) => {
            tracing::error!(
                code = failure.issue.code(),
                diagnostic = ?failure.diagnostic,
                "Desktop startup prerequisites failed"
            );
            failure.status(backup_created)
        }
    };
    coordinator.set_status(status.clone());
    status
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(deprecated)]
pub fn run() {
    tauri::Builder::default()
        .manage(services::deep_link::ImportIntentState::default())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            focus_main_window(app);

            let state = app.state::<services::deep_link::ImportIntentState>();
            match services::deep_link::parse_import_intent_from_argv(&argv).and_then(|intent| {
                services::deep_link::submit_import_intent(app, state.inner(), intent)
            }) {
                Ok(()) => {}
                Err(error) => tracing::warn!(
                    code = error.code(),
                    argument_count = argv.len(),
                    "Rejected warm-instance import intent"
                ),
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            logging::init_file_logging().map_err(std::io::Error::other)?;

            match app.deep_link().get_current() {
                Ok(Some(urls)) => {
                    let state = app.state::<services::deep_link::ImportIntentState>();
                    for url in urls {
                        if let Err(error) = services::deep_link::submit_import_deep_link(
                            app.handle(),
                            state.inner(),
                            url.as_str(),
                        ) {
                            tracing::warn!(
                                code = error.code(),
                                "Rejected cold-start import intent"
                            );
                        }
                    }
                }
                Ok(None) => {}
                Err(_) => tracing::warn!(
                    code = "deep_link_current_unavailable",
                    "Could not inspect the cold-start deep link"
                ),
            }

            let db_path = paths::app_data_dir().join("db.sqlite");
            if !app.manage(services::startup::StartupCoordinator::new(db_path)) {
                return Err(
                    std::io::Error::other("Startup coordinator is already installed").into(),
                );
            }
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                let coordinator = app_handle.state::<services::startup::StartupCoordinator>();
                let _operation = coordinator.lock_operation().await;
                run_startup_attempt(&app_handle, coordinator.inner(), false).await;
            });
            Ok(())
        })
        .invoke_handler(crate::runtime_command_handler!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_modules_do_not_mix_ambient_target_and_db_resolution() {
        fn visit(directory: &std::path::Path, violations: &mut Vec<String>) {
            for entry in std::fs::read_dir(directory).expect("read commands directory") {
                let path = entry.expect("command directory entry").path();
                if path.is_dir() {
                    visit(&path, violations);
                } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                    let source = std::fs::read_to_string(&path).expect("read command source");
                    if source.contains("state.active_target()")
                        && source.contains("state.active_db()")
                    {
                        violations.push(path.display().to_string());
                    }
                }
            }
        }

        let mut violations = Vec::new();
        visit(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands"),
            &mut violations,
        );
        assert!(
            violations.is_empty(),
            "command modules must resolve one request-scoped TargetContext: {violations:?}"
        );
    }

    #[test]
    fn ai_tag_job_registry_poisoning_returns_controlled_fallbacks() {
        let registry = Arc::new(AiTagJobRegistry::default());
        let poisoned = Arc::clone(&registry);
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.jobs.lock().expect("lock");
            panic!("poison AI job registry");
        })
        .join();

        let cancel_flag = registry.register("job-after-poison");

        assert!(!cancel_flag.load(Ordering::SeqCst));
        assert!(!registry.cancel("job-after-poison"));
        registry.finish("job-after-poison");
    }

    #[test]
    fn app_state_does_not_restore_shared_update_or_portability_cancel_flags() {
        let source = include_str!("lib.rs");
        let central_field = ["pub central_update_", "cancel:"].concat();
        let portability_field = ["pub portable_state_", "cancel:"].concat();
        assert!(!source.contains(&central_field));
        assert!(!source.contains(&portability_field));
    }
}
