use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::commands::agents::AgentWithStatus;
use crate::db::{self, DbPool};
use crate::AppState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanState {
    Idle,
    Refreshing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCountsSummary {
    pub cached_skill_counts: std::collections::HashMap<String, usize>,
    pub last_scan_at: Option<String>,
    pub scan_state: ScanState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSnapshot {
    pub agents: Vec<AgentWithStatus>,
    pub cached_skill_counts: std::collections::HashMap<String, usize>,
    pub dashboard_central_summary: DashboardCentralSummary,
    pub collection_count: usize,
    pub discovered_count: usize,
    pub last_scan_at: Option<String>,
    pub scan_state: ScanState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCentralSummary {
    pub central_skill_count: usize,
    pub updates_available: usize,
    pub ai_review_count: usize,
    pub uncategorized_count: usize,
    pub unassigned_source_count: usize,
    pub source_repositories: Vec<db::SkillRepositoryWithStats>,
}

fn parse_scan_state(raw: Option<String>) -> ScanState {
    match raw.as_deref() {
        Some("refreshing") => ScanState::Refreshing,
        Some("error") => ScanState::Error,
        _ => ScanState::Idle,
    }
}

async fn load_scan_state(pool: &DbPool) -> Result<(Option<String>, ScanState), String> {
    let last_scan_at = db::get_setting(pool, "scan_last_completed_at").await?;
    let scan_state = parse_scan_state(db::get_setting(pool, "scan_state").await?);
    Ok((last_scan_at, scan_state))
}

fn to_cached_agent(agent: db::Agent) -> AgentWithStatus {
    AgentWithStatus {
        id: agent.id,
        display_name: agent.display_name,
        category: agent.category,
        global_skills_dir: agent.global_skills_dir,
        project_skills_dir: agent.project_skills_dir,
        icon_name: agent.icon_name,
        is_detected: agent.is_detected,
        is_builtin: agent.is_builtin,
        is_enabled: agent.is_enabled,
    }
}

async fn get_skill_counts_summary_impl(pool: &DbPool) -> Result<SkillCountsSummary, String> {
    let cached_skill_counts = db::get_skill_counts_by_agent(pool).await?;
    let (last_scan_at, scan_state) = load_scan_state(pool).await?;

    Ok(SkillCountsSummary {
        cached_skill_counts,
        last_scan_at,
        scan_state,
    })
}

#[tauri::command]
pub async fn get_skill_counts_summary(
    state: State<'_, AppState>,
) -> Result<SkillCountsSummary, String> {
    let pool = state.active_db().await?;
    get_skill_counts_summary_impl(&pool).await
}

#[tauri::command]
pub async fn get_bootstrap_snapshot(
    state: State<'_, AppState>,
) -> Result<BootstrapSnapshot, String> {
    let pool = state.active_db().await?;
    get_bootstrap_snapshot_impl(&pool).await
}

async fn get_bootstrap_snapshot_impl(pool: &DbPool) -> Result<BootstrapSnapshot, String> {
    let agents = db::get_all_agents(pool).await?;
    let skill_counts = get_skill_counts_summary_impl(pool).await?;
    let dashboard_central_summary = get_dashboard_central_summary_impl(pool).await?;
    let collection_count = db::get_collection_count(pool).await?;
    let discovered_count = db::get_discovered_skill_count(pool).await?;

    Ok(BootstrapSnapshot {
        agents: agents.into_iter().map(to_cached_agent).collect(),
        cached_skill_counts: skill_counts.cached_skill_counts,
        dashboard_central_summary,
        collection_count,
        discovered_count,
        last_scan_at: skill_counts.last_scan_at,
        scan_state: skill_counts.scan_state,
    })
}

async fn get_count(pool: &DbPool, sql: &str) -> Result<usize, String> {
    let row = sqlx::query(sql)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
    Ok(count.max(0) as usize)
}

async fn get_dashboard_central_summary_impl(
    pool: &DbPool,
) -> Result<DashboardCentralSummary, String> {
    let central_skill_count = get_count(
        pool,
        "SELECT COUNT(*) AS cnt FROM skills WHERE is_central = 1",
    )
    .await?;
    let updates_available = get_count(
        pool,
        "SELECT COUNT(*) AS cnt
         FROM skill_update_states
         WHERE status = 'update_available'",
    )
    .await?;
    let ai_review_count = get_count(
        pool,
        "SELECT COUNT(DISTINCT skill_id) AS cnt
         FROM skill_ai_tag_reviews
         WHERE status = 'pending'",
    )
    .await?;
    let uncategorized_count = get_count(
        pool,
        "SELECT COUNT(*) AS cnt
         FROM skills s
         WHERE s.is_central = 1
           AND (
             NOT EXISTS (
               SELECT 1 FROM skill_tag_links l WHERE l.skill_id = s.id
             )
             OR EXISTS (
               SELECT 1 FROM skill_tag_links l
               WHERE l.skill_id = s.id AND l.tag_id = 'uncategorized'
             )
           )",
    )
    .await?;
    let unassigned_source_count = get_count(
        pool,
        "SELECT COUNT(*) AS cnt
         FROM skills s
         LEFT JOIN skill_repository_members m ON s.id = m.skill_id
         LEFT JOIN skill_repositories r ON r.id = m.repository_id
         WHERE s.is_central = 1
           AND (m.skill_id IS NULL OR r.is_unknown = 1)",
    )
    .await?;
    let source_repositories = db::get_skill_repositories_with_stats(pool).await?;

    Ok(DashboardCentralSummary {
        central_skill_count,
        updates_available,
        ai_review_count,
        uncategorized_count,
        unassigned_source_count,
        source_repositories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, Skill, SkillInstallation};
    use sqlx::SqlitePool;

    async fn setup_test_db() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_get_skill_counts_summary_defaults_to_idle_without_settings() {
        let pool = setup_test_db().await;

        let summary = get_skill_counts_summary_impl(&pool).await.unwrap();
        assert!(summary.cached_skill_counts.is_empty());
        assert_eq!(summary.scan_state, ScanState::Idle);
        assert!(summary.last_scan_at.is_none());
    }

    #[tokio::test]
    async fn test_bootstrap_snapshot_returns_cached_counts_and_totals() {
        let pool = setup_test_db().await;

        db::upsert_skill(
            &pool,
            &Skill {
                id: "frontend-design".to_string(),
                name: "frontend-design".to_string(),
                description: Some("Build UI".to_string()),
                file_path: "/tmp/frontend-design/SKILL.md".to_string(),
                canonical_path: Some("/tmp/frontend-design".to_string()),
                is_central: true,
                source: Some("native".to_string()),
                content: None,
                scanned_at: "2026-04-23T01:00:00Z".to_string(),
            },
        )
        .await
        .unwrap();
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "frontend-design".to_string(),
                agent_id: "central".to_string(),
                installed_path: "/tmp/frontend-design".to_string(),
                link_type: "native".to_string(),
                symlink_target: None,
                created_at: "2026-04-23T01:00:00Z".to_string(),
            },
        )
        .await
        .unwrap();
        db::create_collection(&pool, "Frontend", Some("UI skills"))
            .await
            .unwrap();
        db::insert_discovered_skill(
            &pool,
            "claude-code__project__deploy",
            "deploy",
            Some("Deploy the app"),
            "/tmp/project/.claude/skills/deploy/SKILL.md",
            "/tmp/project/.claude/skills/deploy",
            "/tmp/project",
            "project",
            "claude-code",
            "2026-04-23T01:00:00Z",
        )
        .await
        .unwrap();
        db::set_setting(&pool, "scan_state", "idle").await.unwrap();
        db::set_setting(&pool, "scan_last_completed_at", "2026-04-23T01:05:00Z")
            .await
            .unwrap();

        let snapshot = get_bootstrap_snapshot_impl(&pool).await.unwrap();

        assert_eq!(
            snapshot.cached_skill_counts.get("central").copied(),
            Some(1)
        );
        assert_eq!(snapshot.collection_count, 1);
        assert_eq!(snapshot.discovered_count, 1);
        assert_eq!(
            snapshot.last_scan_at.as_deref(),
            Some("2026-04-23T01:05:00Z")
        );
        assert_eq!(snapshot.scan_state, ScanState::Idle);
    }
}
