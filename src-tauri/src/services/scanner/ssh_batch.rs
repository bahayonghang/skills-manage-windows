use std::collections::{HashMap, HashSet};

use crate::services::resource_budget::DEFAULT_FILE_BYTES;
use crate::targets::shell_quote;

use super::{parse_skill_md_content, ScannedSkill};

const PATH_MARKER: &str = "\u{001e}PATH\t";
const EOF_MARKER: &str = "\u{001f}EOF";
pub(super) const REMOTE_READ_CHUNK_SIZE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RemoteScanItem {
    RootOk {
        root: String,
    },
    RootParentOk {
        root: String,
    },
    RootMiss {
        root: String,
    },
    RootUnreadable {
        root: String,
    },
    Skill {
        root: String,
        skill_md_path: String,
        file_type: String,
        symlink_target: Option<String>,
    },
}

pub(super) fn build_probe_script(roots: &[String]) -> String {
    let mut script = String::from("set -eu\n");
    script.push_str("for root in");
    for root in roots {
        script.push(' ');
        script.push_str(&shell_quote(root));
    }
    script.push_str("; do\n");
    script.push_str("  if [ -d \"$root\" ]; then\n");
    script.push_str("    if [ -r \"$root\" ] && [ -x \"$root\" ]; then\n");
    script.push_str("      printf 'ROOT_OK\\t%s\\n' \"$root\"\n");
    script.push_str("      for dir in \"$root\"/* \"$root\"/.[!.]* \"$root\"/..?*; do\n");
    script.push_str("        [ -e \"$dir\" ] || continue\n");
    script.push_str("        if [ -d \"$dir\" ] || [ -L \"$dir\" ]; then\n");
    script.push_str("          file=\"$dir/SKILL.md\"\n");
    script.push_str("          if [ -f \"$file\" ]; then\n");
    script.push_str("            if [ -L \"$dir\" ]; then\n");
    script.push_str("              link=$(readlink \"$dir\" 2>/dev/null || true)\n");
    script.push_str(
        "              printf 'SKILL\\t%s\\t%s\\tsymlink\\t%s\\n' \"$root\" \"$file\" \"$link\"\n",
    );
    script.push_str("            else\n");
    script.push_str("              printf 'SKILL\\t%s\\t%s\\tdir\\t\\n' \"$root\" \"$file\"\n");
    script.push_str("            fi\n");
    script.push_str("          fi\n");
    script.push_str("        fi\n");
    script.push_str("      done\n");
    script.push_str("    else\n");
    script.push_str("      printf 'ROOT_UNREADABLE\\t%s\\n' \"$root\"\n");
    script.push_str("    fi\n");
    script.push_str("  elif [ -d \"$(dirname \"$root\")\" ]; then\n");
    script.push_str("    printf 'ROOT_PARENT_OK\\t%s\\n' \"$root\"\n");
    script.push_str("  else\n");
    script.push_str("    printf 'ROOT_MISS\\t%s\\n' \"$root\"\n");
    script.push_str("  fi\n");
    script.push_str("done\n");
    script
}

pub(super) fn parse_probe_output(output: &str) -> Vec<RemoteScanItem> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(5, '\t');
            match parts.next()? {
                "ROOT_OK" => Some(RemoteScanItem::RootOk {
                    root: parts.next()?.to_string(),
                }),
                "ROOT_PARENT_OK" => Some(RemoteScanItem::RootParentOk {
                    root: parts.next()?.to_string(),
                }),
                "ROOT_MISS" => Some(RemoteScanItem::RootMiss {
                    root: parts.next()?.to_string(),
                }),
                "ROOT_UNREADABLE" => Some(RemoteScanItem::RootUnreadable {
                    root: parts.next()?.to_string(),
                }),
                "SKILL" => Some(RemoteScanItem::Skill {
                    root: parts.next()?.to_string(),
                    skill_md_path: parts.next()?.to_string(),
                    file_type: parts.next().unwrap_or("dir").to_string(),
                    symlink_target: parts
                        .next()
                        .map(str::to_string)
                        .filter(|value| !value.is_empty()),
                }),
                _ => None,
            }
        })
        .collect()
}

pub(super) fn build_batch_read_script(skill_md_paths: &[String]) -> String {
    let read_bytes = DEFAULT_FILE_BYTES + 1;
    let mut script = String::from("set -eu\n");
    script.push_str("for path in");
    for path in skill_md_paths {
        script.push(' ');
        script.push_str(&shell_quote(path));
    }
    script.push_str("; do\n");
    script.push_str(&format!("  printf '{}%s\\n' \"$path\"\n", PATH_MARKER));
    script.push_str("  if [ -r \"$path\" ]; then\n");
    script.push_str("    size=$(LC_ALL=C wc -c < \"$path\") || size=\n");
    script.push_str(&format!(
        "    case \"$size\" in ''|*[!0-9]*) ;; *) [ \"$size\" -le {DEFAULT_FILE_BYTES} ] && dd if=\"$path\" bs={read_bytes} count=1 2>/dev/null || true ;; esac\n"
    ));
    script.push_str("  fi\n");
    script.push_str(&format!("  printf '{}\\n'\n", EOF_MARKER));
    script.push_str("done\n");
    script
}

pub(super) fn parse_batch_read_output(output: &str) -> HashMap<String, String> {
    let mut content_by_path = HashMap::new();
    let mut remaining = output;
    while let Some(start) = remaining.find(PATH_MARKER) {
        let after_marker = &remaining[start + PATH_MARKER.len()..];
        let Some((path, after_path)) = after_marker.split_once('\n') else {
            break;
        };
        let Some(end) = after_path.find(EOF_MARKER) else {
            break;
        };
        let body = &after_path[..end];
        if body.len() as u64 <= DEFAULT_FILE_BYTES {
            content_by_path.insert(path.to_string(), body.to_string());
        }
        remaining = &after_path[end + EOF_MARKER.len()..];
    }

    content_by_path
}

#[cfg(test)]
pub(super) fn encode_batch_read_output(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(path, content)| format!("{PATH_MARKER}{path}\n{content}{EOF_MARKER}\n"))
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn build_scanned_skills_from_contents(
    items: &[RemoteScanItem],
    content_by_path: &HashMap<String, String>,
    is_central: bool,
) -> Vec<ScannedSkill> {
    items
        .iter()
        .filter_map(|item| {
            let RemoteScanItem::Skill {
                skill_md_path,
                file_type,
                symlink_target,
                ..
            } = item
            else {
                return None;
            };
            let content = content_by_path.get(skill_md_path)?;
            let info = parse_skill_md_content(content)?;
            let dir_path = skill_md_path
                .strip_suffix("/SKILL.md")
                .unwrap_or(skill_md_path)
                .to_string();
            let name = dir_path.rsplit('/').next()?.to_string();
            let link_type = if file_type == "symlink" {
                "symlink".to_string()
            } else if is_central {
                "native".to_string()
            } else {
                "copy".to_string()
            };
            Some(ScannedSkill {
                id: name.to_lowercase().replace(' ', "-"),
                name: info.name,
                description: info.description,
                file_path: skill_md_path.clone(),
                dir_path,
                link_type,
                symlink_target: symlink_target.clone(),
                is_central,
                fs_created_at: None,
                fs_updated_at: None,
            })
        })
        .collect()
}

pub(super) fn unique_skill_paths(items: &[RemoteScanItem]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for item in items {
        if let RemoteScanItem::Skill { skill_md_path, .. } = item {
            if seen.insert(skill_md_path.clone()) {
                paths.push(skill_md_path.clone());
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_probe_script_quotes_roots() {
        let script = build_probe_script(&["/home/alice/.claude/skills".to_string()]);
        assert!(script.contains("'/home/alice/.claude/skills'"));
        assert!(script.contains("ROOT_OK"));
        assert!(script.contains("ROOT_MISS"));
        assert!(script.contains("ROOT_UNREADABLE"));
        assert!(script.contains("[ -r \"$root\" ] && [ -x \"$root\" ]"));
    }

    #[test]
    fn parse_probe_output_recovers_roots_and_skills() {
        let items = parse_probe_output(
            "ROOT_OK\t/home/alice/.claude/skills\nSKILL\t/home/alice/.claude/skills\t/home/alice/.claude/skills/foo/SKILL.md\nROOT_MISS\t/home/alice/.kiro/skills\nROOT_UNREADABLE\t/home/alice/.agents/skills\n",
        );

        assert_eq!(
            items,
            vec![
                RemoteScanItem::RootOk {
                    root: "/home/alice/.claude/skills".to_string()
                },
                RemoteScanItem::Skill {
                    root: "/home/alice/.claude/skills".to_string(),
                    skill_md_path: "/home/alice/.claude/skills/foo/SKILL.md".to_string(),
                    file_type: "dir".to_string(),
                    symlink_target: None,
                },
                RemoteScanItem::RootMiss {
                    root: "/home/alice/.kiro/skills".to_string()
                },
                RemoteScanItem::RootUnreadable {
                    root: "/home/alice/.agents/skills".to_string()
                },
            ]
        );
    }

    #[test]
    fn parse_probe_output_recovers_symlink_and_parent_visible_entries() {
        let items = parse_probe_output(
            "ROOT_PARENT_OK\t/home/alice/.openclaw/skills\nSKILL\t/home/alice/.claude/skills\t/home/alice/.claude/skills/foo/SKILL.md\tsymlink\t/central/foo\n",
        );

        assert_eq!(
            items,
            vec![
                RemoteScanItem::RootParentOk {
                    root: "/home/alice/.openclaw/skills".to_string()
                },
                RemoteScanItem::Skill {
                    root: "/home/alice/.claude/skills".to_string(),
                    skill_md_path: "/home/alice/.claude/skills/foo/SKILL.md".to_string(),
                    file_type: "symlink".to_string(),
                    symlink_target: Some("/central/foo".to_string()),
                }
            ]
        );
    }

    #[test]
    fn build_batch_read_script_quotes_paths() {
        let script = build_batch_read_script(&["/tmp/demo path/SKILL.md".to_string()]);
        assert!(script.contains("'/tmp/demo path/SKILL.md'"));
        assert!(script.contains(PATH_MARKER));
        assert!(script.contains(EOF_MARKER));
        assert!(script.contains("wc -c"));
        assert!(script.contains("bs=1048577 count=1"));
        assert!(!script.contains("cat --"));
    }

    #[test]
    fn parse_batch_read_output_preserves_special_characters() {
        let encoded = encode_batch_read_output(&[(
            "/home/alice/.claude/skills/foo/SKILL.md".to_string(),
            "---\nname: Demo\ndescription: line\twith tab\n---\nBody\n".to_string(),
        )]);
        let parsed = parse_batch_read_output(&encoded);

        assert_eq!(
            parsed.get("/home/alice/.claude/skills/foo/SKILL.md"),
            Some(&"---\nname: Demo\ndescription: line\twith tab\n---\nBody\n".to_string())
        );
    }

    #[test]
    fn parse_batch_read_output_drops_limit_plus_one_body() {
        let encoded = encode_batch_read_output(&[(
            "/tmp/oversized/SKILL.md".to_string(),
            "a".repeat(DEFAULT_FILE_BYTES as usize + 1),
        )]);

        assert!(parse_batch_read_output(&encoded).is_empty());
    }

    #[test]
    fn build_scanned_skills_from_contents_parses_skill_md() {
        let path = "/home/alice/.claude/skills/foo/SKILL.md".to_string();
        let mut content_by_path = HashMap::new();
        content_by_path.insert(
            path.clone(),
            "---\nname: Foo\ndescription: Demo\n---\n".to_string(),
        );

        let scanned = build_scanned_skills_from_contents(
            &[RemoteScanItem::Skill {
                root: "/home/alice/.claude/skills".to_string(),
                skill_md_path: path.clone(),
                file_type: "dir".to_string(),
                symlink_target: None,
            }],
            &content_by_path,
            false,
        );

        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].id, "foo");
        assert_eq!(scanned[0].file_path, path);
        assert_eq!(scanned[0].dir_path, "/home/alice/.claude/skills/foo");
        assert_eq!(scanned[0].link_type, "copy");
    }

    #[test]
    fn unique_skill_paths_deduplicates_duplicate_entries() {
        let items = vec![
            RemoteScanItem::Skill {
                root: "/r1".to_string(),
                skill_md_path: "/r1/foo/SKILL.md".to_string(),
                file_type: "dir".to_string(),
                symlink_target: None,
            },
            RemoteScanItem::Skill {
                root: "/r2".to_string(),
                skill_md_path: "/r1/foo/SKILL.md".to_string(),
                file_type: "dir".to_string(),
                symlink_target: None,
            },
        ];

        assert_eq!(
            unique_skill_paths(&items),
            vec!["/r1/foo/SKILL.md".to_string()]
        );
    }
}
