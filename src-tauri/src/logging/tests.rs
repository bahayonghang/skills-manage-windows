use super::*;
use serde_json::json;

#[test]
fn logs_dir_is_under_app_data_dir() {
    assert!(logs_dir().ends_with(".skillsmanage/logs"));
}

#[test]
fn daily_log_writer_writes_skillport_dated_log_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut appender = DailyLogWriter::new(temp_dir.path().to_path_buf());

    appender
        .write_all(b"SkillPort file logging initialized\n")
        .unwrap();
    appender.flush().unwrap();

    let entries = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].starts_with("skillport-"));
    assert!(entries[0].ends_with(".log"));
    let content = fs::read_to_string(temp_dir.path().join(&entries[0])).unwrap();
    assert!(content.contains("SkillPort file logging initialized"));
}

#[test]
fn operation_log_failure_warning_enters_daily_log_file() {
    // M6 稳定化：直接测 tracing → DailyLogWriter 集成链路，不经过 sqlx / tokio runtime。
    //
    // 原实现触发 `record_operation_log_best_effort(:memory:)` 的失败路径，但跨
    // test-thread + sqlx spawn_blocking 会使 thread-local dispatcher 不稳定，
    // 在并发跑整个 lib 测试时偶发失败。错误路径本身已在 operation_log 模块测试中
    // 覆盖（`records_failure_when_insert_errors`），此处只需验证 warn! 能穿过
    // DailyLogWriter 写入每日日志文件。
    let temp_dir = tempfile::tempdir().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let make_writer = move || DailyLogWriter::new(log_dir.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_writer(make_writer)
        .with_ansi(false)
        .compact()
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);

    tracing::dispatcher::with_default(&dispatch, || {
        tracing::warn!("Failed to record operation log: simulated error");
    });

    let entries = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let content = fs::read_to_string(temp_dir.path().join(&entries[0])).unwrap();
    assert!(content.contains("Failed to record operation log"));
}

#[test]
fn runtime_log_file_name_whitelist_rejects_path_traversal() {
    validate_runtime_log_file_name("skillport-2026-06-03.log").unwrap();
    assert!(validate_runtime_log_file_name("../skillport-2026-06-03.log").is_err());
    assert!(validate_runtime_log_file_name("skillport-2026-6-3.log").is_err());
    assert!(validate_runtime_log_file_name("other-2026-06-03.log").is_err());
}

#[test]
fn lists_only_skillport_runtime_log_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("skillport-2026-06-03.log"), "today").unwrap();
    fs::write(temp_dir.path().join("notes.log"), "ignored").unwrap();

    let files = list_runtime_log_files_in_dir(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name, "skillport-2026-06-03.log");
    assert_eq!(files[0].date, "2026-06-03");
}

#[test]
fn reads_runtime_log_with_filters_pagination_and_redaction() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
            temp_dir.path().join("skillport-2026-06-03.log"),
            concat!(
                "2026-06-03T00:00:00Z INFO skillport::startup: ready token=abc\n",
                "2026-06-03T00:01:00Z WARN skillport::frontend: bad source=window.error password=secret\n",
                "2026-06-03T00:02:00Z ERROR skillport::backend: failed api_key=sk-test\n"
            ),
        )
        .unwrap();

    let result = read_runtime_log_file_from_dir(
        temp_dir.path(),
        RuntimeLogReadRequest {
            file_name: "skillport-2026-06-03.log".to_string(),
            query: Some("bad".to_string()),
            level: Some("warn".to_string()),
            source: Some("window".to_string()),
            limit: Some(10),
            offset: Some(0),
            tail: false,
        },
    )
    .unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.lines[0].line_number, 2);
    assert_eq!(result.lines[0].level.as_deref(), Some("warn"));
    assert_eq!(result.lines[0].source, "window.error");
    assert!(result.lines[0].raw.contains("password=[REDACTED]"));
    assert!(!result.lines[0].raw.contains("secret"));
}

#[test]
fn runtime_tail_reads_last_matching_lines() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("skillport-2026-06-03.log"),
        "INFO first\nINFO second\nINFO third\n",
    )
    .unwrap();

    let result = read_runtime_log_file_from_dir(
        temp_dir.path(),
        RuntimeLogReadRequest {
            file_name: "skillport-2026-06-03.log".to_string(),
            query: None,
            level: Some("info".to_string()),
            source: None,
            limit: Some(2),
            offset: Some(0),
            tail: true,
        },
    )
    .unwrap();

    assert_eq!(result.total, 3);
    assert_eq!(result.offset, 1);
    assert_eq!(result.lines[0].line_number, 2);
    assert_eq!(result.lines[1].line_number, 3);
}

#[test]
fn export_runtime_log_file_redacts_sensitive_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("skillport-2026-06-03.log"),
        "INFO token=abc {\"apiKey\":\"sk-test\"}\nWARN connect passphrase=pp-secret\n",
    )
    .unwrap();

    let exported =
        export_runtime_log_file_from_dir(temp_dir.path(), "skillport-2026-06-03.log").unwrap();

    assert!(exported.contains("token=[REDACTED]"));
    assert!(exported.contains("\"apiKey\":\"[REDACTED]\""));
    assert!(exported.contains("passphrase=[REDACTED]"));
    assert!(!exported.contains("sk-test"));
    assert!(!exported.contains("pp-secret"));
}

#[test]
fn clear_runtime_logs_deletes_only_matching_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("skillport-2026-06-01.log"), "old").unwrap();
    fs::write(temp_dir.path().join("keep.txt"), "keep").unwrap();

    let deleted = clear_runtime_logs_in_dir(
        temp_dir.path(),
        RuntimeLogClearRequest {
            file_name: None,
            older_than_days: None,
            all: Some(true),
        },
    )
    .unwrap();

    assert_eq!(deleted, 1);
    assert!(!temp_dir.path().join("skillport-2026-06-01.log").exists());
    assert!(temp_dir.path().join("keep.txt").exists());
}

#[test]
fn cleanup_expired_runtime_logs_keeps_recent_dates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let old_date = (Local::now() - Duration::days(20)).format("%Y-%m-%d");
    let recent_date = (Local::now() - Duration::days(2)).format("%Y-%m-%d");
    let old_name = format!("skillport-{old_date}.log");
    let recent_name = format!("skillport-{recent_date}.log");
    fs::write(temp_dir.path().join(&old_name), "old").unwrap();
    fs::write(temp_dir.path().join(&recent_name), "recent").unwrap();

    let deleted = cleanup_expired_runtime_logs_in_dir(temp_dir.path(), 14).unwrap();

    assert_eq!(deleted, 1);
    assert!(!temp_dir.path().join(old_name).exists());
    assert!(temp_dir.path().join(recent_name).exists());
}

#[test]
fn frontend_runtime_payload_is_truncated_and_redacted() {
    let log = sanitize_frontend_runtime_log_payload(FrontendRuntimeLogPayload {
        level: Some("warning".to_string()),
        source: Some("x".repeat(120)),
        message: Some(format!("token=abc {}", "m".repeat(2_100))),
        details: Some(json!({
            "apiKey": "sk-test",
            "nested": { "password": "secret", "visible": "ok" }
        })),
    });

    assert_eq!(log.level, "warn");
    assert!(log.source.chars().count() <= MAX_FRONTEND_SOURCE_CHARS + 1);
    assert!(log.message.chars().count() <= MAX_FRONTEND_MESSAGE_CHARS + 1);
    assert!(log.message.contains("token=[REDACTED]"));
    assert!(log.details.contains("[REDACTED]"));
    assert!(log.details.contains("visible"));
    assert!(!log.details.contains("sk-test"));
    assert!(!log.details.contains("secret"));
}
