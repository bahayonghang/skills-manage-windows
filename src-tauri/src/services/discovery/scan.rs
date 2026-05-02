use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use tauri::Emitter;

use crate::db::{self, DbPool};
use crate::paths;

use super::roots::platform_skill_patterns;
use super::types::{
    CompletePayload, DiscoverResult, DiscoveredProject, DiscoveredSkill, FoundPayload,
    ProgressPayload, ScanRoot,
};

static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

/// Maximum recursion depth for the directory walker.
const MAX_SCAN_DEPTH: u32 = 8;

/// Directory names that should always be skipped during traversal for
/// performance (these never contain project-level skill dirs).
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "build",
    "dist",
    ".cache",
    "__pycache__",
    ".next",
    ".nuxt",
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".idea",
    ".vscode",
];

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

/// Check whether a directory name should be skipped during traversal.
///
/// - Always skip known heavy/irrelevant directories (node_modules, .git, etc.).
/// - At the root level (depth 0), skip hidden directories (dot-prefixed) since
///   they are typically user config dirs, not project directories.
/// - At deeper levels, allow hidden directories so we can detect platform
///   skill patterns like `.claude/skills/` inside project dirs.
pub fn should_skip_dir(name: &str, depth: u32) -> bool {
    if SKIP_DIRS.contains(&name) {
        return true;
    }

    if depth == 0 && name.starts_with('.') {
        return true;
    }

    false
}

/// Recursively walk a scan root directory, looking for project-level skill dirs.
pub fn scan_root_for_projects(
    root: &Path,
    patterns: &[(String, String, PathBuf)],
    central_dir: &Path,
) -> Vec<DiscoveredProject> {
    let mut projects = Vec::new();
    let mut seen_project_paths = HashSet::new();
    scan_root_recursive(
        root,
        patterns,
        central_dir,
        0,
        &mut projects,
        &mut seen_project_paths,
    );
    projects
}

async fn scan_root_for_projects_blocking(
    root: PathBuf,
    patterns: Vec<(String, String, PathBuf)>,
    central_dir: PathBuf,
) -> Result<Vec<DiscoveredProject>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scan_root_for_projects(&root, &patterns, &central_dir)
    })
    .await
    .map_err(|e| format!("Failed to join Discover scan task: {}", e))
}

fn scan_root_recursive(
    current_dir: &Path,
    patterns: &[(String, String, PathBuf)],
    central_dir: &Path,
    depth: u32,
    projects: &mut Vec<DiscoveredProject>,
    seen_project_paths: &mut HashSet<String>,
) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    if SCAN_CANCEL.load(Ordering::Relaxed) {
        return;
    }

    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if SCAN_CANCEL.load(Ordering::Relaxed) {
            break;
        }

        let entry_path = entry.path();

        let meta = match std::fs::metadata(&entry_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_dir() {
            continue;
        }

        let dir_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if should_skip_dir(dir_name, depth) {
            continue;
        }

        let mut project_skills: Vec<DiscoveredSkill> = Vec::new();

        for (agent_id, display_name, rel_pattern) in patterns {
            let skill_dir = entry_path.join(rel_pattern);

            if !skill_dir.exists() {
                continue;
            }

            let scanned = crate::services::scanner::scan_directory(&skill_dir, false);

            for skill in scanned {
                let project_name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let qualified_id = format!(
                    "{}__{}__{}",
                    agent_id,
                    project_name.to_lowercase().replace(' ', "-"),
                    skill.id
                );

                let central_skill_path = central_dir.join(&skill.id);
                let is_already_central = central_skill_path.exists();

                project_skills.push(DiscoveredSkill {
                    id: qualified_id,
                    name: skill.name,
                    description: skill.description,
                    file_path: skill.file_path,
                    dir_path: skill.dir_path,
                    platform_id: agent_id.clone(),
                    platform_name: display_name.clone(),
                    project_path: entry_path.to_string_lossy().into_owned(),
                    project_name: project_name.clone(),
                    is_already_central,
                });
            }
        }

        if !project_skills.is_empty() {
            let project_path_key = entry_path.to_string_lossy().into_owned();
            if !seen_project_paths.contains(&project_path_key) {
                seen_project_paths.insert(project_path_key.clone());
                let project_name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                projects.push(DiscoveredProject {
                    project_path: project_path_key,
                    project_name,
                    skills: project_skills,
                });
            }
        }

        scan_root_recursive(
            &entry_path,
            patterns,
            central_dir,
            depth + 1,
            projects,
            seen_project_paths,
        );
    }
}

/// Reconcile the `discovered_skills` table after a scan.
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
