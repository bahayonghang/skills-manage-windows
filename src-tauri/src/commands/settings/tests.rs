use super::*;
use crate::db;
use crate::test_support::mem_pool as setup_test_db;

// ── get_scan_directories_impl ─────────────────────────────────────────────

/// Counts unique global_skills_dir paths across all built-in agents — the
/// same number that seed_builtin_scan_directories inserts.
fn expected_builtin_count() -> usize {
    let mut paths = std::collections::HashSet::new();
    for agent in db::builtin_agents() {
        paths.insert(agent.global_skills_dir);
    }
    paths.len()
}

#[tokio::test]
async fn test_get_scan_directories_has_builtin_dirs_initially() {
    let pool = setup_test_db().await;
    let dirs = get_scan_directories_impl(&pool).await.unwrap();
    let builtin_count = expected_builtin_count();
    // After init, built-in scan directories are seeded automatically.
    assert_eq!(
        dirs.len(),
        builtin_count,
        "Fresh database should have {} built-in scan directories, got {}",
        builtin_count,
        dirs.len()
    );
    // All seeded rows must be marked built-in.
    for dir in &dirs {
        assert!(
            dir.is_builtin,
            "Scan directory '{}' seeded during init must have is_builtin=true",
            dir.path
        );
    }
}

#[tokio::test]
async fn test_get_scan_directories_returns_added() {
    let pool = setup_test_db().await;
    add_scan_directory_impl(&pool, "/tmp/proj-a", Some("Project A"))
        .await
        .unwrap();
    add_scan_directory_impl(&pool, "/tmp/proj-b", None)
        .await
        .unwrap();

    let dirs = get_scan_directories_impl(&pool).await.unwrap();
    // N built-in dirs are already there; we added 2 custom ones.
    let builtin_count = expected_builtin_count();
    assert_eq!(dirs.len(), builtin_count + 2);
    let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
    assert!(paths.contains(&"/tmp/proj-a"));
    assert!(paths.contains(&"/tmp/proj-b"));
}

// ── add_scan_directory_impl ───────────────────────────────────────────────

#[tokio::test]
async fn test_add_scan_directory_creates_non_builtin() {
    let pool = setup_test_db().await;
    let dir = add_scan_directory_impl(&pool, "/tmp/my-project", Some("My Project"))
        .await
        .unwrap();

    assert_eq!(dir.path, "/tmp/my-project");
    assert_eq!(dir.label.as_deref(), Some("My Project"));
    assert!(dir.is_active);
    assert!(
        !dir.is_builtin,
        "Newly added directory should not be built-in"
    );
}

#[tokio::test]
async fn test_add_scan_directory_without_label() {
    let pool = setup_test_db().await;
    let dir = add_scan_directory_impl(&pool, "/tmp/no-label", None)
        .await
        .unwrap();
    assert!(dir.label.is_none());
}

#[tokio::test]
async fn test_add_scan_directory_expands_tilde() {
    let pool = setup_test_db().await;
    let dir = add_scan_directory_impl(&pool, "~/.skillsmanage/custom-scan", None)
        .await
        .unwrap();
    assert!(
        !dir.path.starts_with('~'),
        "tilde paths must be expanded before persistence"
    );
    assert!(dir.path.contains(".skillsmanage"));
}

#[tokio::test]
async fn test_add_scan_directory_expands_tilde_with_remote_home() {
    let pool = setup_test_db().await;
    let dir = add_scan_directory_impl_for_home(
        &pool,
        "~/.skillsmanage/remote-scan",
        None,
        Some("/home/alice"),
    )
    .await
    .unwrap();

    assert_eq!(dir.path, "/home/alice/.skillsmanage/remote-scan");
}

#[tokio::test]
async fn test_add_scan_directory_empty_path_fails() {
    let pool = setup_test_db().await;
    let result = add_scan_directory_impl(&pool, "   ", None).await;
    assert!(result.is_err(), "Empty path should fail validation");
}

#[tokio::test]
async fn test_add_scan_directory_duplicate_path_fails() {
    let pool = setup_test_db().await;
    add_scan_directory_impl(&pool, "/tmp/same-path", None)
        .await
        .unwrap();
    let result = add_scan_directory_impl(&pool, "/tmp/same-path", None).await;
    assert!(
        result.is_err(),
        "Duplicate path should fail (UNIQUE constraint)"
    );
}

// ── remove_scan_directory_impl ────────────────────────────────────────────

#[tokio::test]
async fn test_remove_scan_directory_success() {
    let pool = setup_test_db().await;
    add_scan_directory_impl(&pool, "/tmp/removable", None)
        .await
        .unwrap();

    remove_scan_directory_impl(&pool, "/tmp/removable")
        .await
        .unwrap();

    let dirs = get_scan_directories_impl(&pool).await.unwrap();
    // Built-in dirs remain; only the custom /tmp/removable should be gone.
    let builtin_count = expected_builtin_count();
    assert_eq!(
        dirs.len(),
        builtin_count,
        "Only the custom directory should be removed"
    );
    assert!(
        !dirs.iter().any(|d| d.path == "/tmp/removable"),
        "Removed directory must not appear in the list"
    );
}

#[tokio::test]
async fn test_remove_nonexistent_scan_directory_fails() {
    let pool = setup_test_db().await;
    let result = remove_scan_directory_impl(&pool, "/nonexistent/path").await;
    assert!(
        result.is_err(),
        "Removing a nonexistent directory should fail"
    );
}

#[tokio::test]
async fn test_remove_builtin_scan_directory_fails() {
    let pool = setup_test_db().await;
    // Manually insert a builtin directory
    sqlx::query(
        "INSERT INTO scan_directories (path, is_active, is_builtin, added_at)
             VALUES ('/builtin/path', 1, 1, datetime('now'))",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = remove_scan_directory_impl(&pool, "/builtin/path").await;
    assert!(result.is_err(), "Removing a built-in directory should fail");
}

// ── set_scan_directory_active_impl ────────────────────────────────────────

#[tokio::test]
async fn test_set_scan_directory_active_disables() {
    let pool = setup_test_db().await;
    add_scan_directory_impl(&pool, "/tmp/toggle-me", None)
        .await
        .unwrap();
    set_scan_directory_active_impl(&pool, "/tmp/toggle-me", false)
        .await
        .unwrap();
    let dirs = get_scan_directories_impl(&pool).await.unwrap();
    let dir = dirs.iter().find(|d| d.path == "/tmp/toggle-me").unwrap();
    assert!(!dir.is_active, "Directory should be inactive");
}

#[tokio::test]
async fn test_set_scan_directory_active_enables() {
    let pool = setup_test_db().await;
    add_scan_directory_impl(&pool, "/tmp/re-enable-me", None)
        .await
        .unwrap();
    // First disable
    set_scan_directory_active_impl(&pool, "/tmp/re-enable-me", false)
        .await
        .unwrap();
    // Then re-enable
    set_scan_directory_active_impl(&pool, "/tmp/re-enable-me", true)
        .await
        .unwrap();
    let dirs = get_scan_directories_impl(&pool).await.unwrap();
    let dir = dirs.iter().find(|d| d.path == "/tmp/re-enable-me").unwrap();
    assert!(dir.is_active, "Directory should be active again");
}

// ── get_setting_impl ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_setting_not_set_returns_none() {
    let pool = setup_test_db().await;
    let value = get_setting_impl(&pool, "unset_key").await.unwrap();
    assert!(value.is_none(), "Unset key should return None");
}

#[tokio::test]
async fn test_protected_secret_settings_are_blocked() {
    let pool = setup_test_db().await;

    assert!(get_setting_impl(&pool, "github_pat").await.is_err());
    assert!(get_setting_impl(&pool, "ai_api_key").await.is_err());
    assert!(get_setting_impl(&pool, "ai_api_key__deepseek")
        .await
        .is_err());
}

#[tokio::test]
async fn test_batch_settings_block_protected_secret_keys() {
    let pool = setup_test_db().await;
    let mut values = HashMap::new();
    values.insert(
        "central_update_check_mode_v1".to_string(),
        "sync".to_string(),
    );
    values.insert("ai_api_key__deepseek".to_string(), "token".to_string());

    assert!(set_settings_impl(&pool, &values).await.is_err());
    assert!(
        get_settings_impl(&pool, &values.keys().cloned().collect::<Vec<_>>())
            .await
            .is_err()
    );
    assert_eq!(
        db::get_setting(&pool, "ai_api_key__deepseek")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        db::get_setting(&pool, "central_update_check_mode_v1")
            .await
            .unwrap(),
        None,
        "batch validation must finish before the transaction begins"
    );
}

#[tokio::test]
async fn test_set_setting_empty_value_is_allowed() {
    let pool = setup_test_db().await;
    let result = set_setting_impl(&pool, "ai_model__custom", "").await;
    assert!(result.is_ok(), "Setting an empty value should succeed");
    let value = get_setting_impl(&pool, "ai_model__custom").await.unwrap();
    assert_eq!(value.as_deref(), Some(""));
}

#[tokio::test]
async fn test_generic_setters_reject_internal_and_unknown_keys_with_stable_errors() {
    let pool = setup_test_db().await;
    for key in [
        "ssh_targets_v1",
        "wsl_targets_v1",
        "active_target_id_v1",
        "target_config_quarantine_v1",
        "migration_complete_v1",
        "feature_gate_v1",
        "unknown_key",
    ] {
        let error = set_setting_impl(&pool, key, "do-not-echo")
            .await
            .unwrap_err();
        assert!(error.starts_with("setting_key_forbidden:"));
        assert!(!error.contains(key));
        assert!(!error.contains("do-not-echo"));
        assert_eq!(db::get_setting(&pool, key).await.unwrap(), None);
    }
}

#[tokio::test]
async fn test_batch_invalid_value_writes_nothing() {
    let pool = setup_test_db().await;
    let values = HashMap::from([
        ("ai_tag_concurrency".to_string(), "2".to_string()),
        (
            "ai_tag_interval_ms".to_string(),
            "private-value".to_string(),
        ),
    ]);

    let error = set_settings_impl(&pool, &values).await.unwrap_err();
    assert_eq!(
        error,
        "setting_value_invalid: The setting value is invalid."
    );
    assert!(!error.contains("private-value"));
    assert_eq!(
        db::get_setting(&pool, "ai_tag_concurrency").await.unwrap(),
        None
    );
}

#[test]
fn test_setting_audit_details_never_include_caller_keys_or_values() {
    let raw_unknown_key = "password=caller-secret";
    let details = setting_audit_details(["ai_model__custom", raw_unknown_key].into_iter(), false);
    let serialized = serde_json::to_string(&details).unwrap();

    assert_eq!(details["categories"], json!(["ai"]));
    assert_eq!(details["keyCount"], 2);
    assert_eq!(details["valueStored"], false);
    assert!(!serialized.contains("ai_model__custom"));
    assert!(!serialized.contains(raw_unknown_key));
    assert!(!serialized.contains("caller-secret"));
}

#[tokio::test]
async fn skills_cli_recent_sources_roundtrip_and_zero_write() {
    let pool = setup_test_db().await;
    let value = r#"["owner/repo"]"#;
    set_setting_impl(&pool, "skills_cli.recent_sources", value)
        .await
        .unwrap();
    assert_eq!(
        get_setting_impl(&pool, "skills_cli.recent_sources")
            .await
            .unwrap()
            .as_deref(),
        Some(value)
    );

    let secret = r#"["https://user:token@github.com/owner/repo"]"#;
    let error = set_setting_impl(&pool, "skills_cli.recent_sources", secret)
        .await
        .unwrap_err();
    assert!(error.starts_with("setting_value_invalid:"));
    assert!(!error.contains("token"));
    assert_eq!(
        get_setting_impl(&pool, "skills_cli.recent_sources")
            .await
            .unwrap()
            .as_deref(),
        Some(value)
    );

    let values = HashMap::from([
        ("font_scale_v1".to_string(), "1".to_string()),
        (
            "skills_cli.recent_sources".to_string(),
            r#"["owner/repo","owner/repo"]"#.to_string(),
        ),
    ]);
    let batch_error = set_settings_impl(&pool, &values).await.unwrap_err();
    assert_eq!(
        batch_error,
        "setting_value_invalid: The setting value is invalid."
    );
    assert_eq!(db::get_setting(&pool, "font_scale_v1").await.unwrap(), None);
    assert_eq!(
        get_setting_impl(&pool, "skills_cli.recent_sources")
            .await
            .unwrap()
            .as_deref(),
        Some(value)
    );
}
