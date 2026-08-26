use super::*;
use crate::db::{self, OperationLogFilter};
use crate::observability::OperationLifecycle;
use sqlx::SqlitePool;
use std::sync::Arc;

fn test_app_state(pool: SqlitePool) -> AppState {
    AppState {
        db: pool,
        ai_tag_jobs: crate::AiTagJobRegistry::default(),
        central_update_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
            "job.central_update_busy",
            "A Central update job is already running.",
        ),
        central_update_snapshots: crate::CentralUpdateSnapshotCache::default(),
        portable_state_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
            "job.portability_busy",
            "A portability job is already running.",
        ),
        skills_cli_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
            "job.skills_cli_busy",
            "A Skills CLI job is already running.",
        ),
        secrets: Arc::new(crate::secrets::MockSecretStore::default()),
        targets: crate::targets::TargetRegistry::default(),
    }
}

#[test]
fn destructive_skill_commands_have_one_registered_operation_owner() {
    let source = include_str!("skills.rs");
    let production = source.split("#[cfg(test)]").next().unwrap();
    for command in [
        "delete_central_skill",
        "delete_central_skills",
        "reset_unknown_source_skills",
        "delete_skill_repository",
    ] {
        let entry = crate::ipc_registry::command_policy(command).unwrap();
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            panic!("{command} must have an Operation policy");
        };
        assert_eq!(
            definition.lifecycle(),
            OperationLifecycle::StartedThenTerminal
        );
        assert!(production.contains(&format!("\"{command}\"")));
    }
    assert!(!production.contains("OperationLogEvent"));
    assert!(!production.contains("record_operation_log_best_effort"));
    assert!(!production.contains(".subject("));
    assert!(!production.contains(".details("));

    let entry = crate::ipc_registry::command_policy("open_in_file_manager").unwrap();
    let CommandLogPolicy::Operation(definition) = entry.policy else {
        panic!("open_in_file_manager must have an Operation policy");
    };
    assert_eq!(definition.lifecycle(), OperationLifecycle::TerminalOnly);
    assert!(production.contains("operation_definition(\"open_in_file_manager\")"));
}

#[test]
fn reviewed_central_failure_does_not_retain_remote_transport_text() {
    let definition = operation_definition("delete_central_skill");
    let planted = r"C:\Users\private ssh stderr ghp_private_token";
    let failure = reviewed_central_failure(
        definition,
        &central_skills::CentralSkillsError::Remote(planted.to_string()),
    );
    let serialized = format!("{failure:?}");
    assert!(!serialized.contains(planted));
    assert!(serialized.contains("central_skills.remote_failed"));
}

#[tokio::test]
async fn open_file_manager_terminal_entry_contains_no_input_path() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let definition = operation_definition("open_in_file_manager");
    let planted_path = r"C:\Users\private\skills\secret";

    crate::observability::run_operation(
        &state,
        definition,
        OperationContext::new(OperationTarget::local()),
        |_| open_file_manager_result(),
        || async {
            let _business_input_stays_outside_audit = planted_path;
            Ok::<_, ReviewedFailure>(())
        },
    )
    .await
    .unwrap();

    let page = db::list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    let entry = &page.entries[0];
    assert_eq!(entry.action, "open_in_file_manager");
    assert_eq!(entry.status, "succeeded");
    let details: serde_json::Value =
        serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
    assert_eq!(details["operationId"], entry.id);
    assert_eq!(details["affectedCount"], 1);
    assert_eq!(details["mode"], "file_manager");
    assert!(!serde_json::to_string(entry).unwrap().contains(planted_path));
}

#[tokio::test]
async fn open_file_manager_failure_correlation_matches_safe_terminal_row() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let definition = operation_definition("open_in_file_manager");
    let planted = r"C:\Users\private ssh stderr ghp_private_token";
    let result = crate::observability::run_operation(
        &state,
        definition,
        OperationContext::new(OperationTarget::local()),
        |_| open_file_manager_result(),
        || async {
            Err::<(), _>(reviewed_file_open_failure(
                definition,
                &central_skills::CentralSkillsError::Remote(planted.to_string()),
            ))
        },
    )
    .await;
    let error = result.unwrap_err();
    assert_eq!(error.code, "central_skills.remote_failed");
    let correlation_id = error.correlation_id.unwrap();

    let page = db::list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].id, correlation_id);
    assert_eq!(page.entries[0].status, "failed");
    assert!(!serde_json::to_string(&page.entries[0])
        .unwrap()
        .contains(planted));
}
