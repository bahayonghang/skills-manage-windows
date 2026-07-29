use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

use crate::paths;
use crate::targets::{
    connect_remote_target, remote_join, remote_parent, shell_quote, ActiveTarget,
    ConnectedRemoteTarget,
};

mod error;
mod types;
pub use error::LocalRemoteSyncError;
pub use types::*;

struct LocalRemoteSyncPlan {
    preview: LocalRemoteSyncPreview,
    repo_snapshot: LocalSnapshot,
    skill_snapshots: Vec<(LocalSnapshot, LocalRemoteSyncItemPreview)>,
}

const REPO_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".pnpm-store",
    ".pytest_cache",
    ".ruff_cache",
    ".mypy_cache",
    ".tmp",
    ".turbo",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "dist-docs",
    "node_modules",
    "outputs",
    "target",
    "target-codex",
    "venv",
];

const REMOTE_SNAPSHOT_HASH_SCRIPT: &str = r#"
set -eu

root=$1

if [ ! -d "$root" ]; then
  printf 'MISSING\n'
  exit 0
fi

if command -v sha256sum >/dev/null 2>&1; then
  hash_cmd='sha256sum'
elif command -v shasum >/dev/null 2>&1; then
  hash_cmd='shasum'
elif command -v openssl >/dev/null 2>&1; then
  hash_cmd='openssl'
else
  exit 86
fi

while IFS= read -r rel || [ -n "$rel" ]; do
  case "$rel" in
    ""|/*|../*|*/../*|*/..|"."|"..")
      printf 'MISSING_FILE\t%s\n' "$rel"
      continue
      ;;
  esac

  path="$root/$rel"
  if [ ! -f "$path" ]; then
    printf 'MISSING_FILE\t%s\n' "$rel"
    continue
  fi

  case "$hash_cmd" in
    sha256sum) digest=$(sha256sum "$path") ;;
    shasum) digest=$(shasum -a 256 "$path") ;;
    openssl) digest=$(openssl dgst -sha256 -r "$path") ;;
  esac
  set -- $digest
  printf '%s\t%s\n' "$1" "$rel"
done
"#;

const REMOTE_APPLY_ARCHIVE_SCRIPT: &str = r#"
set -eu

target_dir=$1
parent_dir=$2
staging_dir=$3
backup_dir=$4

mkdir -p -- "$parent_dir"
rm -rf -- "$staging_dir" "$backup_dir"
mkdir -p -- "$staging_dir"
tar -xzf - -C "$staging_dir"

if [ -e "$target_dir" ]; then
  if [ -d "$target_dir" ]; then
    cp -R "$target_dir" "$backup_dir"
  else
    mv "$target_dir" "$backup_dir"
    mkdir -p -- "$target_dir"
  fi

  if cp -R "$staging_dir"/. "$target_dir"/; then
    rm -rf -- "$staging_dir" "$backup_dir"
  else
    rc=$?
    rm -rf -- "$target_dir"
    if [ -e "$backup_dir" ]; then mv "$backup_dir" "$target_dir"; fi
    rm -rf -- "$staging_dir"
    exit "$rc"
  fi
else
  mv "$staging_dir" "$target_dir"
fi
"#;

pub async fn preview_local_remote_sync_impl(
    active_target: ActiveTarget,
    repo_path: Option<String>,
) -> Result<LocalRemoteSyncPreview, LocalRemoteSyncError> {
    Ok(build_sync_plan(active_target, repo_path).await?.preview)
}

pub async fn apply_local_remote_sync_impl(
    active_target: ActiveTarget,
    repo_path: Option<String>,
) -> Result<LocalRemoteSyncApplyResult, LocalRemoteSyncError> {
    let plan = build_sync_plan(active_target.clone(), repo_path).await?;
    let connection = connect_remote_target(&active_target)
        .await
        .map_err(|e| LocalRemoteSyncError::Remote(e.to_string()))?;
    let mut result = LocalRemoteSyncApplyResult {
        target_id: plan.preview.target_id.clone(),
        target_label: plan.preview.target_label.clone(),
        synced_repo: None,
        synced_skills: Vec::new(),
        skipped_skills: Vec::new(),
        failed: Vec::new(),
    };

    match plan.preview.repo.status {
        LocalRemoteSyncItemStatus::Skip => {}
        LocalRemoteSyncItemStatus::Add | LocalRemoteSyncItemStatus::Update => {
            if let Err(error) = apply_snapshot(
                &connection,
                &plan.repo_snapshot,
                &plan.preview.repo.remote_path,
            )
            .await
            {
                result.failed.push(LocalRemoteSyncFailure {
                    id: plan.preview.repo.id.clone(),
                    label: plan.preview.repo.label.clone(),
                    target_path: plan.preview.repo.remote_path.clone(),
                    error: error.to_string(),
                });
                return Ok(result);
            }
            result.synced_repo = Some(plan.preview.repo.clone());
        }
        LocalRemoteSyncItemStatus::Error => {
            result.failed.push(LocalRemoteSyncFailure {
                id: plan.preview.repo.id.clone(),
                label: plan.preview.repo.label.clone(),
                target_path: plan.preview.repo.remote_path.clone(),
                error: plan
                    .preview
                    .repo
                    .error
                    .clone()
                    .unwrap_or_else(|| "Repository preview failed.".to_string()),
            });
            return Ok(result);
        }
    }

    let mut skill_snapshots = plan.skill_snapshots;
    skill_snapshots.sort_by(|left, right| left.0.id.cmp(&right.0.id));
    for (snapshot, preview) in skill_snapshots {
        match preview.status {
            LocalRemoteSyncItemStatus::Skip => result.skipped_skills.push(preview),
            LocalRemoteSyncItemStatus::Add | LocalRemoteSyncItemStatus::Update => {
                match apply_snapshot(&connection, &snapshot, &preview.remote_path).await {
                    Ok(()) => result.synced_skills.push(preview),
                    Err(error) => result.failed.push(LocalRemoteSyncFailure {
                        id: preview.id,
                        label: preview.label,
                        target_path: preview.remote_path,
                        error: error.to_string(),
                    }),
                }
            }
            LocalRemoteSyncItemStatus::Error => {
                result.failed.push(LocalRemoteSyncFailure {
                    id: preview.id,
                    label: preview.label,
                    target_path: preview.remote_path,
                    error: preview
                        .error
                        .unwrap_or_else(|| "Skill preview failed.".to_string()),
                });
            }
        }
    }

    Ok(result)
}

async fn build_sync_plan(
    active_target: ActiveTarget,
    repo_path: Option<String>,
) -> Result<LocalRemoteSyncPlan, LocalRemoteSyncError> {
    let repo_root = resolve_repo_root(repo_path.as_deref())?;
    let repo_root_for_snapshot = repo_root.clone();
    let repo_snapshot = crate::fs_util::run_blocking_fs_with(
        "local repository snapshot",
        move || collect_repo_snapshot(&repo_root_for_snapshot),
        LocalRemoteSyncError::task_join,
    )
    .await?;
    let skills_root = paths::central_skills_dir();
    let skills_root_for_snapshot = skills_root.clone();
    let skill_snapshots = crate::fs_util::run_blocking_fs_with(
        "local skill snapshots",
        move || collect_skill_snapshots(&skills_root_for_snapshot),
        LocalRemoteSyncError::task_join,
    )
    .await?;
    let connection = connect_remote_target(&active_target)
        .await
        .map_err(|e| LocalRemoteSyncError::Remote(e.to_string()))?;
    let remote_home = connection.remote_home().trim_end_matches('/').to_string();
    let repo_remote_root = remote_join(&paths::remote_repos_root(&remote_home), &repo_snapshot.id);
    let skills_remote_root = paths::remote_central_skills_root(&remote_home);

    let repo_remote_hash = remote_snapshot_hash(&connection, &repo_remote_root, &repo_snapshot)
        .await
        .map_err(|error| LocalRemoteSyncError::RepoRemoteInspect(error.to_string()))?;
    let repo_preview = preview_item(
        &repo_snapshot,
        LocalRemoteSyncItemKind::Repo,
        repo_remote_root,
        repo_remote_hash,
        None,
    );

    let mut skill_previews = Vec::with_capacity(skill_snapshots.len());
    let mut snapshots_with_preview = Vec::with_capacity(skill_snapshots.len());
    for snapshot in skill_snapshots {
        let remote_path = remote_join(&skills_remote_root, &snapshot.id);
        let (remote_hash, error) =
            match remote_snapshot_hash(&connection, &remote_path, &snapshot).await {
                Ok(hash) => (hash, None),
                Err(error) => (None, Some(error.to_string())),
            };
        let preview = preview_item(
            &snapshot,
            LocalRemoteSyncItemKind::Skill,
            remote_path,
            remote_hash,
            error,
        );
        skill_previews.push(preview.clone());
        snapshots_with_preview.push((snapshot, preview));
    }

    let total_file_count = repo_preview.file_count
        + skill_previews
            .iter()
            .map(|item| item.file_count)
            .sum::<usize>();
    let total_byte_count = repo_preview.byte_count
        + skill_previews
            .iter()
            .map(|item| item.byte_count)
            .sum::<u64>();

    Ok(LocalRemoteSyncPlan {
        preview: LocalRemoteSyncPreview {
            target_id: active_target.id().to_string(),
            target_label: active_target.label().to_string(),
            repo_root: path_display(&repo_root),
            skills_root: path_display(&skills_root),
            repo: repo_preview,
            skills: skill_previews,
            total_file_count,
            total_byte_count,
        },
        repo_snapshot,
        skill_snapshots: snapshots_with_preview,
    })
}

fn resolve_repo_root(repo_path: Option<&str>) -> Result<PathBuf, LocalRemoteSyncError> {
    let root = match repo_path.map(str::trim).filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir().map_err(|error| {
            LocalRemoteSyncError::io("Failed to resolve current directory", error)
        })?,
    };
    if !root.is_dir() {
        return Err(LocalRemoteSyncError::RepoPathNotDirectory(
            root.display().to_string(),
        ));
    }
    root.canonicalize().map_err(|error| {
        LocalRemoteSyncError::io(
            format!(
                "Failed to canonicalize repository path '{}'",
                root.display()
            ),
            error,
        )
    })
}

fn preview_item(
    snapshot: &LocalSnapshot,
    kind: LocalRemoteSyncItemKind,
    remote_path: String,
    remote_hash: Option<String>,
    error: Option<String>,
) -> LocalRemoteSyncItemPreview {
    let status = if error.is_some() {
        LocalRemoteSyncItemStatus::Error
    } else {
        match remote_hash.as_deref() {
            None => LocalRemoteSyncItemStatus::Add,
            Some(hash) if hash == snapshot.hash => LocalRemoteSyncItemStatus::Skip,
            Some(_) => LocalRemoteSyncItemStatus::Update,
        }
    };

    LocalRemoteSyncItemPreview {
        id: snapshot.id.clone(),
        label: snapshot.label.clone(),
        kind,
        local_path: path_display(&snapshot.root),
        remote_path,
        file_count: snapshot.file_count,
        byte_count: snapshot.byte_count,
        local_hash: snapshot.hash.clone(),
        remote_hash,
        status,
        error,
    }
}

pub fn repo_slug(root: &Path) -> String {
    let root_string = root.to_string_lossy().replace('\\', "/");
    let name = root_string
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("repo");

    let slug = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(80)
        .collect::<String>();
    if slug.trim().is_empty() {
        "repo".to_string()
    } else {
        slug
    }
}

pub fn is_safe_relative_archive_path(path: &str) -> bool {
    if path.trim().is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains(':')
        || path.contains('\\')
    {
        return false;
    }
    !Path::new(path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
}

pub fn collect_repo_snapshot(root: &Path) -> Result<LocalSnapshot, LocalRemoteSyncError> {
    let root = root.canonicalize().map_err(|error| {
        LocalRemoteSyncError::io(
            format!(
                "Failed to canonicalize repository path '{}'",
                root.display()
            ),
            error,
        )
    })?;
    let mut files = Vec::new();
    collect_snapshot_files(&root, &root, &mut files, true)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    build_snapshot(repo_slug(&root), repo_slug(&root), root, files)
}

pub fn collect_skill_snapshots(root: &Path) -> Result<Vec<LocalSnapshot>, LocalRemoteSyncError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut dirs = std::fs::read_dir(root)
        .map_err(|error| {
            LocalRemoteSyncError::io(
                format!("Failed to read local skills directory '{}'", root.display()),
                error,
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            LocalRemoteSyncError::io("Failed to read local skills directory entry", error)
        })?;
    dirs.sort_by_key(|entry| entry.path());

    let mut snapshots = Vec::new();
    for entry in dirs {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            LocalRemoteSyncError::io(
                format!("Failed to inspect local skill '{}'", path.display()),
                error,
            )
        })?;
        if !file_type.is_dir() || !path.join("SKILL.md").is_file() {
            continue;
        }
        let id = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| LocalRemoteSyncError::SkillDirNameNotUtf8(path.display().to_string()))?
            .to_string();
        if !is_safe_relative_archive_path(&id) {
            return Err(LocalRemoteSyncError::UnsafeSkillId(id));
        }
        let mut files = Vec::new();
        collect_snapshot_files(&path, &path, &mut files, false)?;
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        snapshots.push(build_snapshot(id.clone(), id, path, files)?);
    }
    snapshots.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(snapshots)
}

fn build_snapshot(
    id: String,
    label: String,
    root: PathBuf,
    files: Vec<SnapshotFile>,
) -> Result<LocalSnapshot, LocalRemoteSyncError> {
    for file in &files {
        if !is_safe_relative_archive_path(&file.relative_path) {
            return Err(LocalRemoteSyncError::UnsafeSnapshotPath {
                id,
                path: file.relative_path.clone(),
            });
        }
    }
    let file_count = files.len();
    let byte_count = files.iter().map(|file| file.bytes.len() as u64).sum();
    let hash = hash_snapshot_files(&files);
    Ok(LocalSnapshot {
        id,
        label,
        root,
        files,
        file_count,
        byte_count,
        hash,
    })
}

fn collect_snapshot_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<SnapshotFile>,
    apply_repo_excludes: bool,
) -> Result<(), LocalRemoteSyncError> {
    let mut entries = std::fs::read_dir(current)
        .map_err(|error| {
            LocalRemoteSyncError::io(format!("Failed to read '{}'", current.display()), error)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| LocalRemoteSyncError::io("Failed to read directory entry", error))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            LocalRemoteSyncError::io(format!("Failed to inspect '{}'", path.display()), error)
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if apply_repo_excludes && should_exclude_repo_dir(root, &path)? {
                continue;
            }
            collect_snapshot_files(root, &path, files, apply_repo_excludes)?;
        } else if file_type.is_file() {
            let relative_path = relative_path_string(root, &path)?;
            if !is_safe_relative_archive_path(&relative_path) {
                return Err(LocalRemoteSyncError::UnsafeLocalPath(
                    path.display().to_string(),
                ));
            }
            let bytes = std::fs::read(&path).map_err(|error| {
                LocalRemoteSyncError::io(format!("Failed to read '{}'", path.display()), error)
            })?;
            files.push(SnapshotFile {
                relative_path,
                bytes,
            });
        }
    }
    Ok(())
}

fn should_exclude_repo_dir(root: &Path, path: &Path) -> Result<bool, LocalRemoteSyncError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| LocalRemoteSyncError::RelativePath {
            path: path.display().to_string(),
            source: error,
        })?;
    let components = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if components
        .windows(2)
        .any(|pair| pair[0] == paths::APP_DATA_DIR_NAME && pair[1] == paths::TARGETS_CACHE_DIR_NAME)
    {
        return Ok(true);
    }

    Ok(components
        .iter()
        .any(|component| REPO_EXCLUDED_DIRS.contains(&component.as_str())))
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, LocalRemoteSyncError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| LocalRemoteSyncError::RelativePath {
            path: path.display().to_string(),
            source: error,
        })?;
    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_string_lossy().into_owned()),
            _ => Err(LocalRemoteSyncError::UnsupportedPathComponents(
                path.display().to_string(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join("/"))
}

fn hash_snapshot_files(files: &[SnapshotFile]) -> String {
    let entries = files.iter().map(|file| {
        let digest = Sha256::digest(&file.bytes);
        (file.relative_path.clone(), hex_digest(&digest))
    });
    hash_entries(entries)
}

fn hash_entries<I>(entries: I) -> String
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (path, digest) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0xff]);
        hasher.update(digest.as_bytes());
        hasher.update([0xfe]);
    }
    format!("sha256-manifest:{}", hex_digest(&hasher.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn build_archive(snapshot: &LocalSnapshot) -> Result<Vec<u8>, LocalRemoteSyncError> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for file in &snapshot.files {
        if !is_safe_relative_archive_path(&file.relative_path) {
            return Err(LocalRemoteSyncError::UnsafeSnapshotPath {
                id: snapshot.id.clone(),
                path: file.relative_path.clone(),
            });
        }
        let mut header = tar::Header::new_gnu();
        header.set_size(file.bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                file.relative_path.as_str(),
                file.bytes.as_slice(),
            )
            .map_err(|error| {
                LocalRemoteSyncError::io(
                    format!("Failed to append archive entry '{}'", file.relative_path),
                    error,
                )
            })?;
    }
    let encoder = builder
        .into_inner()
        .map_err(|error| LocalRemoteSyncError::io("Failed to finalize archive", error))?;
    encoder
        .finish()
        .map_err(|error| LocalRemoteSyncError::io("Failed to compress archive", error))
}

async fn remote_snapshot_hash(
    connection: &ConnectedRemoteTarget,
    remote_root: &str,
    snapshot: &LocalSnapshot,
) -> Result<Option<String>, LocalRemoteSyncError> {
    let manifest = snapshot
        .files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let output = connection
        .run_command_with_stdin_bytes(&remote_hash_command(remote_root), manifest.as_bytes())
        .await
        .map_err(|e| LocalRemoteSyncError::Remote(e.to_string()))?;
    let output = String::from_utf8_lossy(&output);
    parse_remote_hash_output(&output)
}

fn remote_hash_command(remote_root: &str) -> String {
    format!(
        "sh -c {} -- {}",
        shell_quote(REMOTE_SNAPSHOT_HASH_SCRIPT),
        shell_quote(remote_root)
    )
}

pub fn parse_remote_hash_output(output: &str) -> Result<Option<String>, LocalRemoteSyncError> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let line = line.trim_end();
        if line == "MISSING" {
            return Ok(None);
        }
        if let Some(path) = line.strip_prefix("MISSING_FILE\t") {
            if !is_safe_relative_archive_path(path) {
                return Err(LocalRemoteSyncError::UnsafeRemoteHashPath(path.to_string()));
            }
            entries.push((path.to_string(), "missing".to_string()));
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some((digest, path)) = line.split_once('\t') else {
            return Err(LocalRemoteSyncError::UnsupportedRemoteHashLine(
                line.to_string(),
            ));
        };
        let digest = digest.trim();
        let path = path.trim();
        if digest.len() != 64 || !digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(LocalRemoteSyncError::UnsupportedRemoteHashDigest(
                path.to_string(),
            ));
        }
        if !is_safe_relative_archive_path(path) {
            return Err(LocalRemoteSyncError::UnsafeRemoteHashPath(path.to_string()));
        }
        entries.push((path.to_string(), digest.to_ascii_lowercase()));
    }
    Ok(Some(hash_entries(entries)))
}

async fn apply_snapshot(
    connection: &ConnectedRemoteTarget,
    snapshot: &LocalSnapshot,
    remote_path: &str,
) -> Result<(), LocalRemoteSyncError> {
    let parent = remote_parent(remote_path)
        .ok_or_else(|| LocalRemoteSyncError::RemotePathNoParent(remote_path.to_string()))?;
    let archive = build_archive(snapshot)?;
    let safe_id = repo_slug(Path::new(&snapshot.id));
    let staging = format!(
        "{}/.skillport-sync-{}-{}",
        parent.trim_end_matches('/'),
        safe_id,
        Uuid::new_v4()
    );
    let backup = format!(
        "{}/.skillport-sync-backup-{}-{}",
        parent.trim_end_matches('/'),
        safe_id,
        Uuid::new_v4()
    );
    connection
        .run_command_with_stdin_bytes(
            &remote_apply_command(remote_path, &parent, &staging, &backup),
            &archive,
        )
        .await
        .map(|_| ())
        .map_err(|error| LocalRemoteSyncError::RemoteApply {
            path: remote_path.to_string(),
            message: error.to_string(),
        })
}

fn remote_apply_command(
    target_dir: &str,
    parent_dir: &str,
    staging_dir: &str,
    backup_dir: &str,
) -> String {
    format!(
        "sh -c {} -- {} {} {} {}",
        shell_quote(REMOTE_APPLY_ARCHIVE_SCRIPT),
        shell_quote(target_dir),
        shell_quote(parent_dir),
        shell_quote(staging_dir),
        shell_quote(backup_dir)
    )
}

fn path_display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
