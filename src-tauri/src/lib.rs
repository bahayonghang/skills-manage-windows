pub mod central_migration;
pub mod commands;
pub mod db;
pub mod logging;
pub mod operation_log;
pub mod paths;
pub mod secrets;
pub mod services;
pub mod targets;

use db::DbPool;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager};

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
    /// Cooperative cancel flag for the at-most-one Central update job.
    /// Producers (`check_central_skill_updates`, `update_central_skills`)
    /// reset to false on entry and poll between iterations; the
    /// `cancel_central_skill_updates` command stores true to request stop.
    pub central_update_cancel: Arc<AtomicBool>,
    /// Cooperative cancel flag for the at-most-one SkillPort state
    /// portability command (export, preview, or import).
    pub portable_state_cancel: Arc<AtomicBool>,
    /// Application-level sensitive values such as GitHub PAT and AI API keys.
    /// Commands receive this injectable store from AppState so unit tests do
    /// not need to touch the real OS credential vault.
    pub secrets: Arc<dyn secrets::SecretStore>,
    pub targets: targets::TargetRegistry,
}

impl AppState {
    pub async fn active_db(&self) -> Result<DbPool, String> {
        self.targets.active_db(&self.db).await
    }

    pub async fn active_target(&self) -> Result<targets::ActiveTarget, String> {
        self.targets.active_target(&self.db).await
    }
}

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
            Err(error) => {
                tracing::warn!(error = %error, "AI tag job registry lock is poisoned during register");
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
            Err(error) => {
                tracing::warn!(error = %error, "AI tag job registry lock is poisoned during finish");
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            logging::init_file_logging().map_err(std::io::Error::other)?;

            let db_dir = paths::app_data_dir();
            fs::create_dir_all(&db_dir).expect("Failed to create ~/.skillsmanage directory");
            let db_path = paths::path_to_string(&db_dir.join("db.sqlite"));

            // Synchronously open the SQLite pool and initialize schema. These
            // are required before any IPC command can run, so they stay in
            // the setup block. Legacy migration is deferred to the background
            // (spawned below) to keep cold-start latency small.
            let pool = tauri::async_runtime::block_on(async {
                db::create_pool(&db_path)
                    .await
                    .expect("Failed to open SQLite database")
            });
            tauri::async_runtime::block_on(async {
                db::init_database(&pool)
                    .await
                    .expect("Failed to initialize database schema")
            });

            let secrets: Arc<dyn secrets::SecretStore> =
                Arc::new(secrets::SystemSecretStore::default());

            app.manage(AppState {
                db: pool.clone(),
                ai_tag_jobs: AiTagJobRegistry::default(),
                central_update_cancel: Arc::new(AtomicBool::new(false)),
                portable_state_cancel: Arc::new(AtomicBool::new(false)),
                secrets: Arc::clone(&secrets),
                targets: targets::TargetRegistry::default(),
            });

            let github_pat_migration_pool = pool.clone();
            let github_pat_migration_secrets = Arc::clone(&secrets);
            tauri::async_runtime::spawn(async move {
                if let Err(error) = services::github_import::migrate_github_pat_on_startup(
                    &github_pat_migration_pool,
                    github_pat_migration_secrets.as_ref(),
                )
                .await
                {
                    tracing::warn!(error = %error, "Failed to run GitHub token secure-storage migration");
                }
            });

            let ai_api_key_migration_pool = pool.clone();
            let ai_api_key_migration_secrets = Arc::clone(&secrets);
            tauri::async_runtime::spawn(async move {
                if let Err(error) = services::ai_provider::migrate_ai_api_key_on_startup(
                    &ai_api_key_migration_pool,
                    ai_api_key_migration_secrets.as_ref(),
                )
                .await
                {
                    tracing::warn!(error = %error, "Failed to run AI API key secure-storage migration");
                }
            });

            // Legacy central-skills migration runs after the IPC handlers are
            // wired up, so the UI never waits on disk copies during startup.
            // Progress is broadcast via MIGRATION_PROGRESS_EVENT.
            let migration_handle = app.handle().clone();
            let migration_pool = pool;
            tauri::async_runtime::spawn(async move {
                let _ = migration_handle.emit(MIGRATION_PROGRESS_EVENT, MigrationProgress::Started);
                match central_migration::migrate_legacy_central_skills_to_private_store(
                    &migration_pool,
                )
                .await
                {
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
                        tracing::error!(error = %error, "Failed to migrate legacy Central Skills store");
                        let _ = migration_handle.emit(
                            MIGRATION_PROGRESS_EVENT,
                            MigrationProgress::Failed {
                                error: error.clone(),
                            },
                        );
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::get_bootstrap_snapshot,
            commands::bootstrap::get_skill_counts_summary,
            // Targets
            commands::targets::list_targets,
            commands::targets::create_ssh_target,
            commands::targets::update_ssh_target,
            commands::targets::test_ssh_target,
            commands::targets::update_ssh_target_password,
            commands::targets::delete_target,
            commands::targets::set_active_target,
            commands::targets::get_active_target,
            // Operation logs
            commands::logs::list_operation_logs,
            commands::logs::get_operation_log,
            commands::logs::clear_operation_logs,
            commands::logs::export_operation_logs,
            // Scanner
            commands::scanner::scan_all_skills,
            // Agents
            commands::agents::get_agents,
            commands::agents::detect_agents,
            commands::agents::list_platform_paths,
            commands::agents::add_custom_agent,
            commands::agents::update_custom_agent,
            commands::agents::remove_custom_agent,
            commands::agents::set_agent_enabled,
            // Linker
            commands::linker::install_skill_to_agent,
            commands::linker::uninstall_skill_from_agent,
            commands::linker::batch_install_to_agents,
            commands::linker::batch_install_central_skills,
            // Skills
            commands::skills::get_skills_by_agent,
            commands::skills::get_central_skills,
            commands::skills::preview_delete_central_skills,
            commands::skills::delete_central_skill,
            commands::skills::delete_central_skills,
            commands::skills::preview_delete_skill_repository,
            commands::skills::delete_skill_repository,
            commands::skills::get_skill_detail,
            commands::skills::read_skill_content,
            commands::skills::read_file_by_path,
            commands::skills::list_directory_tree,
            commands::skills::open_in_file_manager,
            // Central metadata
            commands::central_metadata::get_skill_repositories,
            commands::central_metadata::create_or_update_skill_repository,
            commands::central_metadata::assign_skills_to_repository,
            commands::central_metadata::get_skill_tags,
            commands::central_metadata::create_skill_tag,
            commands::central_metadata::assign_skill_tags,
            commands::central_metadata::suggest_skill_tags,
            commands::central_metadata::bulk_suggest_skill_tags,
            commands::central_metadata::cancel_ai_tag_job,
            commands::central_metadata::get_pending_ai_tag_reviews,
            commands::central_metadata::accept_ai_tag_review,
            commands::central_metadata::skip_ai_tag_review,
            // Central updates
            commands::central_updates::get_central_skill_update_states,
            commands::central_updates::check_central_skill_updates,
            commands::central_updates::update_central_skills,
            commands::central_updates::cancel_central_skill_updates,
            commands::central_updates::keep_remote_missing_central_skills,
            // Collections
            commands::collections::create_collection,
            commands::collections::get_collections,
            commands::collections::get_collection_detail,
            commands::collections::add_skill_to_collection,
            commands::collections::remove_skill_from_collection,
            commands::collections::delete_collection,
            commands::collections::update_collection,
            commands::collections::batch_install_collection,
            commands::collections::export_collection,
            commands::collections::import_collection,
            // Settings
            commands::settings::get_scan_directories,
            commands::settings::add_scan_directory,
            commands::settings::remove_scan_directory,
            commands::settings::set_scan_directory_active,
            commands::settings::get_setting,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_settings,
            commands::settings::get_ai_api_key_state,
            commands::settings::set_ai_api_key,
            commands::settings::clear_ai_api_key,
            commands::github_import::get_github_pat,
            commands::github_import::set_github_pat,
            commands::github_import::clear_github_pat,
            commands::github_import::test_github_pat,
            // Discover
            commands::discover::discover_scan_roots,
            commands::discover::get_scan_roots,
            commands::discover::get_obsidian_vaults,
            commands::discover::get_obsidian_vault_skills,
            commands::discover::get_discovered_summary,
            commands::discover::set_scan_root_enabled,
            commands::discover::start_project_scan,
            commands::discover::stop_project_scan,
            commands::discover::get_discovered_skills,
            commands::discover::import_discovered_skill_to_central,
            commands::discover::import_discovered_skill_to_platform,
            commands::discover::import_source_skill_to_central,
            commands::discover::import_source_skill_to_platform,
            commands::discover::clear_discovered_skills,
            commands::github_import::preview_github_repo_import,
            commands::github_import::import_github_repo_skills,
            commands::github_import::fetch_github_skill_markdown,
            commands::github_import::discard_github_repo_preview_workspace,
            // Marketplace
            commands::marketplace::list_registries,
            commands::marketplace::add_registry,
            commands::marketplace::remove_registry,
            commands::marketplace::sync_registry,
            commands::marketplace::sync_registry_with_options,
            commands::marketplace::search_marketplace_skills,
            commands::marketplace::install_marketplace_skill,
            commands::marketplace::explain_skill,
            commands::marketplace::get_skill_explanation,
            commands::marketplace::explain_skill_stream,
            commands::marketplace::refresh_skill_explanation,
            commands::portable_state::export_skillport_state,
            commands::portable_state::preview_skillport_state_import,
            commands::portable_state::import_skillport_state,
            commands::portable_state::cancel_skillport_state_portability,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
