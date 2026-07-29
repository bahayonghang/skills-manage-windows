use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::commands::agents::AgentWithStatus;
use crate::db::{self, DbPool};
use crate::operation_log::{
    local_target_context, record_operation_log_best_effort, OperationLogEvent,
};
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
    pub readiness: DashboardReadiness,
    pub source_repositories: Vec<db::SkillRepositoryWithStats>,
}

/// 仪表盘 readiness 评分，4 项加权后归一到 0..=100。
///
/// 计算依据见 `DashboardReadiness::from_counts`；权重为常量，本期暂不开放在
/// 设置中调整。空仓库（`total == 0`）所有 ratio 与 score 均为 0。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardReadiness {
    pub score: u32,
    pub categorized_ratio: f32,
    pub described_ratio: f32,
    pub sourced_ratio: f32,
    pub install_health_ratio: f32,
}

impl DashboardReadiness {
    pub const WEIGHT_CATEGORIZED: f32 = 0.35;
    pub const WEIGHT_DESCRIBED: f32 = 0.25;
    pub const WEIGHT_SOURCED: f32 = 0.20;
    pub const WEIGHT_INSTALL: f32 = 0.20;

    pub fn from_counts(counts: db::DashboardReadinessCounts) -> Self {
        if counts.total == 0 {
            return Self {
                score: 0,
                categorized_ratio: 0.0,
                described_ratio: 0.0,
                sourced_ratio: 0.0,
                install_health_ratio: 0.0,
            };
        }
        let total = counts.total as f32;
        let categorized_ratio = counts.categorized as f32 / total;
        let described_ratio = counts.described as f32 / total;
        let sourced_ratio = counts.sourced as f32 / total;
        let install_health_ratio = counts.installed as f32 / total;
        let raw = Self::WEIGHT_CATEGORIZED * categorized_ratio
            + Self::WEIGHT_DESCRIBED * described_ratio
            + Self::WEIGHT_SOURCED * sourced_ratio
            + Self::WEIGHT_INSTALL * install_health_ratio;
        let score = (raw * 100.0).round().clamp(0.0, 100.0) as u32;
        Self {
            score,
            categorized_ratio,
            described_ratio,
            sourced_ratio,
            install_health_ratio,
        }
    }
}

impl Default for DashboardReadiness {
    fn default() -> Self {
        Self {
            score: 0,
            categorized_ratio: 0.0,
            described_ratio: 0.0,
            sourced_ratio: 0.0,
            install_health_ratio: 0.0,
        }
    }
}

fn parse_scan_state(raw: Option<String>) -> ScanState {
    match raw.as_deref() {
        Some("refreshing") => ScanState::Refreshing,
        Some("error") => ScanState::Error,
        _ => ScanState::Idle,
    }
}

async fn recover_stale_scan_state_if_needed(pool: &DbPool) -> Result<(), String> {
    let raw_scan_state = db::get_setting(pool, "scan_state")
        .await
        .map_err(|e| e.to_string())?;
    if raw_scan_state.as_deref() != Some("refreshing") {
        return Ok(());
    }

    let Some(last_scan_at) = db::get_setting(pool, "scan_last_completed_at")
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let Ok(last_scan_at) = chrono::DateTime::parse_from_rfc3339(&last_scan_at) else {
        return Ok(());
    };
    let age = chrono::Utc::now().signed_duration_since(last_scan_at.with_timezone(&chrono::Utc));
    if age < chrono::Duration::minutes(10) {
        return Ok(());
    }

    db::set_setting(pool, "scan_state", "idle")
        .await
        .map_err(|e| e.to_string())?;
    record_operation_log_best_effort(
        pool,
        local_target_context(),
        OperationLogEvent::new(
            "scan",
            "scan.state_recovered",
            "succeeded",
            "Recovered stale scan_state from refreshing to idle",
        )
        .details(serde_json::json!({
            "previousState": "refreshing",
            "nextState": "idle",
            "lastScanCompletedAt": last_scan_at.to_rfc3339(),
        })),
    )
    .await;
    Ok(())
}

async fn load_scan_state(pool: &DbPool) -> Result<(Option<String>, ScanState), String> {
    recover_stale_scan_state_if_needed(pool).await?;
    let last_scan_at = db::get_setting(pool, "scan_last_completed_at")
        .await
        .map_err(|e| e.to_string())?;
    let scan_state = parse_scan_state(
        db::get_setting(pool, "scan_state")
            .await
            .map_err(|e| e.to_string())?,
    );
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
    let cached_skill_counts = db::get_skill_counts_by_agent(pool)
        .await
        .map_err(|e| e.to_string())?;
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
) -> crate::ipc_error::IpcResult<SkillCountsSummary> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            get_skill_counts_summary_impl(&pool).await
        }
        .await
    )
}

#[tauri::command]
pub async fn get_dashboard_central_summary(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<DashboardCentralSummary> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            get_dashboard_central_summary_impl(&pool).await
        }
        .await
    )
}

#[tauri::command]
pub async fn get_bootstrap_snapshot(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<BootstrapSnapshot> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            get_bootstrap_snapshot_impl(&pool).await
        }
        .await
    )
}

async fn get_bootstrap_snapshot_impl(pool: &DbPool) -> Result<BootstrapSnapshot, String> {
    let agents = db::get_all_agents(pool).await.map_err(|e| e.to_string())?;
    let skill_counts = get_skill_counts_summary_impl(pool).await?;
    let dashboard_central_summary = get_dashboard_central_summary_impl(pool).await?;
    let collection_count = db::get_collection_count(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(BootstrapSnapshot {
        agents: agents.into_iter().map(to_cached_agent).collect(),
        cached_skill_counts: skill_counts.cached_skill_counts,
        dashboard_central_summary,
        collection_count,
        last_scan_at: skill_counts.last_scan_at,
        scan_state: skill_counts.scan_state,
    })
}

async fn get_dashboard_central_summary_impl(
    pool: &DbPool,
) -> Result<DashboardCentralSummary, String> {
    let row = sqlx::query(
        "SELECT
           (SELECT COUNT(*) FROM skills WHERE is_central = 1) AS central_skill_count,
           (SELECT COUNT(*) FROM skill_update_states WHERE status = 'update_available') AS updates_available,
           (SELECT COUNT(DISTINCT skill_id) FROM skill_ai_tag_reviews WHERE status = 'pending') AS ai_review_count,
           (SELECT COUNT(*)
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
              )) AS uncategorized_count,
           (SELECT COUNT(*)
            FROM skills s
            LEFT JOIN skill_repository_members m ON s.id = m.skill_id
            LEFT JOIN skill_repositories r ON r.id = m.repository_id
            WHERE s.is_central = 1
              AND (m.skill_id IS NULL OR r.is_unknown = 1)) AS unassigned_source_count",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    let read_count = |column: &str| -> Result<usize, String> {
        let count: i64 = row.try_get(column).map_err(|e| e.to_string())?;
        Ok(count.max(0) as usize)
    };
    let central_skill_count = read_count("central_skill_count")?;
    let updates_available = read_count("updates_available")?;
    let ai_review_count = read_count("ai_review_count")?;
    let uncategorized_count = read_count("uncategorized_count")?;
    let unassigned_source_count = read_count("unassigned_source_count")?;
    let source_repositories = db::get_skill_repositories_with_stats(pool)
        .await
        .map_err(|e| e.to_string())?;
    let readiness_counts = db::count_central_readiness_inputs(pool)
        .await
        .map_err(|e| e.to_string())?;
    let readiness = DashboardReadiness::from_counts(readiness_counts);

    Ok(DashboardCentralSummary {
        central_skill_count,
        updates_available,
        ai_review_count,
        uncategorized_count,
        unassigned_source_count,
        readiness,
        source_repositories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, Skill, SkillInstallation};
    use crate::test_support::mem_pool as setup_test_db;

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
                uid: "frontend-design-uid".to_string(),
                name: "frontend-design".to_string(),
                description: Some("Build UI".to_string()),
                file_path: "/tmp/frontend-design/SKILL.md".to_string(),
                canonical_path: Some("/tmp/frontend-design".to_string()),
                is_central: true,
                source: Some("native".to_string()),
                content: None,
                scanned_at: "2026-04-23T01:00:00Z".to_string(),
                fs_created_at: None,
                fs_updated_at: None,
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
        assert_eq!(
            snapshot.last_scan_at.as_deref(),
            Some("2026-04-23T01:05:00Z")
        );
        assert_eq!(snapshot.scan_state, ScanState::Idle);
    }

    #[tokio::test]
    async fn test_load_scan_state_recovers_stale_refreshing_state() {
        let pool = setup_test_db().await;
        let stale_time = (chrono::Utc::now() - chrono::Duration::minutes(11)).to_rfc3339();
        db::set_setting(&pool, "scan_state", "refreshing")
            .await
            .unwrap();
        db::set_setting(&pool, "scan_last_completed_at", &stale_time)
            .await
            .unwrap();

        let (last_scan_at, scan_state) = load_scan_state(&pool).await.unwrap();

        assert_eq!(scan_state, ScanState::Idle);
        assert_eq!(last_scan_at.as_deref(), Some(stale_time.as_str()));
    }

    async fn insert_central_skill(pool: &DbPool, id: &str, description: Option<&str>) {
        db::upsert_skill(
            pool,
            &Skill {
                id: id.to_string(),
                uid: format!("{id}-uid"),
                name: id.to_string(),
                description: description.map(str::to_string),
                file_path: format!("/tmp/{id}/SKILL.md"),
                canonical_path: Some(format!("/tmp/{id}")),
                is_central: true,
                source: Some("native".to_string()),
                content: None,
                scanned_at: "2026-04-23T01:00:00Z".to_string(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_readiness_zero_when_no_central_skills() {
        let pool = setup_test_db().await;

        let summary = get_dashboard_central_summary_impl(&pool).await.unwrap();

        assert_eq!(summary.readiness.score, 0);
        assert_eq!(summary.readiness.categorized_ratio, 0.0);
        assert_eq!(summary.readiness.described_ratio, 0.0);
        assert_eq!(summary.readiness.sourced_ratio, 0.0);
        assert_eq!(summary.readiness.install_health_ratio, 0.0);
    }

    #[tokio::test]
    async fn test_readiness_full_score_when_all_inputs_satisfied() {
        let pool = setup_test_db().await;
        insert_central_skill(&pool, "frontend-design", Some("Build UI")).await;

        let tag = db::create_skill_tag(&pool, "ui", None, None).await.unwrap();
        db::assign_skill_tags(
            &pool,
            &["frontend-design".to_string()],
            std::slice::from_ref(&tag.id),
            "manual",
            None,
            None,
        )
        .await
        .unwrap();

        let repo = db::create_or_update_skill_repository(
            &pool,
            Some("acme/ui"),
            "acme/ui",
            "github",
            Some("acme"),
            Some("ui"),
            Some("main"),
            Some("https://github.com/acme/ui"),
            false,
        )
        .await
        .unwrap();
        db::assign_skills_to_repository(
            &pool,
            &repo.id,
            &["frontend-design".to_string()],
            Some("skills/frontend-design"),
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

        let summary = get_dashboard_central_summary_impl(&pool).await.unwrap();

        assert_eq!(summary.readiness.score, 100);
        assert!((summary.readiness.categorized_ratio - 1.0).abs() < f32::EPSILON);
        assert!((summary.readiness.described_ratio - 1.0).abs() < f32::EPSILON);
        assert!((summary.readiness.sourced_ratio - 1.0).abs() < f32::EPSILON);
        assert!((summary.readiness.install_health_ratio - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_readiness_partial_score_applies_weighted_formula() {
        let pool = setup_test_db().await;
        // 4 个 central skill：1 个全 4 项满足，1 个只有 description，1 个只有 install，1 个完全空。
        insert_central_skill(&pool, "full", Some("Has everything")).await;
        insert_central_skill(&pool, "described-only", Some("Just words")).await;
        insert_central_skill(&pool, "installed-only", None).await;
        insert_central_skill(&pool, "blank", None).await;

        let tag = db::create_skill_tag(&pool, "ui", None, None).await.unwrap();
        db::assign_skill_tags(
            &pool,
            &["full".to_string()],
            std::slice::from_ref(&tag.id),
            "manual",
            None,
            None,
        )
        .await
        .unwrap();

        let repo = db::create_or_update_skill_repository(
            &pool,
            Some("acme/ui"),
            "acme/ui",
            "github",
            Some("acme"),
            Some("ui"),
            Some("main"),
            Some("https://github.com/acme/ui"),
            false,
        )
        .await
        .unwrap();
        db::assign_skills_to_repository(
            &pool,
            &repo.id,
            &["full".to_string()],
            Some("skills/full"),
        )
        .await
        .unwrap();

        for skill_id in ["full", "installed-only"] {
            db::upsert_skill_installation(
                &pool,
                &SkillInstallation {
                    skill_id: skill_id.to_string(),
                    agent_id: "central".to_string(),
                    installed_path: format!("/tmp/{skill_id}"),
                    link_type: "native".to_string(),
                    symlink_target: None,
                    created_at: "2026-04-23T01:00:00Z".to_string(),
                },
            )
            .await
            .unwrap();
        }

        let summary = get_dashboard_central_summary_impl(&pool).await.unwrap();

        // 期望比率：分类 1/4、描述 2/4、有源 1/4、安装 2/4
        assert!((summary.readiness.categorized_ratio - 0.25).abs() < 1e-5);
        assert!((summary.readiness.described_ratio - 0.5).abs() < 1e-5);
        assert!((summary.readiness.sourced_ratio - 0.25).abs() < 1e-5);
        assert!((summary.readiness.install_health_ratio - 0.5).abs() < 1e-5);

        // 加权:0.35*0.25 + 0.25*0.5 + 0.20*0.25 + 0.20*0.5 = 0.3625 -> 36
        assert_eq!(summary.readiness.score, 36);
    }

    #[test]
    fn test_readiness_from_counts_handles_zero_total() {
        let readiness = DashboardReadiness::from_counts(db::DashboardReadinessCounts {
            total: 0,
            categorized: 9,
            described: 9,
            sourced: 9,
            installed: 9,
        });
        assert_eq!(readiness.score, 0);
        assert_eq!(readiness.categorized_ratio, 0.0);
    }
}
