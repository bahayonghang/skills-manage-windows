//! Tauri command shell for `scan_all_skills`. Business logic lives in
//! `crate::services::scanner`; this file translates IPC arguments + state into
//! pool/target inputs and records operation logs.

use std::time::Instant;

use chrono::Utc;
use serde_json::json;
use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::db;
use crate::services::scanner::{scan_all_skills_impl, scan_ssh_skills_impl};
use crate::targets::ActiveTarget;
use crate::AppState;

// Re-export public types + helpers used by other modules (commands::discover).
// Keeps `super::scanner::parse_skill_md` / `super::scanner::scan_directory`
// call sites in commands/discover.rs working without modification.
pub use crate::services::scanner::{
    detect_link_type, parse_skill_md, parse_skill_md_content, scan_directory, ScanResult,
    ScannedSkill, SkillInfo,
};

/// Tauri command: scan all agent skill directories and persist the results to
/// SQLite. Returns a `ScanResult` with per-agent skill counts.
#[tauri::command]
pub async fn scan_all_skills(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let _ = db::set_setting(&pool, "scan_state", "refreshing").await;
    let started_at = Instant::now();

    let scan_result = match active_target {
        ActiveTarget::Local => scan_all_skills_impl(&pool).await,
        ActiveTarget::Ssh(target) => scan_ssh_skills_impl(&pool, &target).await,
    };

    match scan_result {
        Ok(result) => {
            let completed_at = Utc::now().to_rfc3339();
            let _ = db::set_setting(&pool, "scan_last_completed_at", &completed_at).await;
            let _ = db::set_setting(&pool, "scan_state", "idle").await;
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "scan",
                    "scan.all",
                    "succeeded",
                    format!(
                        "Scanned {} skills across {} agents",
                        result.total_skills, result.agents_scanned
                    ),
                )
                .subject("scan_root", "all", "All scan directories")
                .details(json!({
                    "totalSkills": result.total_skills,
                    "agentsScanned": result.agents_scanned,
                    "skillsByAgent": result.skills_by_agent,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
            Ok(result)
        }
        Err(error) => {
            let _ = db::set_setting(&pool, "scan_state", "error").await;
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new("scan", "scan.all", "failed", "Failed to scan skills")
                    .subject("scan_root", "all", "All scan directories")
                    .error(&error)
                    .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
            Err(error)
        }
    }
}
