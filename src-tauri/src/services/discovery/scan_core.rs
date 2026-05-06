//! Discover scan core — filesystem traversal only.
//!
//! Pure layer for project skill discovery. Knows about:
//!
//! - Recursion budget (`MAX_SCAN_DEPTH`).
//! - Skip policy (`SKIP_DIRS`, [`should_skip_dir`]).
//! - Per-project skill assembly via [`crate::services::scanner::scan_directory`]
//!   and central-skill detection.
//! - Cooperative cancellation via a caller-supplied `AtomicBool`.
//!
//! Knows nothing about: SQLite, Tauri events, scan-root persistence, or any
//! process-wide cancel flag. The orchestration layer in `scan.rs` wires those
//! pieces together and supplies the cancel flag.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::types::{DiscoveredProject, DiscoveredSkill};

/// Maximum recursion depth for the directory walker.
pub const MAX_SCAN_DEPTH: u32 = 8;

/// Directory names that should always be skipped during traversal for
/// performance. These never contain project-level skill dirs.
pub const SKIP_DIRS: &[&str] = &[
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

/// Check whether a directory name should be skipped during traversal.
///
/// - Always skip known heavy/irrelevant directories (node_modules, .git, …).
/// - At the root level (depth 0), skip hidden directories (dot-prefixed) since
///   they are typically user config dirs, not project directories.
/// - At deeper levels, allow hidden directories so platform skill patterns
///   like `.claude/skills/` inside project dirs remain detectable.
pub fn should_skip_dir(name: &str, depth: u32) -> bool {
    if SKIP_DIRS.contains(&name) {
        return true;
    }

    if depth == 0 && name.starts_with('.') {
        return true;
    }

    false
}

/// Recursively walk a scan root directory, returning the projects whose
/// directories contain at least one platform skill match.
///
/// `cancel` is polled at directory entry boundaries; when set, the walker
/// returns whatever it has accumulated so far without surfacing an error.
pub fn scan_root_for_projects(
    root: &Path,
    patterns: &[(String, String, PathBuf)],
    central_dir: &Path,
    cancel: &AtomicBool,
) -> Vec<DiscoveredProject> {
    let mut projects = Vec::new();
    let mut seen_project_paths = HashSet::new();
    scan_root_recursive(
        root,
        patterns,
        central_dir,
        cancel,
        0,
        &mut projects,
        &mut seen_project_paths,
    );
    projects
}

#[allow(clippy::too_many_arguments)]
fn scan_root_recursive(
    current_dir: &Path,
    patterns: &[(String, String, PathBuf)],
    central_dir: &Path,
    cancel: &AtomicBool,
    depth: u32,
    projects: &mut Vec<DiscoveredProject>,
    seen_project_paths: &mut HashSet<String>,
) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
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
            cancel,
            depth + 1,
            projects,
            seen_project_paths,
        );
    }
}
