//! Discover scan orchestration — DB reconciliation, Tauri events,
//! and the global cancel flag.
//!
//! Filesystem traversal lives in [`super::scan_core`]. This module wires the
//! core into the IPC flow: it owns the at-most-one cancel flag, runs each
//! enabled scan root in a blocking pool, emits progress / found / complete
//! events, persists discovered skills, and reconciles the cache.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use tauri::Emitter;

use crate::db::{self, DbPool};
use crate::paths;

use super::roots::platform_skill_patterns;
use super::scan_core;
use super::types::{
    CompletePayload, DiscoverResult, DiscoveredProject, FoundPayload, ProgressPayload, ScanRoot,
};

/// Process-wide cancel flag for the at-most-one Discover scan job. Set by
/// [`stop_project_scan_impl`]; consumed by [`scan_root_for_projects_blocking`]
/// when it hands work to the scan core.
static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) fn set_scan_cancel_for_test(cancelled: bool) {
    SCAN_CANCEL.store(cancelled, Ordering::Relaxed);
}

#[cfg(test)]
pub(super) fn is_scan_cancelled() -> bool {
    SCAN_CANCEL.load(Ordering::Relaxed)
}

/// Cancel an in-progress project scan.
pub fn stop_project_scan_impl() -> Result<(), String> {
    SCAN_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

async fn scan_root_for_projects_blocking(
    root: PathBuf,
    patterns: Vec<(String, String, PathBuf)>,
    central_dir: PathBuf,
) -> Result<Vec<DiscoveredProject>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scan_core::scan_root_for_projects(&root, &patterns, &central_dir, &SCAN_CANCEL)
    })
    .await
    .map_err(|e| format!("Failed to join Discover scan task: {}", e))
}

/// Reconcile the `discovered_skills` table after a scan.
///
/// Removes rows whose project lies under a scanned root and whose underlying
/// skill directory no longer exists on disk. Rows outside the scanned scope
/// are left alone so partial scans never delete unrelated cache entries.
pub async fn reconcile_discovered_skills(
    pool: &DbPool,
    scan_roots: &[&ScanRoot],
    found_skill_ids: &[String],
) -> Result<(), String> {
    let all_rows = db::get_all_discovered_skills(pool).await?;

    let found_set: HashSet<&str> = found_skill_ids.iter().map(String::as_str).collect();

    for row in &all_rows {
        if found_set.contains(row.id.as_str()) {
            continue;
        }

        let project_path = Path::new(&row.project_path);
        let under_scanned_root = scan_roots.iter().any(|root| {
            project_path.starts_with(&root.path)
                || project_path.as_os_str() == OsStr::new(&root.path)
        });

        if !under_scanned_root {
            continue;
        }

        if !Path::new(&row.dir_path).exists() {
            db::delete_discovered_skill(pool, &row.id).await?;
        }
    }

    Ok(())
}

/// Start a project-discovery scan across the given root directories.
pub async fn start_project_scan_impl(
    pool: &DbPool,
    app: &tauri::AppHandle,
    roots: Vec<ScanRoot>,
) -> Result<DiscoverResult, String> {
    SCAN_CANCEL.store(false, Ordering::Relaxed);

    let patterns = platform_skill_patterns(pool);
    let central_dir = paths::central_skills_dir();
    let enabled_roots: Vec<&ScanRoot> = roots.iter().filter(|r| r.enabled && r.exists).collect();
    let total_roots = enabled_roots.len();

    let mut all_projects: Vec<DiscoveredProject> = Vec::new();
    let mut total_skills = 0;
    let mut roots_scanned = 0;

    for root in &enabled_roots {
        if SCAN_CANCEL.load(Ordering::Relaxed) {
            break;
        }

        let root_path = PathBuf::from(&root.path);
        let found_projects =
            scan_root_for_projects_blocking(root_path, patterns.clone(), central_dir.clone())
                .await?;

        roots_scanned += 1;
        let percent = if total_roots > 0 {
            (roots_scanned as u32 * 100) / total_roots as u32
        } else {
            100
        };

        for project in &found_projects {
            total_skills += project.skills.len();

            let _ = app.emit(
                "discover:found",
                FoundPayload {
                    project: project.clone(),
                },
            );
        }

        all_projects.extend(found_projects);

        let _ = app.emit(
            "discover:progress",
            ProgressPayload {
                percent: percent.min(100),
                current_path: root.path.clone(),
                skills_found: total_skills,
                projects_found: all_projects.len(),
            },
        );
    }

    let now = Utc::now().to_rfc3339();
    let mut found_skill_ids: Vec<String> = Vec::new();
    let mut discovered_inserts: Vec<db::DiscoveredSkillInsert<'_>> = Vec::new();

    for project in &all_projects {
        for skill in &project.skills {
            found_skill_ids.push(skill.id.clone());
            discovered_inserts.push(db::DiscoveredSkillInsert {
                id: &skill.id,
                name: &skill.name,
                description: skill.description.as_deref(),
                file_path: &skill.file_path,
                dir_path: &skill.dir_path,
                project_path: &skill.project_path,
                project_name: &skill.project_name,
                platform_id: &skill.platform_id,
                discovered_at: &now,
            });
        }
    }
    db::insert_discovered_skills(pool, &discovered_inserts).await?;

    reconcile_discovered_skills(pool, &enabled_roots, &found_skill_ids).await?;

    let total_projects = all_projects.len();

    let _ = app.emit(
        "discover:complete",
        CompletePayload {
            total_projects,
            total_skills,
        },
    );

    Ok(DiscoverResult {
        total_projects,
        total_skills,
        projects: all_projects,
    })
}
