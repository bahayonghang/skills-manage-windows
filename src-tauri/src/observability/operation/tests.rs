use super::*;
use crate::observability::OperationCategory;
use sqlx::SqlitePool;
use std::io::Write;
use std::sync::{Arc, Mutex};

fn definition(lifecycle: OperationLifecycle) -> OperationDefinition {
    OperationDefinition::registered(
        "test_operation",
        OperationCategory::Central,
        OperationPhase::Filesystem,
        lifecycle,
    )
}

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
        secrets: std::sync::Arc::new(crate::secrets::MockSecretStore::default()),
        targets: crate::targets::TargetRegistry::default(),
    }
}

#[test]
fn safe_identifier_rejects_paths_whitespace_and_long_values() {
    assert_eq!(SafeIdentifier::new("skill::id").into_string(), "skill::id");
    for value in [
        r"C:\Users\alice\private",
        "https://private.invalid/repo",
        "host name",
        &"x".repeat(161),
    ] {
        assert_eq!(SafeIdentifier::new(value).into_string(), "unknown");
    }
}

#[test]
fn operation_target_accepts_only_logical_target_ids_not_hosts() {
    assert_eq!(
        OperationTarget::new(OperationTargetKind::Ssh, "ssh-demo")
            .id
            .into_string(),
        "ssh-demo"
    );
    assert_eq!(
        OperationTarget::new(OperationTargetKind::Wsl, "wsl-demo")
            .id
            .into_string(),
        "wsl-demo"
    );
    for (kind, value) in [
        (OperationTargetKind::Ssh, "private.example.com"),
        (OperationTargetKind::Ssh, "10.0.0.1"),
        (OperationTargetKind::Wsl, "Ubuntu-Private"),
        (OperationTargetKind::Local, "another-local"),
    ] {
        assert_eq!(
            OperationTarget::new(kind, value).id.into_string(),
            "unknown"
        );
    }
}

#[tokio::test]
async fn terminal_success_uses_row_id_as_operation_id_and_is_searchable() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let batch_id = OperationBatchId::new();
    let context = OperationContext::new(OperationTarget::local())
        .subject(
            OperationSubjectKind::Skill,
            SafeIdentifier::new("skill::id"),
        )
        .batch(batch_id.clone());
    let value = run_operation(
        &state,
        definition(OperationLifecycle::TerminalOnly),
        context,
        |_| {
            SafeOperationResult::succeeded("Operation completed.")
                .count(SafeDetailKey::AffectedCount, 3)
        },
        || async { Ok::<_, ReviewedFailure>(7) },
    )
    .await
    .unwrap();
    assert_eq!(value, 7);

    let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    let entry = &page.entries[0];
    let details: Value = serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
    assert_eq!(details["operationId"], entry.id);
    assert_eq!(details["affectedCount"], 3);
    assert_eq!(entry.subject_type.as_deref(), Some("skill"));
    assert_eq!(entry.subject_id.as_deref(), Some("skill::id"));
    assert_eq!(entry.batch_id.as_deref(), Some(batch_id.as_str()));
    assert_ne!(entry.batch_id.as_deref(), Some(entry.id.as_str()));

    let exported = db::export_operation_logs_json(
        &pool,
        db::OperationLogFilter {
            operation_id: Some(entry.id.clone()),
            ..db::OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert!(exported.contains(&entry.id));

    let exact = db::list_operation_logs(
        &pool,
        db::OperationLogFilter {
            operation_id: Some(entry.id.clone()),
            ..db::OperationLogFilter::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(exact.total, 1);
}

#[tokio::test]
async fn started_operation_finishes_by_updating_the_same_row() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let batch_id = OperationBatchId::new();
    run_operation(
        &state,
        definition(OperationLifecycle::StartedThenTerminal),
        OperationContext::new(OperationTarget::local()).batch(batch_id.clone()),
        |_| SafeOperationResult::partial("Operation partially completed."),
        || async { Ok::<_, ReviewedFailure>(()) },
    )
    .await
    .unwrap();

    let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].status, "partial");
    assert_eq!(page.entries[0].batch_id.as_deref(), Some(batch_id.as_str()));
    assert!(page.entries[0].duration_ms.is_some());
}

#[tokio::test]
async fn reviewed_failure_has_correlation_without_raw_display_data() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let planted = r"C:\Users\alice\secret ghp_private ssh host output";
    let result = run_operation(
        &state,
        definition(OperationLifecycle::StartedThenTerminal),
        OperationTarget::local(),
        |_| SafeOperationResult::succeeded("Operation completed."),
        || async {
            let _raw_source_never_crosses_the_interface = planted;
            Err::<(), _>(ReviewedFailure::new(ReviewedDiagnostic::unexpected(
                definition(OperationLifecycle::StartedThenTerminal),
            )))
        },
    )
    .await;
    let error = result.unwrap_err();
    assert_eq!(error.code, "internal.unexpected");
    assert!(error.correlation_id.is_some());

    let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].status, "failed");
    let serialized = serde_json::to_string(&page.entries[0]).unwrap();
    assert!(!serialized.contains(planted));
    assert!(serialized.contains(error.correlation_id.as_deref().unwrap()));
}

#[tokio::test]
async fn reviewed_cancellation_records_cancelled_terminal_state() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let result = run_operation(
        &state,
        definition(OperationLifecycle::StartedThenTerminal),
        OperationTarget::local(),
        |_| SafeOperationResult::succeeded("Operation completed."),
        || async {
            Err::<(), _>(ReviewedFailure::new(ReviewedDiagnostic::new(
                "operation.cancelled",
                "operation.control",
                OperationPhase::Job,
                "The operation was cancelled.",
                false,
            )))
        },
    )
    .await;
    let error = result.unwrap_err();
    assert_eq!(error.code, "operation.cancelled");
    assert!(error.correlation_id.is_some());

    let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].status, "cancelled");
    assert_eq!(page.entries[0].level, "warn");
}

#[tokio::test]
async fn startup_sweep_marks_only_started_rows_interrupted() {
    let pool = crate::test_support::mem_pool().await;
    let started_id = OperationId::new();
    db::insert_operation_log_with_id(
        &pool,
        started_id.as_str(),
        started_entry(
            &started_id,
            definition(OperationLifecycle::StartedThenTerminal),
            &OperationContext::new(OperationTarget::local()),
        ),
    )
    .await
    .unwrap();
    record_terminal(
        &pool,
        definition(OperationLifecycle::TerminalOnly),
        OperationTarget::local(),
        SafeOperationResult::succeeded("Operation completed."),
    )
    .await;

    mark_interrupted_operations_best_effort(&pool).await;
    let started = db::get_operation_log(&pool, started_id.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(started.status, "interrupted");
    let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .filter(|entry| entry.status == "succeeded")
            .count(),
        1
    );
}

struct SharedLogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedLogBuffer {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn unavailable_log_storage_never_changes_business_result() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let writer = Arc::clone(&logs);
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || SharedLogBuffer(Arc::clone(&writer)))
        .with_ansi(false)
        .compact()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    // Exemption from test_support::mem_pool: this test needs no schema.
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let state = test_app_state(pool);
    let result = run_operation(
        &state,
        definition(OperationLifecycle::StartedThenTerminal),
        OperationTarget::local(),
        |_| SafeOperationResult::succeeded("Operation completed."),
        || async { Ok::<_, ReviewedFailure>(11) },
    )
    .await;
    assert_eq!(result.unwrap(), 11);

    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert_eq!(
        logged
            .matches("Could not persist operation lifecycle event")
            .count(),
        2,
        "started and terminal failures must each remain observable"
    );
    assert!(
        logged.contains("started"),
        "missing started phase: {logged}"
    );
    assert!(
        logged.contains("terminal"),
        "missing terminal phase: {logged}"
    );
    assert!(!logged.contains("no such table"));
}

#[test]
fn runtime_failure_context_supports_runtime_only_and_excluded_policies() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let writer = Arc::clone(&logs);
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || SharedLogBuffer(Arc::clone(&writer)))
        .with_ansi(false)
        .compact()
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);

    let planted = r"C:\Users\alice\private.log ghp_super_secret";
    let error = tracing::dispatcher::with_default(&dispatch, || {
        record_runtime_failure(
            RuntimeFailureContext::new(
                crate::ipc_registry::command_policy("get_central_skills").unwrap(),
            )
            .target_kind(OperationTargetKind::Local),
            IpcError::from(planted.to_string()),
        )
    });
    let correlation_id = error.correlation_id.as_deref().unwrap();
    assert!(Uuid::parse_str(correlation_id).is_ok());
    assert!(!serde_json::to_string(&error).unwrap().contains(planted));

    let excluded = tracing::dispatcher::with_default(&dispatch, || {
        record_runtime_failure(
            RuntimeFailureContext::new(
                crate::ipc_registry::command_policy("record_frontend_runtime_log").unwrap(),
            ),
            IpcError::new("internal.unexpected", "The operation failed.", false),
        )
    });
    assert!(excluded.correlation_id.is_none());

    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert_eq!(logged.matches("IPC operation failed").count(), 1);
    assert!(logged.contains("get_central_skills"));
    assert!(logged.contains("runtime"));
    assert!(logged.contains("command"));
    assert!(logged.contains(correlation_id));
    assert!(!logged.contains(planted));
}

#[test]
fn runtime_failure_reuses_the_operation_row_id() {
    let operation_id = OperationId::new();
    let error = record_runtime_failure(
        RuntimeFailureContext::new(
            crate::ipc_registry::command_policy("update_central_skills").unwrap(),
        ),
        IpcError::new("internal.unexpected", "The operation failed.", false)
            .with_correlation_id(operation_id.as_str()),
    );

    assert_eq!(error.correlation_id.as_deref(), Some(operation_id.as_str()));
}
