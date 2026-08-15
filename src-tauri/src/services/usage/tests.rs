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

// ─── 增量扫描（skill_call_file_cache，migration 5）────────────────────────────

fn call_facts(rows: &[SkillCallRow]) -> Vec<(String, i64, String, String, String)> {
    rows.iter()
        .map(|r| {
            (
                r.skill.clone(),
                r.timestamp_ms,
                r.project.clone(),
                r.session_id.clone(),
                r.source.clone(),
            )
        })
        .collect()
}

// The guard intentionally serializes process-wide environment changes for the
// complete async test; no task spawned by this test acquires the same lock.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn refresh_incremental_rescan_reuses_file_cache_and_stays_equivalent() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = setup_pool().await;
    let dir = TempDir::new().unwrap();
    write_claude_fixture(&dir);
    std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());
    let claude_only: Vec<Box<dyn UsageProvider>> =
        vec![Box::new(providers::claude_code::ClaudeCodeProvider)];

    // 第一轮：全量扫描并建立文件缓存
    let first = refresh_with_providers(&pool, &Scope::Local, true, claude_only)
        .await
        .unwrap();
    assert_eq!(first.calls_written, 2);
    let cache_rows = db::list_file_cache_rows(&pool, "local", "claude-code")
        .await
        .unwrap();
    assert_eq!(cache_rows.len(), 1, "history.jsonl cached");
    let facts_before = call_facts(&db::list_calls_for_target(&pool, "local").await.unwrap());

    // 第二轮（force）：指纹未变 → 缓存命中，skill_calls 内容逐条相等；
    // 缓存行的 scanned_at_ms 不动 = 没有发生重新解析 + upsert
    let claude_only: Vec<Box<dyn UsageProvider>> =
        vec![Box::new(providers::claude_code::ClaudeCodeProvider)];
    refresh_with_providers(&pool, &Scope::Local, true, claude_only)
        .await
        .unwrap();
    let facts_after = call_facts(&db::list_calls_for_target(&pool, "local").await.unwrap());
    assert_eq!(facts_before, facts_after, "incremental rescan diverged");
    let cache_second = db::list_file_cache_rows(&pool, "local", "claude-code")
        .await
        .unwrap();
    assert_eq!(cache_second.len(), 1);
    assert_eq!(cache_second[0].scanned_at_ms, cache_rows[0].scanned_at_ms);

    // 改动 history.jsonl → 指纹失配 → 重解析，新事实进入 skill_calls
    let history = r#"{"display":"/review","project":"/p1","sessionId":"s1","timestamp":1700000000000}
{"display":"/facts","project":"/p2","sessionId":"s2","timestamp":1700000010000}
{"display":"/brand-new","project":"/p3","sessionId":"s3","timestamp":1700000020000}"#;
    std::fs::write(dir.path().join("history.jsonl"), history).unwrap();
    let claude_only: Vec<Box<dyn UsageProvider>> =
        vec![Box::new(providers::claude_code::ClaudeCodeProvider)];
    refresh_with_providers(&pool, &Scope::Local, true, claude_only)
        .await
        .unwrap();
    let facts_third = call_facts(&db::list_calls_for_target(&pool, "local").await.unwrap());
    assert_eq!(facts_third.len(), 3);
    assert!(facts_third.iter().any(|(skill, ..)| skill == "brand-new"));
    let cache_third = db::list_file_cache_rows(&pool, "local", "claude-code")
        .await
        .unwrap();
    assert_eq!(cache_third.len(), 1, "changed file upserted in place");

    // 删除 history.jsonl → provider 不可用 → 缓存行清空、事实清空
    std::fs::remove_file(dir.path().join("history.jsonl")).unwrap();
    let claude_only: Vec<Box<dyn UsageProvider>> =
        vec![Box::new(providers::claude_code::ClaudeCodeProvider)];
    refresh_with_providers(&pool, &Scope::Local, true, claude_only)
        .await
        .unwrap();
    assert!(
        db::list_calls_for_target(&pool, "local")
            .await
            .unwrap()
            .is_empty(),
        "unavailable provider facts are replaced with empty"
    );
    assert!(
        db::list_file_cache_rows(&pool, "local", "claude-code")
            .await
            .unwrap()
            .is_empty(),
        "unavailable provider cache rows are cleared"
    );

    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

// ─── build_unused_report ─────────────────────────────────────────────────────

fn unused_call(skill: &str, timestamp_ms: i64, source: &str) -> NewSkillCall {
    NewSkillCall {
        skill: skill.to_string(),
        timestamp_ms,
        project: "/project".to_string(),
        session_id: "session".to_string(),
        source: source.to_string(),
    }
}

fn unused_metadata(skill: &str, status: &str, resolved: Option<&str>) -> db::NewSkillUsageMetadata {
    db::NewSkillUsageMetadata {
        skill: skill.to_string(),
        match_status: status.to_string(),
        resolved_skill_id: resolved.map(str::to_string),
        static_token_estimate: Some(10),
        static_byte_count: Some(20),
    }
}

async fn seed_observation(pool: &DbPool, agent_id: &str, row_id: &str, name: &str) {
    db::upsert_agent_skill_observation(
        pool,
        &db::AgentSkillObservation {
            row_id: row_id.to_string(),
            agent_id: agent_id.to_string(),
            skill_id: row_id.to_string(),
            name: name.to_string(),
            description: None,
            file_path: format!("/agent/{agent_id}/{name}/SKILL.md"),
            dir_path: format!("/agent/{agent_id}/{name}"),
            source_kind: "global".to_string(),
            source_root: format!("/agent/{agent_id}"),
            link_type: "native".to_string(),
            symlink_target: None,
            is_read_only: false,
            scanned_at: "2024-01-01T00:00:00Z".to_string(),
            fs_created_at: None,
            fs_updated_at: None,
        },
    )
    .await
    .expect("seed observation");
}

#[tokio::test]
async fn build_unused_report_classifies_central_skills() {
    let pool = setup_pool().await;
    let temp = TempDir::new().unwrap();
    seed_central_skill(
        &pool,
        &temp.path().join("never-skill"),
        "never-skill",
        "Never",
    )
    .await;
    seed_central_skill(
        &pool,
        &temp.path().join("stale-skill"),
        "stale-skill",
        "Stale",
    )
    .await;
    seed_central_skill(
        &pool,
        &temp.path().join("fresh-skill"),
        "fresh-skill",
        "Fresh",
    )
    .await;
    sqlx::query(
        "INSERT INTO skill_installations (skill_id, agent_id, installed_path, link_type, created_at)
         VALUES ('never-skill', 'claude-code', '/agent/claude-code/never-skill', 'native', '2024-01-01')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let now = Utc::now().timestamp_millis();
    let old = now - 200 * 86_400_000;
    db::replace_calls_for_target(
        &pool,
        "local",
        &[
            unused_call("stale-skill", old, "Claude Code"),
            unused_call("fresh-skill", now - 1_000, "Claude Code"),
        ],
        &[],
        &[
            unused_metadata("stale-skill", "matched", Some("stale-skill")),
            unused_metadata("fresh-skill", "matched", Some("fresh-skill")),
        ],
        now,
    )
    .await
    .unwrap();

    let report = build_unused_report(&pool, &pool, "local", None, 90)
        .await
        .unwrap();
    let by_id: HashMap<String, &aggregate::UnusedSkillEntry> = report
        .central
        .iter()
        .map(|entry| (entry.skill_id.clone().unwrap(), entry))
        .collect();
    assert_eq!(by_id.len(), 2, "fresh-skill must be excluded: {by_id:?}");

    let never = by_id.get("never-skill").unwrap();
    assert_eq!(never.status, aggregate::UnusedSkillStatus::NeverUsed);
    assert_eq!(never.call_count, 0);
    assert_eq!(never.last_used_ms, None);
    assert_eq!(never.match_status, UsageSkillMatchStatus::Unmatched);
    assert_eq!(never.origin, aggregate::UnusedSkillOrigin::Central);
    assert_eq!(never.agents, vec!["claude-code".to_string()]);
    assert_eq!(never.static_token_estimate, None);

    let stale = by_id.get("stale-skill").unwrap();
    assert_eq!(stale.status, aggregate::UnusedSkillStatus::Stale);
    assert_eq!(stale.call_count, 1);
    assert_eq!(stale.last_used_ms, Some(old));
    assert_eq!(stale.match_status, UsageSkillMatchStatus::Matched);
    assert_eq!(stale.static_token_estimate, Some(10));
    assert_eq!(stale.static_byte_count, Some(20));
}

#[tokio::test]
async fn build_unused_report_covers_platform_observations_with_match_statuses() {
    let pool = setup_pool().await;
    seed_observation(&pool, "claude-code", "row-loose", "Loose Skill").await;
    seed_observation(&pool, "claude-code", "row-dup", "dup-skill").await;
    seed_observation(&pool, "codex", "row-dup-2", "dup-skill").await;
    seed_observation(&pool, "codex", "row-recent", "recent-skill").await;

    let now = Utc::now().timestamp_millis();
    let old = now - 120 * 86_400_000;
    db::replace_calls_for_target(
        &pool,
        "local",
        &[
            // 名称带大小写/空白变体，normalize 后仍归属 dup-skill
            unused_call(" Dup-Skill ", old, "Claude Code"),
            unused_call("recent-skill", now - 1_000, "Claude Code"),
        ],
        &[],
        &[
            unused_metadata("dup-skill", "ambiguous", None),
            unused_metadata("recent-skill", "unmatched", None),
        ],
        now,
    )
    .await
    .unwrap();

    let report = build_unused_report(&pool, &pool, "local", None, 90)
        .await
        .unwrap();
    let by_name: HashMap<String, &aggregate::UnusedSkillEntry> = report
        .platforms
        .iter()
        .map(|entry| (entry.name.clone(), entry))
        .collect();
    assert_eq!(
        by_name.len(),
        2,
        "recent-skill must be excluded: {by_name:?}"
    );

    let loose = by_name.get("Loose Skill").unwrap();
    assert_eq!(loose.status, aggregate::UnusedSkillStatus::NeverUsed);
    assert_eq!(loose.match_status, UsageSkillMatchStatus::Unmatched);
    assert_eq!(loose.skill_id, None);
    assert_eq!(loose.origin, aggregate::UnusedSkillOrigin::Platform);
    assert_eq!(loose.agents, vec!["claude-code".to_string()]);
    assert_eq!(
        loose.installed_path.as_deref(),
        Some("/agent/claude-code/Loose Skill")
    );

    let dup = by_name.get("dup-skill").unwrap();
    assert_eq!(dup.status, aggregate::UnusedSkillStatus::Stale);
    assert_eq!(dup.match_status, UsageSkillMatchStatus::Ambiguous);
    assert_eq!(dup.skill_id, None);
    assert_eq!(dup.call_count, 1);
    assert_eq!(dup.last_used_ms, Some(old));
    assert_eq!(
        dup.agents,
        vec!["claude-code".to_string(), "codex".to_string()]
    );
}

#[tokio::test]
async fn build_unused_report_applies_source_filter_to_call_aggregation() {
    let pool = setup_pool().await;
    let temp = TempDir::new().unwrap();
    seed_central_skill(
        &pool,
        &temp.path().join("codex-central"),
        "codex-central",
        "Codex only",
    )
    .await;

    let now = Utc::now().timestamp_millis();
    db::replace_calls_for_target(
        &pool,
        "local",
        &[unused_call("codex-central", now - 1_000, "Codex CLI")],
        &[],
        &[unused_metadata(
            "codex-central",
            "matched",
            Some("codex-central"),
        )],
        now,
    )
    .await
    .unwrap();

    let unfiltered = build_unused_report(&pool, &pool, "local", None, 90)
        .await
        .unwrap();
    assert!(
        unfiltered.central.is_empty(),
        "recently used skill is excluded without source filter"
    );

    // source 过滤只作用于 calls 聚合：Claude Code 视角下该 skill 零调用
    let claude = build_unused_report(&pool, &pool, "local", Some("Claude Code"), 90)
        .await
        .unwrap();
    assert_eq!(claude.central.len(), 1);
    let entry = &claude.central[0];
    assert_eq!(entry.skill_id.as_deref(), Some("codex-central"));
    assert_eq!(entry.status, aggregate::UnusedSkillStatus::NeverUsed);
    assert_eq!(entry.call_count, 0);
    assert_eq!(entry.last_used_ms, None);
    // metadata 不按 source 过滤：身份与静态体量仍可用
    assert_eq!(entry.match_status, UsageSkillMatchStatus::Matched);
    assert_eq!(entry.static_token_estimate, Some(10));
}

#[tokio::test]
async fn build_unused_report_scopes_usage_facts_to_target() {
    let pool = setup_pool().await;
    let temp = TempDir::new().unwrap();
    seed_central_skill(
        &pool,
        &temp.path().join("remote-used"),
        "remote-used",
        "Used on remote only",
    )
    .await;
    seed_observation(&pool, "claude-code", "row-plat", "plat-skill").await;

    let now = Utc::now().timestamp_millis();
    db::replace_calls_for_target(
        &pool,
        "ssh-prod",
        &[
            unused_call("remote-used", now - 1_000, "Claude Code"),
            unused_call("plat-skill", now - 1_000, "Claude Code"),
        ],
        &[],
        &[
            unused_metadata("remote-used", "matched", Some("remote-used")),
            unused_metadata("plat-skill", "unmatched", None),
        ],
        now,
    )
    .await
    .unwrap();

    // local target 看不到 ssh-prod 的调用：两个维度都按 never_used 出现
    let local = build_unused_report(&pool, &pool, "local", None, 90)
        .await
        .unwrap();
    assert_eq!(local.central.len(), 1);
    assert_eq!(local.central[0].skill_id.as_deref(), Some("remote-used"));
    assert_eq!(
        local.central[0].status,
        aggregate::UnusedSkillStatus::NeverUsed
    );
    assert_eq!(local.platforms.len(), 1);
    assert_eq!(local.platforms[0].name, "plat-skill");
    assert_eq!(
        local.platforms[0].status,
        aggregate::UnusedSkillStatus::NeverUsed
    );

    // ssh-prod target 下两者都最近使用过：不进未使用清单
    let remote = build_unused_report(&pool, &pool, "ssh-prod", None, 90)
        .await
        .unwrap();
    assert!(remote.central.is_empty());
    assert!(remote.platforms.is_empty());
}
