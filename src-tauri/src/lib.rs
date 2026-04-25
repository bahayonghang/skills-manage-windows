pub mod central_migration;
pub mod commands;
pub mod db;
pub mod path_utils;
pub mod paths;

use db::DbPool;
use std::collections::HashMap;
use std::fs;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Manager;

/// Application state shared across Tauri commands.
pub struct AppState {
    pub db: DbPool,
    pub ai_tag_jobs: AiTagJobRegistry,
}

#[derive(Default)]
pub struct AiTagJobRegistry {
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl AiTagJobRegistry {
    pub fn register(&self, job_id: &str) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.insert(job_id.to_string(), Arc::clone(&cancel_flag));
        }
        cancel_flag
    }

    pub fn cancel(&self, job_id: &str) -> bool {
        let Ok(jobs) = self.jobs.lock() else {
            return false;
        };
        let Some(cancel_flag) = jobs.get(job_id) else {
            return false;
        };
        cancel_flag.store(true, Ordering::SeqCst);
        true
    }

    pub fn finish(&self, job_id: &str) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.remove(job_id);
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
            let db_dir = path_utils::app_data_dir();
            fs::create_dir_all(&db_dir).expect("Failed to create ~/.skillsmanage directory");
            let db_path = path_utils::path_to_string(&db_dir.join("db.sqlite"));

            // Create pool and initialize schema
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
            tauri::async_runtime::block_on(async {
                if let Err(error) =
                    central_migration::migrate_legacy_central_skills_to_private_store(&pool).await
                {
                    eprintln!("Failed to migrate legacy Central Skills store: {}", error);
                }
            });

            app.manage(AppState {
                db: pool,
                ai_tag_jobs: AiTagJobRegistry::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::get_bootstrap_snapshot,
            commands::bootstrap::get_skill_counts_summary,
            // Scanner
            commands::scanner::scan_all_skills,
            // Agents
            commands::agents::get_agents,
            commands::agents::detect_agents,
            commands::agents::add_custom_agent,
            commands::agents::update_custom_agent,
            commands::agents::remove_custom_agent,
            commands::agents::set_agent_enabled,
            // Linker
            commands::linker::install_skill_to_agent,
            commands::linker::uninstall_skill_from_agent,
            commands::linker::batch_install_to_agents,
            // Skills
            commands::skills::get_skills_by_agent,
            commands::skills::get_central_skills,
            commands::skills::delete_central_skill,
            commands::skills::get_skill_detail,
            commands::skills::read_skill_content,
            commands::skills::read_file_by_path,
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
            commands::settings::set_setting,
            // Discover
            commands::discover::discover_scan_roots,
            commands::discover::get_scan_roots,
            commands::discover::get_discovered_summary,
            commands::discover::set_scan_root_enabled,
            commands::discover::start_project_scan,
            commands::discover::stop_project_scan,
            commands::discover::get_discovered_skills,
            commands::discover::import_discovered_skill_to_central,
            commands::discover::import_discovered_skill_to_platform,
            commands::discover::clear_discovered_skills,
            commands::github_import::preview_github_repo_import,
            commands::github_import::import_github_repo_skills,
            commands::github_import::fetch_github_skill_markdown,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
