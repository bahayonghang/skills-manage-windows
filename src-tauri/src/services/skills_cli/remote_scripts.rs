//! Remote shell scripts for Skills CLI link / unlink / backup cleanup.
//!
//! Platform-slot deletes never use `rm -rf`. Recursive delete is generated
//! only for SkillPort canonical backup paths (`.skillport-remove-<id>`).

use crate::targets::{remote_join, remote_parent, shell_quote};

use super::argv::{NPX_JS_POSIX_RELATIVE, NPX_JS_POSIX_WELL_KNOWN};
use super::error::SkillsCliError;

/// Shared by remote doctor and the launcher probe so non-interactive SSH sees
/// Linuxbrew / Homebrew Node. Never wrap the probe in `bash -lc`.
pub(crate) const REMOTE_NODE_PATH_EXPORT: &str = concat!(
    r#"export PATH="/home/linuxbrew/.linuxbrew/bin:"#,
    r#"${HOME}/.linuxbrew/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}""#,
);

pub(crate) const SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE: usize = 32;
#[allow(dead_code)]
pub(crate) const SKILLS_CLI_REMOTE_MUTATION_PROBE_OVERHEAD: usize = 1;

const VERIFIED_REMOVE_HEREDOC: &str = "SKILLPORT_VERIFIED_LINK_REMOVE";
const BACKUP_NAME_PREFIX: &str = ".skillport-remove-";
const UPDATE_OP_DIR_PREFIX: &str = ".skillport-update-op-";
const UPDATE_STAGING_DIR_PREFIX: &str = ".skillport-update-staging-";

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

pub(crate) fn is_skillport_update_scratch_path(path: &str) -> bool {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    (name.starts_with(UPDATE_OP_DIR_PREFIX) && name.len() > UPDATE_OP_DIR_PREFIX.len())
        || (name.starts_with(UPDATE_STAGING_DIR_PREFIX)
            && name.len() > UPDATE_STAGING_DIR_PREFIX.len())
}

pub(crate) fn remote_update_backup_dir(canonical_root: &str, operation_id: &str) -> String {
    remote_join(
        canonical_root,
        &format!("{UPDATE_OP_DIR_PREFIX}{operation_id}"),
    )
}

pub(crate) fn remote_update_staging_dir(canonical_root: &str, operation_id: &str) -> String {
    remote_join(
        canonical_root,
        &format!("{UPDATE_STAGING_DIR_PREFIX}{operation_id}"),
    )
}

pub(crate) fn build_remote_doctor_probe_script() -> String {
    let mut script = String::from(REMOTE_NODE_PATH_EXPORT);
    script.push('\n');
    script.push_str(
        r#"printf 'XDG=%s\n' "${XDG_STATE_HOME-}"
printf 'HOME=%s\n' "$HOME"
if command -v node >/dev/null 2>&1; then
  printf 'NODEV=%s\n' "$(node --version 2>/dev/null)"
else
  printf 'NODEV=\n'
fi
"#,
    );
    script
}

/// One round-trip: resolve `node` then probe `npx-cli.js` in the same order as
/// local [`super::argv::NPX_JS_POSIX_RELATIVE`] then
/// [`super::argv::NPX_JS_POSIX_WELL_KNOWN`] entries.
pub(crate) fn build_remote_launcher_probe_script() -> String {
    let mut script = String::from(REMOTE_NODE_PATH_EXPORT);
    script.push('\n');
    script.push_str(
        r#"set -eu
NODE=$(command -v node 2>/dev/null || true)
printf 'NODE=%s\n' "$NODE"
if [ -z "$NODE" ]; then
  printf 'NPX=\n'
  exit 0
fi
NODE_DIR=$(dirname "$NODE")
NPX=
"#,
    );
    for relative in NPX_JS_POSIX_RELATIVE {
        script.push_str("if [ -z \"$NPX\" ] && [ -f \"$NODE_DIR/");
        script.push_str(relative);
        script.push_str("\" ]; then NPX=\"$NODE_DIR/");
        script.push_str(relative);
        script.push_str("\"; fi\n");
    }
    for well_known in NPX_JS_POSIX_WELL_KNOWN {
        script.push_str("if [ -z \"$NPX\" ] && [ -f ");
        script.push_str(&shell_quote(well_known));
        script.push_str(" ]; then NPX=");
        script.push_str(&shell_quote(well_known));
        script.push_str("; fi\n");
    }
    script.push_str("printf 'NPX=%s\\n' \"$NPX\"\n");
    script
}

pub(crate) fn parse_remote_launcher_probe(
    stdout: &str,
) -> Result<super::NodeLauncher, SkillsCliError> {
    let mut node = String::new();
    let mut npx = String::new();
    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("NODE=") {
            node = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("NPX=") {
            npx = value.trim().to_string();
        }
    }
    if node.is_empty() || npx.is_empty() {
        return Err(SkillsCliError::CliUnavailable);
    }
    Ok(super::NodeLauncher {
        program: std::path::PathBuf::from(node),
        npx_js: std::path::PathBuf::from(npx),
    })
}

/// Multi-root content hash. Emits `ROOT` / `MISSING` / `END`. Skips update
/// scratch dirs. Comparison uses the same framed digest as local hashing.
pub(crate) const REMOTE_SKILL_HASH_SCRIPT: &str = r#"
set -eu

if command -v sha256sum >/dev/null 2>&1; then
  hash_cmd='sha256sum'
elif command -v shasum >/dev/null 2>&1; then
  hash_cmd='shasum'
elif command -v openssl >/dev/null 2>&1; then
  hash_cmd='openssl'
else
  exit 86
fi

for root in "$@"; do
  if [ ! -d "$root" ]; then
    printf 'MISSING\t%s\n' "$root"
    continue
  fi
  printf 'ROOT\t%s\n' "$root"
  (cd "$root" && find . \( -name '.skillport-update-op-*' -o -name '.skillport-update-staging-*' \) -prune -o -type f -print | LC_ALL=C sort | while IFS= read -r path; do
    case "$path" in
      */.skillport-update-op-*|*/.skillport-update-staging-*) continue ;;
    esac
    case "$hash_cmd" in
      sha256sum) digest=$(sha256sum "$path") ;;
      shasum) digest=$(shasum -a 256 "$path") ;;
      openssl) digest=$(openssl dgst -sha256 -r "$path") ;;
    esac
    set -- $digest
    digest=$1
    size=$(wc -c < "$path" | tr -d '[:space:]')
    rel=${path#./}
    printf '%s\t%s\t%s\n' "$digest" "$size" "$rel"
  done)
  printf 'END\t%s\n' "$root"
done
"#;

pub(crate) fn build_copy_tree_if_exists_script(source: &str, dest: &str) -> String {
    format!(
        "set -eu\nif [ -d {source} ]; then\n  mkdir -p -- {dest}\n  cp -a -- {source}/. {dest}/\nfi\n",
        source = shell_quote(source),
        dest = shell_quote(dest)
    )
}

pub(crate) fn build_copy_trees_if_exist_script(pairs: &[(String, String)]) -> String {
    let mut script = String::from("set -eu\n");
    for (source, dest) in pairs {
        script.push_str(&build_copy_tree_if_exists_script(source, dest));
    }
    script
}

pub(crate) fn build_extract_tar_command(staging: &str) -> String {
    format!(
        "mkdir -p -- {staging} && tar -x -C {staging}",
        staging = shell_quote(staging)
    )
}

pub(crate) fn build_swap_canonicals_script(pairs: &[(String, String)]) -> String {
    let mut script = String::from("set -eu\n");
    for (staged, canonical) in pairs {
        let parent = remote_parent(canonical).unwrap_or_else(|| ".".to_string());
        script.push_str(&format!(
            "mkdir -p -- {parent}\nif [ -e {canonical} ]; then rm -rf -- {canonical}; fi\nmv -- {staged} {canonical}\n",
            parent = shell_quote(&parent),
            canonical = shell_quote(canonical),
            staged = shell_quote(staged)
        ));
    }
    script
}

pub(crate) fn build_remove_update_scratch_script(paths: &[&str]) -> Result<String, SkillsCliError> {
    if paths.is_empty() {
        return Ok("set -eu\ntrue\n".to_string());
    }
    for path in paths {
        if !is_skillport_update_scratch_path(path) {
            return Err(SkillsCliError::Io {
                context: "remove update scratch",
                source: std::io::Error::other("refused non-update recursive delete"),
            });
        }
    }
    let mut script = String::from("set -eu\n");
    for path in paths {
        script.push_str("rm -rf -- ");
        script.push_str(&shell_quote(path));
        script.push('\n');
    }
    Ok(script)
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
        assert!(is_skillport_update_scratch_path(
            "/agents/skills/.skillport-update-op-abc"
        ));
        assert!(is_skillport_update_scratch_path(
            "/agents/skills/.skillport-update-staging-abc"
        ));
        assert!(!is_skillport_update_scratch_path("/agents/skills/demo"));
        assert!(build_remove_update_scratch_script(&["/agents/skills/demo"]).is_err());
        let cleanup = build_remove_update_scratch_script(&[
            "/agents/skills/.skillport-update-op-abc",
            "/agents/skills/.skillport-update-staging-abc",
        ])
        .unwrap();
        assert!(cleanup.contains("rm -rf --"));
    }

    #[test]
    fn remote_node_probes_share_linuxbrew_path_and_npx_layout() {
        let probe = build_remote_launcher_probe_script();
        assert!(probe.contains("command -v node"));
        assert!(probe.contains("node_modules/npm/bin/npx-cli.js"));
        assert!(probe.contains("/home/linuxbrew/.linuxbrew/bin"));
        assert!(probe.contains("${HOME}/.linuxbrew/bin"));
        assert!(probe.contains("/opt/homebrew/bin"));
        assert!(probe.contains("/usr/local/bin"));
        assert!(probe.contains("../lib/node_modules/npm/bin/npx-cli.js"));
        assert!(!probe.contains("npx.cmd"));
        assert!(!probe.contains("cmd /c"));
        assert!(!probe.contains("bash -lc"));
        assert!(!probe.contains("zsh -lic"));
        let doctor = build_remote_doctor_probe_script();
        assert!(doctor.contains("/home/linuxbrew/.linuxbrew/bin"));
        assert!(doctor.contains("${HOME}/.linuxbrew/bin"));
        assert!(doctor.contains("NODEV"));
        assert!(!doctor.contains("bash -lc"));
        assert!(!doctor.contains("zsh -lic"));
        assert_eq!(
            doctor.lines().next(),
            probe.lines().next(),
            "doctor and launcher must share the PATH export"
        );
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
