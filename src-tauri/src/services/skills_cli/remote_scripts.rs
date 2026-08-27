//! Remote shell scripts for Skills CLI link / unlink / backup cleanup.
//!
//! Platform-slot deletes never use `rm -rf`. Recursive delete is generated
//! only for SkillPort canonical backup paths (`.skillport-remove-<id>`).

use crate::targets::{remote_parent, shell_quote};

use super::error::SkillsCliError;

pub(crate) const SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE: usize = 32;
#[allow(dead_code)]
pub(crate) const SKILLS_CLI_REMOTE_MUTATION_PROBE_OVERHEAD: usize = 1;

const VERIFIED_REMOVE_HEREDOC: &str = "SKILLPORT_VERIFIED_LINK_REMOVE";
const BACKUP_NAME_PREFIX: &str = ".skillport-remove-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifiedLinkRemoveStatus {
    Removed,
    SkippedNotLink,
    Absent,
}

#[allow(dead_code)]
pub(crate) fn remote_mutation_command_budget(n: usize) -> usize {
    n.div_ceil(SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE) + SKILLS_CLI_REMOTE_MUTATION_PROBE_OVERHEAD
}

pub(crate) fn is_windows_remote_os(remote_os: &str) -> bool {
    remote_os.eq_ignore_ascii_case("windows")
}

pub(crate) fn is_skillport_canonical_backup_path(path: &str) -> bool {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    name.starts_with(BACKUP_NAME_PREFIX) && name.len() > BACKUP_NAME_PREFIX.len()
}

pub(crate) fn build_create_managed_link_script(
    windows: bool,
    target: &str,
    link: &str,
) -> Result<String, SkillsCliError> {
    build_create_managed_links_script(windows, &[(target.to_string(), link.to_string())])
}

pub(crate) fn build_create_managed_links_script(
    windows: bool,
    pairs: &[(String, String)],
) -> Result<String, SkillsCliError> {
    if pairs.is_empty() {
        return Ok("set -eu\ntrue\n".to_string());
    }
    let mut script = String::from("set -eu\n");
    for (target, link) in pairs {
        if target.is_empty()
            || link.is_empty()
            || target.contains('\n')
            || link.contains('\n')
            || target.contains('\r')
            || link.contains('\r')
        {
            return Err(SkillsCliError::PlacementUnavailable);
        }
        let parent = remote_parent(link).ok_or(SkillsCliError::PlacementUnavailable)?;
        let quoted_parent = shell_quote(&parent);
        let quoted_target = shell_quote(target);
        let quoted_link = shell_quote(link);
        script.push_str("mkdir -p -- ");
        script.push_str(&quoted_parent);
        script.push('\n');
        if windows {
            // Remote hosts have no reparse API. Local Skills CLI forbids cmd.exe /
            // mklink because this process can call FSCTL_SET_REPARSE_POINT; that
            // premise does not hold over SSH. Never fall back to copy.
            // UNVERIFIED: depends on the remote sh layer's cmd.exe //c mklink /J.
            script.push_str("cmd.exe //c mklink /J ");
            script.push_str(&quoted_link);
            script.push(' ');
            script.push_str(&quoted_target);
            script.push('\n');
        } else {
            script.push_str("ln -s -- ");
            script.push_str(&quoted_target);
            script.push(' ');
            script.push_str(&quoted_link);
            script.push('\n');
        }
    }
    Ok(script)
}

pub(crate) fn build_verified_link_remove_script(windows: bool, paths: &[String]) -> String {
    let body = if windows {
        r#"while IFS= read -r p; do
  [ -n "$p" ] || continue
  if [ -L "$p" ]; then
    rmdir "$p"
    printf '%s\tremoved\n' "$p"
  elif [ -e "$p" ]; then printf '%s\tskipped_not_link\n' "$p"
  else printf '%s\tabsent\n' "$p"; fi
done <<'SKILLPORT_VERIFIED_LINK_REMOVE'
"#
    } else {
        r#"while IFS= read -r p; do
  [ -n "$p" ] || continue
  if [ -L "$p" ]; then rm -f "$p"; printf '%s\tremoved\n' "$p"
  elif [ -e "$p" ]; then printf '%s\tskipped_not_link\n' "$p"
  else printf '%s\tabsent\n' "$p"; fi
done <<'SKILLPORT_VERIFIED_LINK_REMOVE'
"#
    };
    let mut script = String::from(body);
    for path in paths {
        if path.is_empty()
            || path.contains('\n')
            || path.contains('\r')
            || path == VERIFIED_REMOVE_HEREDOC
        {
            continue;
        }
        script.push_str(path);
        script.push('\n');
    }
    script.push_str(VERIFIED_REMOVE_HEREDOC);
    script.push('\n');
    script
}

pub(crate) fn parse_verified_link_remove_output(
    requested: &[String],
    stdout: &str,
) -> Vec<(String, VerifiedLinkRemoveStatus)> {
    let mut by_path = std::collections::HashMap::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let Some(path) = parts.next() else {
            continue;
        };
        let status = match parts.next().unwrap_or("") {
            "removed" => VerifiedLinkRemoveStatus::Removed,
            "skipped_not_link" => VerifiedLinkRemoveStatus::SkippedNotLink,
            _ => VerifiedLinkRemoveStatus::Absent,
        };
        by_path.insert(path.to_string(), status);
    }
    requested
        .iter()
        .map(|path| {
            (
                path.clone(),
                by_path
                    .get(path)
                    .copied()
                    .unwrap_or(VerifiedLinkRemoveStatus::Absent),
            )
        })
        .collect()
}

pub(crate) fn build_rename_script(from: &str, to: &str) -> String {
    format!(
        "set -eu\nmv -- {from} {to}\n",
        from = shell_quote(from),
        to = shell_quote(to)
    )
}

pub(crate) fn build_atomic_replace_script(temp: &str, dest: &str) -> String {
    format!(
        "set -eu\nmv -f -- {temp} {dest}\n",
        temp = shell_quote(temp),
        dest = shell_quote(dest)
    )
}

pub(crate) fn build_remove_canonical_backup_script(path: &str) -> Result<String, SkillsCliError> {
    if !is_skillport_canonical_backup_path(path) {
        return Err(SkillsCliError::RecoveryRequired);
    }
    // rm -rf is allowed ONLY on SkillPort-generated canonical backup paths
    // whose last segment is `.skillport-remove-<operation_id>`. Never use this
    // script (or ConnectedRemoteTarget::remove_tree) on a platform slot.
    Ok(format!(
        "set -eu\nrm -rf -- {path}\n",
        path = shell_quote(path)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_remove_script_never_contains_recursive_rm() {
        let unix = build_verified_link_remove_script(false, &["/a/slot".to_string()]);
        let windows = build_verified_link_remove_script(true, &["/a/slot".to_string()]);
        assert!(!unix.contains("rm -rf"), "{unix}");
        assert!(!windows.contains("rm -rf"), "{windows}");
        assert!(unix.contains("rm -f"), "{unix}");
        assert!(windows.contains("rmdir"), "{windows}");
        assert!(unix.contains("skipped_not_link"), "{unix}");
    }

    #[test]
    fn create_scripts_never_copy() {
        let unix = build_create_managed_link_script(false, "/canon/demo", "/plat/demo").unwrap();
        let windows = build_create_managed_link_script(true, "/canon/demo", "/plat/demo").unwrap();
        assert!(unix.contains("ln -s"), "{unix}");
        assert!(windows.contains("mklink /J"), "{windows}");
        assert!(!unix.contains("cp "), "{unix}");
        assert!(!windows.contains("cp "), "{windows}");
        assert!(!unix.contains("rm -rf"), "{unix}");
        assert!(!windows.contains("rm -rf"), "{windows}");
    }

    #[test]
    fn backup_script_requires_skillport_prefix() {
        assert!(build_remove_canonical_backup_script("/root/demo").is_err());
        let ok = build_remove_canonical_backup_script("/root/.skillport-remove-abcd").unwrap();
        assert!(ok.contains("rm -rf --"), "{ok}");
        assert!(is_skillport_canonical_backup_path(
            "/agents/skills/.skillport-remove-uuid"
        ));
        assert!(!is_skillport_canonical_backup_path("/cursor/skills/demo"));
    }

    #[test]
    fn command_budget_matches_ceil_n_over_k_plus_c() {
        let k = SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE;
        assert_eq!(remote_mutation_command_budget(1), 2);
        assert_eq!(remote_mutation_command_budget(k), 2);
        assert_eq!(remote_mutation_command_budget(k + 1), 3);
        assert_eq!(remote_mutation_command_budget(4 * k), 5);
        assert_eq!(
            remote_mutation_command_budget(1),
            remote_mutation_command_budget(k)
        );
    }
}
