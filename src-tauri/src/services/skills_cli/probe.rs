//! Constant-round-trip remote path probes for Skills CLI inventory.
//!
//! Paths are inlined in the script body (heredoc), never passed as argv, so
//! SSH round-trips stay constant in skill × platform count.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::services::installation::fs_util::{REASON_BROKEN_LINK, REASON_NOT_A_DIRECTORY};

use super::placement::ObservedSlot;
use super::SkillsCliManagedLinkKind;

const PATHS_HEREDOC: &str = "SKILLPORT_PATHS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathProbeKind {
    Link,
    Dir,
    File,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathProbe {
    pub path: String,
    pub kind: PathProbeKind,
    pub link_target: Option<String>,
}

pub(crate) fn build_path_probe_script(paths: &[String]) -> String {
    let mut script = String::from(
        r#"while IFS= read -r p; do
  [ -n "$p" ] || continue
  if [ -L "$p" ]; then k=link; t=$(readlink "$p" 2>/dev/null || true)
  elif [ -d "$p" ]; then k=dir; t=
  elif [ -e "$p" ]; then k=file; t=
  else k=absent; t=; fi
  printf '%s\t%s\t%s\n' "$p" "$k" "$t"
done <<'SKILLPORT_PATHS'
"#,
    );
    for path in paths {
        if path.is_empty() || path.contains('\n') || path.contains('\r') || path == PATHS_HEREDOC {
            continue;
        }
        script.push_str(path);
        script.push('\n');
    }
    script.push_str(PATHS_HEREDOC);
    script.push('\n');
    script
}

pub(crate) fn parse_path_probe_output(requested: &[String], stdout: &str) -> Vec<PathProbe> {
    let mut by_path = HashMap::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let Some(path) = parts.next() else {
            continue;
        };
        let kind = match parts.next().unwrap_or("absent") {
            "link" => PathProbeKind::Link,
            "dir" => PathProbeKind::Dir,
            "file" => PathProbeKind::File,
            _ => PathProbeKind::Absent,
        };
        let link_target = parts
            .next()
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        by_path.insert(
            path.to_string(),
            PathProbe {
                path: path.to_string(),
                kind,
                link_target,
            },
        );
    }
    requested
        .iter()
        .map(|path| {
            by_path.get(path).cloned().unwrap_or_else(|| PathProbe {
                path: path.clone(),
                kind: PathProbeKind::Absent,
                link_target: None,
            })
        })
        .collect()
}

pub(crate) fn index_probes(probes: &[PathProbe]) -> HashMap<String, PathProbe> {
    let mut map = HashMap::new();
    for probe in probes {
        map.insert(probe.path.clone(), probe.clone());
    }
    map
}

pub(crate) fn probe_exists(map: &HashMap<String, PathProbe>, path: &str) -> bool {
    map.get(path)
        .is_some_and(|probe| probe.kind != PathProbeKind::Absent)
}

pub(crate) fn canonical_owned_from_probe(probe: Option<&PathProbe>) -> bool {
    matches!(probe.map(|item| item.kind), Some(PathProbeKind::Dir))
}

pub(crate) fn observed_slot_from_probe(
    probe: &PathProbe,
    canonical: &str,
    link_kind: SkillsCliManagedLinkKind,
    posix: bool,
) -> ObservedSlot {
    match probe.kind {
        PathProbeKind::Absent => ObservedSlot::Absent,
        PathProbeKind::Dir => ObservedSlot::PlainDirectory,
        PathProbeKind::File => ObservedSlot::Conflict {
            reason_code: REASON_NOT_A_DIRECTORY.to_string(),
        },
        PathProbeKind::Link => {
            let Some(target) = probe
                .link_target
                .as_deref()
                .filter(|value| !value.is_empty())
            else {
                return ObservedSlot::Conflict {
                    reason_code: REASON_BROKEN_LINK.to_string(),
                };
            };
            if probe_resolves_to_canonical(&probe.path, target, canonical, posix) {
                ObservedSlot::ManagedLink {
                    kind: link_kind,
                    resolves_to_canonical: true,
                }
            } else {
                ObservedSlot::Conflict {
                    reason_code: crate::services::installation::fs_util::REASON_WRONG_LINK_TARGET
                        .to_string(),
                }
            }
        }
    }
}

pub(crate) fn collect_inventory_probe_paths(
    skill_names: impl Iterator<Item = String>,
    canonical_root: &str,
    platform_dirs: &[String],
    join_child: impl Fn(&str, &str) -> String,
    parent_of: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let names: Vec<String> = skill_names.collect();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |path: String| {
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    };
    for name in &names {
        push(join_child(canonical_root, name));
    }
    for name in &names {
        for dir in platform_dirs {
            push(join_child(dir, name));
        }
    }
    for dir in platform_dirs {
        push(dir.clone());
        if let Some(parent) = parent_of(dir) {
            push(parent);
        }
    }
    paths
}

fn probe_resolves_to_canonical(
    slot_path: &str,
    link_target: &str,
    canonical: &str,
    posix: bool,
) -> bool {
    let resolved = if posix {
        if link_target.starts_with('/') {
            link_target.to_string()
        } else if let Some(parent) = crate::targets::remote_parent(slot_path) {
            crate::targets::remote_join(&parent, link_target)
        } else {
            link_target.to_string()
        }
    } else {
        let raw = Path::new(link_target);
        if raw.is_absolute() {
            raw.to_string_lossy().into_owned()
        } else {
            Path::new(slot_path)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(raw)
                .to_string_lossy()
                .into_owned()
        }
    };
    normalize_compare(&resolved) == normalize_compare(canonical)
}

fn normalize_compare(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_inlines_paths_in_heredoc_not_argv_shape() {
        let script = build_path_probe_script(&[
            "/remote/home/.agents/skills/one".to_string(),
            "/remote/home/.cursor/skills/one".to_string(),
        ]);
        assert!(script.contains("<<'SKILLPORT_PATHS'"));
        assert!(script.contains("/remote/home/.agents/skills/one\n"));
        assert!(!script.contains("for path in /remote"));
    }

    #[test]
    fn missing_output_lines_are_absent() {
        let requested = vec!["/a".to_string(), "/b".to_string()];
        let probes = parse_path_probe_output(&requested, "/a\tdir\t\n");
        assert_eq!(probes[0].kind, PathProbeKind::Dir);
        assert_eq!(probes[1].kind, PathProbeKind::Absent);
    }

    #[test]
    fn unix_link_to_canonical_is_managed_symlink() {
        let probe = PathProbe {
            path: "/home/me/.cursor/skills/demo".to_string(),
            kind: PathProbeKind::Link,
            link_target: Some("/home/me/.agents/skills/demo".to_string()),
        };
        let slot = observed_slot_from_probe(
            &probe,
            "/home/me/.agents/skills/demo",
            SkillsCliManagedLinkKind::Symlink,
            true,
        );
        assert_eq!(
            slot,
            ObservedSlot::ManagedLink {
                kind: SkillsCliManagedLinkKind::Symlink,
                resolves_to_canonical: true,
            }
        );
    }

    #[test]
    fn windows_link_to_canonical_is_junction() {
        let probe = PathProbe {
            path: "/c/Users/me/.cursor/skills/demo".to_string(),
            kind: PathProbeKind::Link,
            link_target: Some("/c/Users/me/.agents/skills/demo".to_string()),
        };
        let slot = observed_slot_from_probe(
            &probe,
            "/c/Users/me/.agents/skills/demo",
            SkillsCliManagedLinkKind::WindowsJunction,
            true,
        );
        assert_eq!(
            slot,
            ObservedSlot::ManagedLink {
                kind: SkillsCliManagedLinkKind::WindowsJunction,
                resolves_to_canonical: true,
            }
        );
    }

    #[test]
    fn windows_dir_probe_is_plain_directory_not_managed_link() {
        let probe = PathProbe {
            path: "/c/Users/me/.cursor/skills/demo".to_string(),
            kind: PathProbeKind::Dir,
            link_target: None,
        };
        let slot = observed_slot_from_probe(
            &probe,
            "/c/Users/me/.agents/skills/demo",
            SkillsCliManagedLinkKind::WindowsJunction,
            true,
        );
        assert_eq!(slot, ObservedSlot::PlainDirectory);
    }
}
