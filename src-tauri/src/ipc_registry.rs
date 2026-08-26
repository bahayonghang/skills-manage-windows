// Runtime command registration is declared once here and consumed by both
// Tauri's handler and compile-time parity inventories.

#[macro_export]
#[doc(hidden)]
macro_rules! __skillport_runtime_commands {
    ($callback:ident) => {
        $callback! {
            get_startup_status => (commands::startup::get_startup_status, runtime_only(ReadOnly)),
            retry_startup => (commands::startup::retry_startup, operation(Startup, Startup, StartedThenTerminal)),
            rebuild_startup_database => (commands::startup::rebuild_startup_database, operation(Startup, Recovery, StartedThenTerminal)),
            exit_startup => (commands::startup::exit_startup, operation(Startup, Startup, TerminalOnly)),
            mark_import_intent_frontend_ready => (commands::deep_link::mark_import_intent_frontend_ready, excluded(FrontendReadyBridge)),
            get_bootstrap_snapshot => (commands::bootstrap::get_bootstrap_snapshot, runtime_only(ReadOnly)),
            get_skill_counts_summary => (commands::bootstrap::get_skill_counts_summary, runtime_only(ReadOnly)),
            get_dashboard_central_summary => (commands::bootstrap::get_dashboard_central_summary, runtime_only(ReadOnly)),
            get_app_runtime_info => (commands::app_runtime::get_app_runtime_info, runtime_only(ReadOnly)),
            list_targets => (commands::targets::list_targets, runtime_only(ReadOnly)),
            get_target_config_quarantine_status => (commands::targets::get_target_config_quarantine_status, runtime_only(ReadOnly)),
            list_wsl_distributions => (commands::targets::list_wsl_distributions, runtime_only(ReadOnly)),
            create_ssh_target => (commands::targets::create_ssh_target, operation(Target, Database, TerminalOnly)),
            update_ssh_target => (commands::targets::update_ssh_target, operation(Target, Database, TerminalOnly)),
            test_ssh_target => (commands::targets::test_ssh_target, operation(Target, Network, StartedThenTerminal)),
            update_ssh_target_password => (commands::targets::update_ssh_target_password, operation(Secret, Database, TerminalOnly)),
            create_wsl_target => (commands::targets::create_wsl_target, operation(Target, Database, TerminalOnly)),
            update_wsl_target => (commands::targets::update_wsl_target, operation(Target, Database, TerminalOnly)),
            test_wsl_target => (commands::targets::test_wsl_target, operation(Target, Network, StartedThenTerminal)),
            delete_target => (commands::targets::delete_target, operation(Target, Database, TerminalOnly)),
            set_active_target => (commands::targets::set_active_target, operation(Target, Database, TerminalOnly)),
            get_active_target => (commands::targets::get_active_target, runtime_only(ReadOnly)),
            preview_local_remote_sync => (commands::local_remote_sync::preview_local_remote_sync, runtime_only(Preview)),
            apply_local_remote_sync => (commands::local_remote_sync::apply_local_remote_sync, operation(Sync, Filesystem, StartedThenTerminal)),
            list_operation_logs => (commands::logs::list_operation_logs, runtime_only(ReadOnly)),
            get_operation_log => (commands::logs::get_operation_log, runtime_only(ReadOnly)),
            clear_operation_logs => (commands::logs::clear_operation_logs, operation(Logs, Database, TerminalOnly)),
            export_operation_logs => (commands::logs::export_operation_logs, operation(Logs, Filesystem, StartedThenTerminal)),
            list_pending_fs_db_operations => (commands::logs::list_pending_fs_db_operations, runtime_only(ReadOnly)),
            retry_fs_db_operation => (commands::logs::retry_fs_db_operation, operation(Logs, Recovery, StartedThenTerminal)),
            preview_fs_db_operation_reconciliation => (commands::logs::preview_fs_db_operation_reconciliation, runtime_only(Preview)),
            reconcile_fs_db_operation => (commands::logs::reconcile_fs_db_operation, operation(Logs, Recovery, StartedThenTerminal)),
            get_daily_operation_counts => (commands::logs::get_daily_operation_counts, runtime_only(ReadOnly)),
            list_runtime_log_files => (commands::logs::list_runtime_log_files, runtime_only(ReadOnly)),
            read_runtime_log_file => (commands::logs::read_runtime_log_file, runtime_only(ReadOnly)),
            export_runtime_log_file => (commands::logs::export_runtime_log_file, operation(Logs, Filesystem, StartedThenTerminal)),
            clear_runtime_logs => (commands::logs::clear_runtime_logs, operation(Logs, Filesystem, StartedThenTerminal)),
            record_frontend_runtime_log => (commands::logs::record_frontend_runtime_log, excluded(SelfLogging)),
            scan_all_skills => (commands::scanner::scan_all_skills, operation(Scan, Filesystem, StartedThenTerminal)),
            get_agents => (commands::agents::get_agents, runtime_only(ReadOnly)),
            detect_agents => (commands::agents::detect_agents, operation(Agent, Filesystem, StartedThenTerminal)),
            list_platform_paths => (commands::agents::list_platform_paths, runtime_only(ReadOnly)),
            add_custom_agent => (commands::agents::add_custom_agent, operation(Agent, Database, TerminalOnly)),
            update_custom_agent => (commands::agents::update_custom_agent, operation(Agent, Database, TerminalOnly)),
            remove_custom_agent => (commands::agents::remove_custom_agent, operation(Agent, Database, TerminalOnly)),
            set_agent_enabled => (commands::agents::set_agent_enabled, operation(Agent, Database, TerminalOnly)),
            install_skill_to_agent => (commands::linker::install_skill_to_agent, operation(Install, Filesystem, StartedThenTerminal)),
            uninstall_skill_from_agent => (commands::linker::uninstall_skill_from_agent, operation(Install, Filesystem, StartedThenTerminal)),
            batch_uninstall_skills_from_agent => (commands::linker::batch_uninstall_skills_from_agent, operation(Install, Filesystem, StartedThenTerminal)),
            batch_install_to_agents => (commands::linker::batch_install_to_agents, operation(Install, Filesystem, StartedThenTerminal)),
            batch_install_central_skills => (commands::linker::batch_install_central_skills, operation(Install, Filesystem, StartedThenTerminal)),
            get_skills_by_agent => (commands::skills::get_skills_by_agent, runtime_only(ReadOnly)),
            get_central_skills => (commands::skills::get_central_skills, runtime_only(ReadOnly)),
            get_central_skills_page => (commands::skills::get_central_skills_page, runtime_only(ReadOnly)),
            preview_delete_central_skills => (commands::skills::preview_delete_central_skills, runtime_only(Preview)),
            delete_central_skill => (commands::skills::delete_central_skill, operation(Central, Filesystem, StartedThenTerminal)),
            delete_central_skills => (commands::skills::delete_central_skills, operation(Central, Filesystem, StartedThenTerminal)),
            preview_reset_unknown_source_skills => (commands::skills::preview_reset_unknown_source_skills, runtime_only(Preview)),
            reset_unknown_source_skills => (commands::skills::reset_unknown_source_skills, operation(Central, Filesystem, StartedThenTerminal)),
            preview_delete_skill_repository => (commands::skills::preview_delete_skill_repository, runtime_only(Preview)),
            delete_skill_repository => (commands::skills::delete_skill_repository, operation(Central, Filesystem, StartedThenTerminal)),
            get_skill_detail => (commands::skills::get_skill_detail, runtime_only(ReadOnly)),
            read_skill_content => (commands::skills::read_skill_content, runtime_only(ReadOnly)),
            read_file_by_path => (commands::skills::read_file_by_path, runtime_only(ReadOnly)),
            list_directory_tree => (commands::skills::list_directory_tree, runtime_only(ReadOnly)),
            open_in_file_manager => (commands::skills::open_in_file_manager, operation(Central, Filesystem, TerminalOnly)),
            get_skill_repositories => (commands::central_metadata::get_skill_repositories, runtime_only(ReadOnly)),
            create_or_update_skill_repository => (commands::central_metadata::create_or_update_skill_repository, operation(Catalog, Database, TerminalOnly)),
            assign_skills_to_repository => (commands::central_metadata::assign_skills_to_repository, operation(Catalog, Database, TerminalOnly)),
            set_skill_repository_pinned => (commands::central_metadata::set_skill_repository_pinned, operation(Catalog, Database, TerminalOnly)),
            get_skill_tags => (commands::central_metadata::get_skill_tags, runtime_only(ReadOnly)),
            get_central_top_tags => (commands::central_metadata::get_central_top_tags, runtime_only(ReadOnly)),
            create_skill_tag => (commands::central_metadata::create_skill_tag, operation(Catalog, Database, TerminalOnly)),
            assign_skill_tags => (commands::central_metadata::assign_skill_tags, operation(Catalog, Database, TerminalOnly)),
            unassign_skill_tags => (commands::central_metadata::unassign_skill_tags, operation(Catalog, Database, TerminalOnly)),
            suggest_skill_tags => (commands::central_metadata::suggest_skill_tags, operation(Catalog, Database, TerminalOnly)),
            bulk_suggest_skill_tags => (commands::central_metadata::bulk_suggest_skill_tags, operation(Catalog, Database, TerminalOnly)),
            cancel_ai_tag_job => (commands::central_metadata::cancel_ai_tag_job, operation(Catalog, Job, TerminalOnly)),
            get_pending_ai_tag_reviews => (commands::central_metadata::get_pending_ai_tag_reviews, runtime_only(ReadOnly)),
            accept_ai_tag_review => (commands::central_metadata::accept_ai_tag_review, operation(Catalog, Database, TerminalOnly)),
            skip_ai_tag_review => (commands::central_metadata::skip_ai_tag_review, operation(Catalog, Database, TerminalOnly)),
            preview_central_store_location_change => (commands::central_store_location::preview_central_store_location_change, runtime_only(Preview)),
            apply_central_store_location_change => (commands::central_store_location::apply_central_store_location_change, operation(Central, Filesystem, StartedThenTerminal)),
            get_central_skill_update_states => (commands::central_updates::get_central_skill_update_states, runtime_only(ReadOnly)),
            check_central_skill_updates => (commands::central_updates::check_central_skill_updates, operation(Update, Network, StartedThenTerminal)),
            check_central_repository_sync => (commands::central_updates::check_central_repository_sync, operation(Update, Network, StartedThenTerminal)),
            apply_central_repository_sync => (commands::central_updates::apply_central_repository_sync, operation(Update, Filesystem, StartedThenTerminal)),
            update_central_skills => (commands::central_updates::update_central_skills, operation(Update, Filesystem, StartedThenTerminal)),
            cancel_central_skill_updates => (commands::central_updates::cancel_central_skill_updates, operation(Update, Job, TerminalOnly)),
            keep_remote_missing_central_skills => (commands::central_updates::keep_remote_missing_central_skills, operation(Update, Database, TerminalOnly)),
            refresh_skill_update_inventory => (commands::skill_update_inventory::refresh_skill_update_inventory, operation(Update, Network, StartedThenTerminal)),
            retry_failed_update_repositories => (commands::skill_update_inventory::retry_failed_update_repositories, operation(Update, Network, StartedThenTerminal)),
            get_skill_update_inventory => (commands::skill_update_inventory::get_skill_update_inventory, runtime_only(ReadOnly)),
            clear_skill_update_inventory => (commands::skill_update_inventory::clear_skill_update_inventory, operation(Update, Database, TerminalOnly)),
            apply_skill_update_decisions => (commands::skill_update_inventory::apply_skill_update_decisions, operation(Update, Filesystem, StartedThenTerminal)),
            force_update_central_skills => (commands::skill_update_inventory::force_update_central_skills, operation(Update, Network, StartedThenTerminal)),
            force_mirror_central_repositories => (commands::skill_update_inventory::force_mirror_central_repositories, operation(Update, Network, StartedThenTerminal)),
            scan_platform_duplicate_skills => (commands::skill_update_inventory::scan_platform_duplicate_skills, operation(Update, Filesystem, StartedThenTerminal)),
            scan_deleted_platform_copies => (commands::skill_update_inventory::scan_deleted_platform_copies, operation(Update, Filesystem, StartedThenTerminal)),
            create_collection => (commands::collections::create_collection, operation(Catalog, Database, TerminalOnly)),
            get_collections => (commands::collections::get_collections, runtime_only(ReadOnly)),
            get_collection_detail => (commands::collections::get_collection_detail, runtime_only(ReadOnly)),
            add_skill_to_collection => (commands::collections::add_skill_to_collection, operation(Catalog, Database, TerminalOnly)),
            remove_skill_from_collection => (commands::collections::remove_skill_from_collection, operation(Catalog, Database, TerminalOnly)),
            delete_collection => (commands::collections::delete_collection, operation(Catalog, Database, TerminalOnly)),
            update_collection => (commands::collections::update_collection, operation(Catalog, Database, TerminalOnly)),
            batch_install_collection => (commands::collections::batch_install_collection, operation(Catalog, Filesystem, StartedThenTerminal)),
            export_collection => (commands::collections::export_collection, operation(Catalog, Filesystem, StartedThenTerminal)),
            import_collection => (commands::collections::import_collection, operation(Catalog, Filesystem, StartedThenTerminal)),
            get_scan_directories => (commands::settings::get_scan_directories, runtime_only(ReadOnly)),
            add_scan_directory => (commands::settings::add_scan_directory, operation(Settings, Database, TerminalOnly)),
            remove_scan_directory => (commands::settings::remove_scan_directory, operation(Settings, Database, TerminalOnly)),
            set_scan_directory_active => (commands::settings::set_scan_directory_active, operation(Settings, Database, TerminalOnly)),
            get_setting => (commands::settings::get_setting, runtime_only(ReadOnly)),
            get_settings => (commands::settings::get_settings, runtime_only(ReadOnly)),
            set_setting => (commands::settings::set_setting, operation(Settings, Database, TerminalOnly)),
            set_settings => (commands::settings::set_settings, operation(Settings, Database, TerminalOnly)),
            get_ai_api_key_state => (commands::settings::get_ai_api_key_state, runtime_only(ReadOnly)),
            set_ai_api_key => (commands::settings::set_ai_api_key, operation(Secret, Database, TerminalOnly)),
            clear_ai_api_key => (commands::settings::clear_ai_api_key, operation(Secret, Database, TerminalOnly)),
            get_github_pat => (commands::github_import::get_github_pat, runtime_only(ReadOnly)),
            set_github_pat => (commands::github_import::set_github_pat, operation(Secret, Database, TerminalOnly)),
            clear_github_pat => (commands::github_import::clear_github_pat, operation(Secret, Database, TerminalOnly)),
            test_github_pat => (commands::github_import::test_github_pat, operation(Secret, Network, StartedThenTerminal)),
            get_obsidian_vaults => (commands::obsidian::get_obsidian_vaults, runtime_only(ReadOnly)),
            get_obsidian_vault_skills => (commands::obsidian::get_obsidian_vault_skills, runtime_only(ReadOnly)),
            open_obsidian_path => (commands::obsidian::open_obsidian_path, operation(Obsidian, Filesystem, TerminalOnly)),
            import_obsidian_skill_to_central => (commands::obsidian::import_obsidian_skill_to_central, operation(Obsidian, Filesystem, StartedThenTerminal)),
            import_obsidian_skill_to_platform => (commands::obsidian::import_obsidian_skill_to_platform, operation(Obsidian, Filesystem, StartedThenTerminal)),
            preview_github_repo_import => (commands::github_import::preview_github_repo_import, runtime_only(Preview)),
            import_github_repo_skills => (commands::github_import::import_github_repo_skills, operation(Import, Network, StartedThenTerminal)),
            fetch_github_skill_markdown => (commands::github_import::fetch_github_skill_markdown, runtime_only(ReadOnly)),
            discard_github_repo_preview_snapshot => (commands::github_import::discard_github_repo_preview_snapshot, runtime_only(InternalRefresh)),
            preview_local_skill_archive => (commands::local_archive_import::preview_local_skill_archive, runtime_only(Preview)),
            import_local_skill_archive => (commands::local_archive_import::import_local_skill_archive, operation(Import, Filesystem, StartedThenTerminal)),
            pick_project_folder => (commands::projects::pick_project_folder, operation(Project, Filesystem, StartedThenTerminal)),
            add_project => (commands::projects::add_project, operation(Project, Database, TerminalOnly)),
            list_projects => (commands::projects::list_projects, runtime_only(ReadOnly)),
            rename_project => (commands::projects::rename_project, operation(Project, Database, TerminalOnly)),
            set_project_pinned => (commands::projects::set_project_pinned, operation(Project, Database, TerminalOnly)),
            rescan_project => (commands::projects::rescan_project, operation(Project, Filesystem, StartedThenTerminal)),
            get_project_skills => (commands::projects::get_project_skills, runtime_only(ReadOnly)),
            remove_project => (commands::projects::remove_project, operation(Project, Database, TerminalOnly)),
            install_skill_to_project => (commands::projects::install_skill_to_project, operation(Project, Filesystem, StartedThenTerminal)),
            uninstall_skill_from_project => (commands::projects::uninstall_skill_from_project, operation(Project, Filesystem, StartedThenTerminal)),
            list_projects_using_skill => (commands::projects::list_projects_using_skill, runtime_only(ReadOnly)),
            list_registries => (commands::marketplace::list_registries, runtime_only(ReadOnly)),
            add_registry => (commands::marketplace::add_registry, operation(Marketplace, Database, TerminalOnly)),
            remove_registry => (commands::marketplace::remove_registry, operation(Marketplace, Database, TerminalOnly)),
            sync_registry => (commands::marketplace::sync_registry, operation(Marketplace, Network, StartedThenTerminal)),
            sync_registry_with_options => (commands::marketplace::sync_registry_with_options, operation(Marketplace, Network, StartedThenTerminal)),
            search_marketplace_skills => (commands::marketplace::search_marketplace_skills, runtime_only(ReadOnly)),
            install_marketplace_skill => (commands::marketplace::install_marketplace_skill, operation(Marketplace, Network, StartedThenTerminal)),
            search_skills_sh => (commands::marketplace::search_skills_sh, runtime_only(ReadOnly)),
            resolve_skills_sh_url => (commands::marketplace::resolve_skills_sh_url, runtime_only(ReadOnly)),
            browse_skills_sh_directory => (commands::marketplace::browse_skills_sh_directory, runtime_only(ReadOnly)),
            read_skills_sh_file => (commands::marketplace::read_skills_sh_file, runtime_only(ReadOnly)),
            install_from_skills_sh => (commands::marketplace::install_from_skills_sh, operation(Marketplace, Network, StartedThenTerminal)),
            explain_skill => (commands::marketplace::explain_skill, operation(Marketplace, Network, StartedThenTerminal)),
            test_ai_connection => (commands::marketplace::test_ai_connection, operation(Marketplace, Network, StartedThenTerminal)),
            get_skill_explanation => (commands::marketplace::get_skill_explanation, runtime_only(ReadOnly)),
            get_skill_explanation_summaries => (commands::marketplace::get_skill_explanation_summaries, runtime_only(ReadOnly)),
            explain_skill_stream => (commands::marketplace::explain_skill_stream, operation(Marketplace, Network, StartedThenTerminal)),
            refresh_skill_explanation => (commands::marketplace::refresh_skill_explanation, operation(Marketplace, Network, StartedThenTerminal)),
            export_skillport_state => (commands::portable_state::export_skillport_state, operation(PortableState, Filesystem, StartedThenTerminal)),
            preview_skillport_state_import => (commands::portable_state::preview_skillport_state_import, runtime_only(Preview)),
            preview_skillport_state_import_file => (commands::portable_state::preview_skillport_state_import_file, runtime_only(Preview)),
            save_skillport_state_export => (commands::portable_state::save_skillport_state_export, operation(PortableState, Filesystem, StartedThenTerminal)),
            import_skillport_state => (commands::portable_state::import_skillport_state, operation(PortableState, Filesystem, StartedThenTerminal)),
            cancel_skillport_state_portability => (commands::portable_state::cancel_skillport_state_portability, operation(PortableState, Job, TerminalOnly)),
            list_saved_views => (commands::saved_views::list_saved_views, runtime_only(ReadOnly)),
            create_saved_view => (commands::saved_views::create_saved_view, operation(Catalog, Database, TerminalOnly)),
            update_saved_view => (commands::saved_views::update_saved_view, operation(Catalog, Database, TerminalOnly)),
            delete_saved_view => (commands::saved_views::delete_saved_view, operation(Catalog, Database, TerminalOnly)),
            reorder_saved_views => (commands::saved_views::reorder_saved_views, operation(Catalog, Database, TerminalOnly)),
            list_tag_groups => (commands::tag_groups::list_tag_groups, runtime_only(ReadOnly)),
            create_tag_group => (commands::tag_groups::create_tag_group, operation(Catalog, Database, TerminalOnly)),
            update_tag_group => (commands::tag_groups::update_tag_group, operation(Catalog, Database, TerminalOnly)),
            delete_tag_group => (commands::tag_groups::delete_tag_group, operation(Catalog, Database, TerminalOnly)),
            reorder_tag_groups => (commands::tag_groups::reorder_tag_groups, operation(Catalog, Database, TerminalOnly)),
            set_tag_group => (commands::tag_groups::set_tag_group, operation(Catalog, Database, TerminalOnly)),
            usage_refresh => (commands::usage::usage_refresh, operation(Usage, Database, TerminalOnly)),
            usage_get_overview => (commands::usage::usage_get_overview, runtime_only(ReadOnly)),
            usage_get_recent => (commands::usage::usage_get_recent, runtime_only(ReadOnly)),
            usage_get_providers => (commands::usage::usage_get_providers, runtime_only(ReadOnly)),
            usage_get_skill_detail => (commands::usage::usage_get_skill_detail, runtime_only(ReadOnly)),
            usage_get_skill_counts => (commands::usage::usage_get_skill_counts, runtime_only(ReadOnly)),
            usage_get_skill_usage_stats => (commands::usage::usage_get_skill_usage_stats, runtime_only(ReadOnly)),
            usage_resolve_skill_id => (commands::usage::usage_resolve_skill_id, runtime_only(ReadOnly)),
            usage_get_scope_info => (commands::usage::usage_get_scope_info, runtime_only(ReadOnly)),
            usage_get_unused_skills => (commands::usage::usage_get_unused_skills, runtime_only(ReadOnly)),
            skills_cli_doctor => (commands::skills_cli::skills_cli_doctor, runtime_only(ReadOnly)),
            skills_cli_list_global => (commands::skills_cli::skills_cli_list_global, runtime_only(ReadOnly)),
            skills_cli_install_targets => (commands::skills_cli::skills_cli_install_targets, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_preview_source => (commands::skills_cli::skills_cli_preview_source, runtime_only(Preview)),
            skills_cli_add_global => (commands::skills_cli::skills_cli_add_global, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_remove_global => (commands::skills_cli::skills_cli_remove_global, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_read_skill_md => (commands::skills_cli::skills_cli_read_skill_md, runtime_only(ReadOnly)),
            skills_cli_link_platform => (commands::skills_cli::skills_cli_link_platform, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_unlink_platform => (commands::skills_cli::skills_cli_unlink_platform, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_reveal_skill_folder => (commands::skills_cli::skills_cli_reveal_skill_folder, operation(SkillsCli, Filesystem, TerminalOnly)),
            skills_cli_export_inventory => (commands::skills_cli::skills_cli_export_inventory, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_preview_remove_global => (commands::skills_cli::skills_cli_preview_remove_global, runtime_only(Preview)),
            cancel_skills_cli_job => (commands::skills_cli::cancel_skills_cli_job, operation(SkillsCli, Job, TerminalOnly)),
            skills_cli_check_updates => (commands::skills_cli::skills_cli_check_updates, operation(SkillsCli, Network, StartedThenTerminal)),
            skills_cli_update_inventory => (commands::skills_cli::skills_cli_update_inventory, runtime_only(ReadOnly)),
            skills_cli_verify_update_baseline => (commands::skills_cli::skills_cli_verify_update_baseline, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_apply_updates => (commands::skills_cli::skills_cli_apply_updates, operation(SkillsCli, Filesystem, StartedThenTerminal)),
            skills_cli_retry_update_recovery => (commands::skills_cli::skills_cli_retry_update_recovery, operation(SkillsCli, Recovery, StartedThenTerminal)),
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __skillport_generated_commands {
    ($callback:ident) => {
        $callback! {
            preview_local_remote_sync => commands::local_remote_sync::preview_local_remote_sync,
            apply_local_remote_sync => commands::local_remote_sync::apply_local_remote_sync,
            install_skill_to_agent => commands::linker::install_skill_to_agent,
            batch_install_to_agents => commands::linker::batch_install_to_agents,
            batch_install_central_skills => commands::linker::batch_install_central_skills,
            delete_central_skill => commands::skills::delete_central_skill,
            delete_central_skills => commands::skills::delete_central_skills,
            delete_skill_repository => commands::skills::delete_skill_repository,
            unassign_skill_tags => commands::central_metadata::unassign_skill_tags,
            preview_central_store_location_change => commands::central_store_location::preview_central_store_location_change,
            apply_central_store_location_change => commands::central_store_location::apply_central_store_location_change,
            get_central_skill_update_states => commands::central_updates::get_central_skill_update_states,
            apply_central_repository_sync => commands::central_updates::apply_central_repository_sync,
            keep_remote_missing_central_skills => commands::central_updates::keep_remote_missing_central_skills,
            refresh_skill_update_inventory => commands::skill_update_inventory::refresh_skill_update_inventory,
            retry_failed_update_repositories => commands::skill_update_inventory::retry_failed_update_repositories,
            get_skill_update_inventory => commands::skill_update_inventory::get_skill_update_inventory,
            clear_skill_update_inventory => commands::skill_update_inventory::clear_skill_update_inventory,
            force_update_central_skills => commands::skill_update_inventory::force_update_central_skills,
            force_mirror_central_repositories => commands::skill_update_inventory::force_mirror_central_repositories,
            scan_platform_duplicate_skills => commands::skill_update_inventory::scan_platform_duplicate_skills,
            scan_deleted_platform_copies => commands::skill_update_inventory::scan_deleted_platform_copies,
            remove_skill_from_collection => commands::collections::remove_skill_from_collection,
            delete_collection => commands::collections::delete_collection,
            batch_install_collection => commands::collections::batch_install_collection,
            import_collection => commands::collections::import_collection,
            remove_scan_directory => commands::settings::remove_scan_directory,
            get_ai_api_key_state => commands::settings::get_ai_api_key_state,
            set_ai_api_key => commands::settings::set_ai_api_key,
            clear_ai_api_key => commands::settings::clear_ai_api_key,
            get_github_pat => commands::github_import::get_github_pat,
            set_github_pat => commands::github_import::set_github_pat,
            clear_github_pat => commands::github_import::clear_github_pat,
            test_github_pat => commands::github_import::test_github_pat,
            import_obsidian_skill_to_central => commands::obsidian::import_obsidian_skill_to_central,
            import_obsidian_skill_to_platform => commands::obsidian::import_obsidian_skill_to_platform,
            remove_project => commands::projects::remove_project,
            install_skill_to_project => commands::projects::install_skill_to_project,
            uninstall_skill_from_project => commands::projects::uninstall_skill_from_project,
            remove_registry => commands::marketplace::remove_registry,
            install_marketplace_skill => commands::marketplace::install_marketplace_skill,
            install_from_skills_sh => commands::marketplace::install_from_skills_sh,
            test_ai_connection => commands::marketplace::test_ai_connection,
            skills_cli_doctor => commands::skills_cli::skills_cli_doctor,
            skills_cli_list_global => commands::skills_cli::skills_cli_list_global,
            skills_cli_install_targets => commands::skills_cli::skills_cli_install_targets,
            skills_cli_preview_source => commands::skills_cli::skills_cli_preview_source,
            skills_cli_add_global => commands::skills_cli::skills_cli_add_global,
            skills_cli_remove_global => commands::skills_cli::skills_cli_remove_global,
            skills_cli_read_skill_md => commands::skills_cli::skills_cli_read_skill_md,
            skills_cli_link_platform => commands::skills_cli::skills_cli_link_platform,
            skills_cli_unlink_platform => commands::skills_cli::skills_cli_unlink_platform,
            skills_cli_reveal_skill_folder => commands::skills_cli::skills_cli_reveal_skill_folder,
            skills_cli_export_inventory => commands::skills_cli::skills_cli_export_inventory,
            skills_cli_preview_remove_global => commands::skills_cli::skills_cli_preview_remove_global,
            cancel_skills_cli_job => commands::skills_cli::cancel_skills_cli_job,
            skills_cli_check_updates => commands::skills_cli::skills_cli_check_updates,
            skills_cli_update_inventory => commands::skills_cli::skills_cli_update_inventory,
            skills_cli_verify_update_baseline => commands::skills_cli::skills_cli_verify_update_baseline,
            skills_cli_apply_updates => commands::skills_cli::skills_cli_apply_updates,
            skills_cli_retry_update_recovery => commands::skills_cli::skills_cli_retry_update_recovery,
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __skillport_build_handler {
    ($($name:ident => ($command:path, $policy_kind:ident($($policy_arg:ident),*)),)+) => {
        tauri::generate_handler![$($command),+]
    };
}

#[macro_export]
macro_rules! runtime_command_handler {
    () => {
        $crate::__skillport_runtime_commands!(__skillport_build_handler)
    };
}

macro_rules! build_runtime_command_names {
    ($($name:ident => ($command:path, $policy_kind:ident($($policy_arg:ident),*)),)+) => {
        &[$(stringify!($name)),+]
    };
}

macro_rules! build_generated_command_names {
    ($($name:ident => $command:path,)+) => {
        &[$(stringify!($name)),+]
    };
}

macro_rules! command_policy_entry {
    ($name:expr, operation($category:ident, $phase:ident, $lifecycle:ident)) => {
        crate::observability::CommandPolicyEntry::operation(
            $name,
            crate::observability::OperationCategory::$category,
            crate::observability::OperationPhase::$phase,
            crate::observability::OperationLifecycle::$lifecycle,
        )
    };
    ($name:expr, runtime_only($reason:ident)) => {
        crate::observability::CommandPolicyEntry::runtime_only(
            $name,
            crate::observability::RuntimeOnlyReason::$reason,
        )
    };
    ($name:expr, excluded($reason:ident)) => {
        crate::observability::CommandPolicyEntry::excluded(
            $name,
            crate::observability::ExclusionReason::$reason,
        )
    };
}

macro_rules! build_command_policies {
    ($($name:ident => ($command:path, $policy_kind:ident($($policy_arg:ident),*)),)+) => {
        &[$(
            command_policy_entry!(
                stringify!($name),
                $policy_kind($($policy_arg),*)
            )
        ),+]
    };
}

pub const RUNTIME_COMMAND_NAMES: &[&str] =
    crate::__skillport_runtime_commands!(build_runtime_command_names);
pub const GENERATED_COMMAND_NAMES: &[&str] =
    crate::__skillport_generated_commands!(build_generated_command_names);
pub const RUNTIME_COMMAND_POLICIES: &[crate::observability::CommandPolicyEntry] =
    crate::__skillport_runtime_commands!(build_command_policies);

pub fn command_policy(command: &str) -> Option<&'static crate::observability::CommandPolicyEntry> {
    RUNTIME_COMMAND_POLICIES
        .iter()
        .find(|entry| entry.command == command)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::observability::{
        CommandLogPolicy, ExclusionReason, OperationCategory, OperationLifecycle, RuntimeOnlyReason,
    };

    #[test]
    fn every_runtime_command_has_one_policy_from_the_same_registry() {
        assert_eq!(RUNTIME_COMMAND_NAMES.len(), RUNTIME_COMMAND_POLICIES.len());
        let unique_names: HashSet<_> = RUNTIME_COMMAND_NAMES.iter().copied().collect();
        assert_eq!(unique_names.len(), RUNTIME_COMMAND_NAMES.len());
        for (name, entry) in RUNTIME_COMMAND_NAMES
            .iter()
            .zip(RUNTIME_COMMAND_POLICIES.iter())
        {
            assert_eq!(*name, entry.command);
            assert_eq!(command_policy(name), Some(entry));
        }
    }

    #[test]
    fn controlled_exclusions_are_narrow_and_explicit() {
        let excluded: Vec<_> = RUNTIME_COMMAND_POLICIES
            .iter()
            .filter_map(|entry| match entry.policy {
                CommandLogPolicy::Excluded(reason) => Some((entry.command, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(
            excluded,
            vec![
                (
                    "mark_import_intent_frontend_ready",
                    ExclusionReason::FrontendReadyBridge,
                ),
                ("record_frontend_runtime_log", ExclusionReason::SelfLogging,),
            ]
        );
    }

    #[test]
    fn representative_policies_preserve_product_semantics() {
        assert!(matches!(
            command_policy("get_central_skills").unwrap().policy,
            CommandLogPolicy::RuntimeOnly(RuntimeOnlyReason::ReadOnly)
        ));
        assert!(matches!(
            command_policy("preview_github_repo_import").unwrap().policy,
            CommandLogPolicy::RuntimeOnly(RuntimeOnlyReason::Preview)
        ));
        let policy = command_policy("update_central_skills").unwrap().policy;
        let CommandLogPolicy::Operation(definition) = policy else {
            panic!("update command must be auditable");
        };
        assert_eq!(definition.category(), OperationCategory::Update);
        assert_eq!(
            definition.lifecycle(),
            OperationLifecycle::StartedThenTerminal
        );
        assert!(matches!(
            command_policy("skills_cli_doctor").unwrap().policy,
            CommandLogPolicy::RuntimeOnly(RuntimeOnlyReason::ReadOnly)
        ));
        for command in [
            "skills_cli_preview_source",
            "skills_cli_preview_remove_global",
        ] {
            assert!(matches!(
                command_policy(command).unwrap().policy,
                CommandLogPolicy::RuntimeOnly(RuntimeOnlyReason::Preview)
            ));
        }
    }
}
