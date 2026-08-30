use super::*;
use crate::db::{self, DbPool, OperationLogFilter};

fn test_app_state(pool: DbPool) -> AppState {
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

fn refresh_result(
    calls_written: i64,
    providers_available: i64,
    scope: UsageScopeInfo,
    used_cached_data: bool,
    scanning: bool,
    refresh_error: Option<String>,
) -> UsageRefreshResult {
    UsageRefreshResult {
        summary: RefreshSummary {
            cached: used_cached_data,
            calls_written,
            providers_available,
            scanned_at_ms: 1_700_000_000_000,
        },
        overview: UsageOverview {
            kpis: Default::default(),
            top_skills: Vec::new(),
            heatmap: Vec::new(),
            last_scan_ms: Some(1_700_000_000_000),
        },
        recent: Vec::new(),
        providers: Vec::new(),
        scope,
        used_cached_data,
        scanning,
        refresh_error,
    }
}

#[test]
fn scope_info_for_remote_read_paths_is_optimistic() {
    let target = ActiveUsageTarget {
        active: ActiveTarget::Local,
        target_id: "ssh-prod".to_string(),
        label: "alice@prod".to_string(),
        is_remote: true,
    };

    let optimistic = scope_info_for_target(&target, None);
    assert!(optimistic.remote_reachable);

    let unreachable = scope_info_for_target(&target, Some(false));
    assert!(!unreachable.remote_reachable);
}

#[test]
fn cached_fallback_summary_preserves_last_successful_scan_time() {
    let summary = cached_fallback_summary(Some(1_700_000_000_000));
    assert!(summary.cached);
    assert_eq!(summary.scanned_at_ms, 1_700_000_000_000);

    let empty_summary = cached_fallback_summary(None);
    assert!(!empty_summary.cached);
    assert_eq!(empty_summary.scanned_at_ms, 0);
}

#[test]
fn background_rescan_only_for_stale_local_cache_without_force() {
    let now = 10_000_000_i64;
    let fresh = now - usage::CACHE_TTL_MS + 1;
    let stale_boundary = now - usage::CACHE_TTL_MS;

    assert!(!should_background_rescan(false, None, now));
    assert!(!should_background_rescan(false, Some(fresh), now));
    assert!(should_background_rescan(false, Some(stale_boundary), now));
    assert!(should_background_rescan(false, Some(1), now));
    assert!(!should_background_rescan(true, Some(1), now));
    assert!(!should_background_rescan(true, None, now));
}

#[test]
fn usage_refresh_result_serializes_scanning_flag() {
    let result = refresh_result(
        0,
        0,
        UsageScopeInfo {
            target_id: "local".to_string(),
            label: "Local".to_string(),
            is_remote: false,
            remote_reachable: false,
        },
        true,
        true,
        None,
    );
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["scanning"], serde_json::json!(true));
    assert_eq!(json["usedCachedData"], serde_json::json!(true));
}

#[tokio::test]
async fn usage_refresh_failure_keeps_one_row_and_boundary_correlation() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let definition = usage_refresh_definition();
    let raw_seed = "token=secret https://example.invalid C:\\Users\\private";
    let result = crate::observability::run_operation(
        &state,
        definition,
        OperationContext::new(OperationTarget::local()),
        |_| SafeOperationResult::succeeded("Usage refresh completed."),
        || async { Err::<(), _>(usage_refresh_failure(definition, raw_seed.to_string())) },
    )
    .await;
    let operation_error = result.unwrap_err();
    let operation_id = operation_error.correlation_id.clone().unwrap();
    let boundary_error = crate::ipc_error::complete_named_boundary(
        "usage_refresh",
        std::time::Instant::now(),
        Err::<(), _>(operation_error),
    )
    .unwrap_err();
    assert_eq!(
        boundary_error.correlation_id.as_deref(),
        Some(operation_id.as_str())
    );

    let page = db::list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    let entry = &page.entries[0];
    assert_eq!(entry.id, operation_id);
    assert_eq!(entry.action, "usage_refresh");
    assert_eq!(entry.category, "usage");
    assert_eq!(entry.status, "failed");
    assert!(!serde_json::to_string(entry).unwrap().contains(raw_seed));
}

#[tokio::test]
async fn usage_refresh_partial_details_are_safe_counts_and_static_modes() {
    let pool = crate::test_support::mem_pool().await;
    let state = test_app_state(pool.clone());
    let definition = usage_refresh_definition();
    let raw_seed = "token=secret https://example.invalid C:\\Users\\private";
    let result = refresh_result(
        4,
        2,
        UsageScopeInfo {
            target_id: raw_seed.to_string(),
            label: raw_seed.to_string(),
            is_remote: true,
            remote_reachable: false,
        },
        true,
        false,
        Some(raw_seed.to_string()),
    );

    crate::observability::run_operation(
        &state,
        definition,
        OperationContext::new(OperationTarget::local()),
        |result| usage_refresh_operation_result(result, true),
        || async { Ok::<_, ReviewedFailure>(result) },
    )
    .await
    .unwrap();

    let page = db::list_operation_logs(&pool, OperationLogFilter::default())
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    let entry = &page.entries[0];
    assert_eq!(entry.status, "partial");
    let details: serde_json::Value =
        serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
    assert_eq!(details["affectedCount"], 4);
    assert_eq!(details["succeededCount"], 2);
    assert_eq!(details["changed"], true);
    assert_eq!(details["mode"], "fallback");
    assert_eq!(details["scope"], "remote");
    assert!(!serde_json::to_string(entry).unwrap().contains(raw_seed));
}

#[test]
fn overlay_skill_usage_stats_prefills_zero_and_empty_list() {
    let empty = overlay_skill_usage_stats(&[], vec![]);
    assert!(empty.is_empty());

    let prefilled = overlay_skill_usage_stats(
        &["review".to_string(), "git-commit".to_string()],
        vec![crate::db::SkillUsageStatRow {
            skill: "review".to_string(),
            count: 4,
            last_used_ms: Some(9),
        }],
    );
    assert_eq!(prefilled["review"].count, 4);
    assert_eq!(prefilled["review"].last_used_ms, Some(9));
    assert_eq!(prefilled["git-commit"].count, 0);
    assert_eq!(prefilled["git-commit"].last_used_ms, None);
}

#[test]
fn skill_usage_stat_serializes_camel_case() {
    let json = serde_json::to_value(&SkillUsageStat {
        count: 3,
        last_used_ms: Some(42),
    })
    .unwrap();
    assert_eq!(json["count"], 3);
    assert_eq!(json["lastUsedMs"], 42);
    assert!(json.get("last_used_ms").is_none());
}
