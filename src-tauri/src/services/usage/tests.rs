use super::*;
use crate::test_support::{mem_pool as setup_pool, seed_central_skill};
use tempfile::TempDir;

#[test]
fn skill_call_serializes_to_camel_case() {
    let call = SkillCall {
        skill: "review".to_string(),
        timestamp_ms: 1_700_000_000_000,
        project: "/tmp/x".to_string(),
        session_id: "s1".to_string(),
        source: "Claude Code".to_string(),
    };
    let json = serde_json::to_string(&call).unwrap();
    assert!(json.contains("\"timestampMs\""));
    assert!(json.contains("\"sessionId\""));
    assert!(!json.contains("\"timestamp_ms\""));
    assert!(!json.contains("\"session_id\""));
}

#[test]
fn scope_target_id_branches() {
    assert_eq!(Scope::Local.target_id(), "local");
}

#[test]
fn join_posix_path_preserves_remote_home_root() {
    assert_eq!(
        super::join_posix_path("/home/alice/", &[".codex", "sessions"]),
        "/home/alice/.codex/sessions"
    );
    assert_eq!(super::join_posix_path("/", &[".claude"]), "/.claude");
}

struct FailingProvider;
struct ReviewProvider;

#[async_trait::async_trait]
impl UsageProvider for FailingProvider {
    fn id(&self) -> &'static str {
        "failing"
    }

    fn display_name(&self) -> &'static str {
        "Failing Provider"
    }

    async fn available(&self, _scope: &Scope) -> bool {
        true
    }

    async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        Err(UsageError::Remote("fixture failure".to_string()))
    }
}

#[async_trait::async_trait]
impl UsageProvider for ReviewProvider {
    fn id(&self) -> &'static str {
        "review-provider"
    }

    fn display_name(&self) -> &'static str {
        "Review Provider"
    }

    async fn available(&self, _scope: &Scope) -> bool {
        true
    }

    async fn collect(&self, _scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        Ok(vec![SkillCall {
            skill: "review".to_string(),
            timestamp_ms: Utc::now().timestamp_millis(),
            project: "/project".to_string(),
            session_id: "session".to_string(),
            source: self.display_name().to_string(),
        }])
    }
}

fn write_claude_fixture(dir: &TempDir) {
    let history = r#"{"display":"/review","project":"/p1","sessionId":"s1","timestamp":1700000000000}
{"display":"/facts","project":"/p2","sessionId":"s2","timestamp":1700000010000}"#;
    std::fs::write(dir.path().join("history.jsonl"), history).unwrap();
}

// The guard intentionally serializes process-wide environment changes for the
// complete async test; no task spawned by this test acquires the same lock.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn refresh_scans_then_caches_within_ttl_then_force_rescans() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = setup_pool().await;
    let dir = TempDir::new().unwrap();
    write_claude_fixture(&dir);
    std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());

    let first = refresh(&pool, &Scope::Local, false).await.unwrap();
    assert!(!first.cached);
    assert!(first.calls_written >= 2);
    assert!(first.providers_available >= 1);
    let first_scan_ms = first.scanned_at_ms;

    let cached = refresh(&pool, &Scope::Local, false).await.unwrap();
    assert!(cached.cached);
    assert_eq!(cached.calls_written, 0);
    assert_eq!(cached.scanned_at_ms, first_scan_ms);

    let forced = refresh(&pool, &Scope::Local, true).await.unwrap();
    assert!(!forced.cached);
    assert!(forced.calls_written >= 2);

    let overview = build_overview(&pool, "local", None, 50).await.unwrap();
    assert!(overview.kpis.total_calls >= 2);
    assert_eq!(overview.heatmap.len(), 16 * 7);
    assert!(overview.last_scan_ms.is_some());
    let health = list_provider_health(&pool, "local").await.unwrap();
    assert_eq!(health.len(), 8);
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

#[tokio::test]
async fn refresh_marks_available_provider_unavailable_when_collect_fails() {
    let pool = setup_pool().await;
    let summary =
        refresh_with_providers(&pool, &Scope::Local, true, vec![Box::new(FailingProvider)])
            .await
            .unwrap();
    assert!(!summary.cached);
    assert_eq!(summary.calls_written, 0);
    assert_eq!(summary.providers_available, 0);
    let health = list_provider_health(&pool, "local").await.unwrap();
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].provider_id, "failing");
    assert!(!health[0].available);
}

#[tokio::test]
async fn refresh_enriches_unique_central_skill_with_static_metrics() {
    let pool = setup_pool().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("review");
    seed_central_skill(&pool, &skill_dir, "review", "Review code").await;
    refresh_with_providers(&pool, &Scope::Local, true, vec![Box::new(ReviewProvider)])
        .await
        .unwrap();
    let metadata = db::get_usage_metadata_for_skill(&pool, "local", "review")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(metadata.match_status, "matched");
    assert_eq!(metadata.resolved_skill_id.as_deref(), Some("review"));
    assert!(metadata.static_token_estimate.unwrap() > 0);
    assert!(metadata.static_byte_count.unwrap() > 0);
}

#[tokio::test]
async fn build_overview_and_recent_filter_by_source() {
    let pool = setup_pool().await;
    let now = Utc::now().timestamp_millis();
    let calls = vec![
        NewSkillCall {
            skill: "review".into(),
            timestamp_ms: now - 3_000,
            project: "/p1".into(),
            session_id: "s1".into(),
            source: "Claude Code".into(),
        },
        NewSkillCall {
            skill: "commit".into(),
            timestamp_ms: now - 2_000,
            project: "/p1".into(),
            session_id: "s2".into(),
            source: "Claude Code".into(),
        },
        NewSkillCall {
            skill: "review".into(),
            timestamp_ms: now - 1_000,
            project: "/p2".into(),
            session_id: "s3".into(),
            source: "Codex CLI".into(),
        },
    ];
    db::replace_calls_for_target(&pool, "local", &calls, &[], &[], now)
        .await
        .unwrap();

    let all = build_overview(&pool, "local", None, 50).await.unwrap();
    assert_eq!(all.kpis.total_calls, 3);
    assert_eq!(all.kpis.unique_sources, 2);
    assert_eq!(all.kpis.unique_sessions, 3);

    let claude = build_overview(&pool, "local", Some("Claude Code"), 50)
        .await
        .unwrap();
    assert_eq!(claude.kpis.total_calls, 2);
    assert_eq!(claude.kpis.unique_sources, 1);
    assert_eq!(claude.kpis.unique_sessions, 2);
    assert_eq!(claude.top_skills.len(), 2);
    assert_eq!(claude.heatmap.iter().map(|day| day.count).sum::<i64>(), 2);

    let codex = build_overview(&pool, "local", Some("Codex CLI"), 50)
        .await
        .unwrap();
    assert_eq!(codex.kpis.total_calls, 1);
    assert_eq!(codex.top_skills[0].skill, "review");
    assert_eq!(
        db::list_recent_calls(&pool, "local", Some("Claude Code"), 20)
            .await
            .unwrap()
            .len(),
        2
    );
}
