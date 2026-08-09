use super::tree_import::{try_prepare_tree_import, TreeImportOutcome, TreeSelectionScope};
use super::*;
use crate::services::resource_budget::ResourceBudget;

/// Remote repository inventory: one NUL-delimited record per regular file with
/// its repository-relative path, byte length, and SHA-256.
pub(super) const REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT: &str = r#"set -eu
repo_dir=$1
if command -v sha256sum >/dev/null 2>&1; then
  hash_tool=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  hash_tool=shasum
elif command -v openssl >/dev/null 2>&1; then
  hash_tool=openssl
else
  printf 'Missing required remote tool: sha256sum\n' >&2
  exit 127
fi
find "$repo_dir" -type f -exec sh -c '
repo_dir=$1
hash_tool=$2
shift 2
for file do
  relative=${file#"$repo_dir"/}
  size=$(wc -c < "$file")
  case "$hash_tool" in
    sha256sum) digest=$(sha256sum "$file" | cut -d" " -f1) ;;
    shasum) digest=$(shasum -a 256 "$file" | cut -d" " -f1) ;;
    *) digest=$(openssl dgst -sha256 "$file" | sed "s/.*= //") ;;
  esac
  printf "%s\0%s\0%s\0" "$relative" "$size" "$digest"
done
' sh "$repo_dir" "$hash_tool" {} +
"#;

pub(super) fn snapshot_preview_repository_files(
    snapshot: &GitHubRepoSnapshot,
) -> Vec<PreviewSnapshotFile> {
    snapshot_files_from_local(snapshot)
}

pub(super) fn parse_remote_preview_repository_files(
    output: &str,
) -> Result<Vec<PreviewSnapshotFile>, GithubImportError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }

    let payload = output
        .strip_suffix('\0')
        .ok_or(GithubImportError::RemotePreviewInvalidFileManifest)?;
    let fields = payload.split('\0').collect::<Vec<_>>();
    if fields.len() % 3 != 0 {
        return Err(GithubImportError::RemotePreviewInvalidFileManifest);
    }

    let budget = ResourceBudget::default_skill();
    let mut total_bytes = 0_u64;
    let mut seen_paths = HashSet::new();
    let mut files = Vec::with_capacity(fields.len() / 3);
    for record in fields.chunks_exact(3) {
        if files.len() >= budget.archive_files {
            return Err(GithubImportError::ArchiveFileBudgetExceeded(
                budget.archive_files,
            ));
        }

        let repo_path = normalize_repo_path(record[0])?;
        if repo_path.is_empty() || !seen_paths.insert(repo_path.clone()) {
            return Err(GithubImportError::RemotePreviewInvalidFileManifest);
        }
        let byte_len = record[1]
            .trim()
            .parse::<u64>()
            .map_err(|_| GithubImportError::RemotePreviewInvalidFileManifest)?;
        let sha256 = parse_remote_file_sha256(record[2])?;
        budget
            .reject_archive_entry_size(&repo_path, byte_len)
            .map_err(GithubImportError::Budget)?;
        total_bytes = total_bytes
            .checked_add(byte_len)
            .ok_or(GithubImportError::ArchiveSizeOverflow)?;
        budget
            .reject_archive_expanded_size(total_bytes)
            .map_err(GithubImportError::Budget)?;
        files.push(PreviewSnapshotFile {
            repo_path,
            byte_len,
            sha256,
        });
    }
    files.sort_by(|left, right| left.repo_path.as_bytes().cmp(right.repo_path.as_bytes()));
    Ok(files)
}

fn parse_remote_file_sha256(raw: &str) -> Result<[u8; 32], GithubImportError> {
    let trimmed = raw.trim();
    if trimmed.len() != 64 || !trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(GithubImportError::RemotePreviewInvalidFileManifest);
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in trimmed.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| GithubImportError::RemotePreviewInvalidFileManifest)?;
        digest[index] = u8::from_str_radix(text, 16)
            .map_err(|_| GithubImportError::RemotePreviewInvalidFileManifest)?;
    }
    Ok(digest)
}

pub(super) async fn remote_preview_repository_files(
    connection: &ConnectedRemoteTarget,
    remote_repo_dir: &str,
) -> Result<Vec<PreviewSnapshotFile>, GithubImportError> {
    let output = connection
        .run_script(REMOTE_PREVIEW_FILE_INVENTORY_SCRIPT, &[remote_repo_dir])
        .await
        .map_err(|error| GithubImportError::Remote(error.to_string()))?;
    parse_remote_preview_repository_files(&output)
}

/// Attach the per-candidate file manifest (path + byte length + SHA-256) and
/// return the candidate content digests recorded in the registered snapshot.
pub(super) fn attach_preview_file_manifests(
    skills: &mut [GitHubSkillPreview],
    repository_files: &[PreviewSnapshotFile],
) -> Result<Vec<PreviewSnapshotCandidate>, GithubImportError> {
    let mut candidates = Vec::with_capacity(skills.len());
    for skill in skills {
        let mut files = repository_files
            .iter()
            .filter_map(|file| {
                repo_file_relative_to_source(&file.repo_path, &skill.source_path).map(|path| {
                    GitHubSkillPreviewFile {
                        path,
                        byte_len: file.byte_len,
                        sha256: encode_file_digest(&file.sha256),
                    }
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
        if !files.iter().any(|file| file.path == "SKILL.md") {
            return Err(GithubImportError::PreviewFileManifestIncomplete(
                skill.source_path.clone(),
            ));
        }
        candidates.push(PreviewSnapshotCandidate {
            source_path: skill.source_path.clone(),
            content_digest: candidate_content_digest(&files)?,
        });
        skill.files = Some(files);
    }
    Ok(candidates)
}

#[allow(dead_code)]
pub(crate) async fn preview_github_repo_import_impl(
    pool: &DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    repo_url: &str,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let auth = github_direct_auth_from_secret_store(pool, secrets).await?;
    preview_github_repo_import_with_auth(pool, repo_url, auth.as_deref()).await
}

pub(super) struct TreeCandidateInspection {
    pub(super) inspected: InspectedGitHubRepoSkills,
    pub(super) fetched_files: HashMap<String, Vec<u8>>,
}

pub(super) async fn inspect_tree_candidates_from_manifest(
    client: &reqwest::Client,
    manifest: &tree_manifest::RepositoryManifest,
    pinned_repo: &GitHubRepoRef,
    display_repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth: Option<&str>,
) -> Result<TreeCandidateInspection, GithubImportError> {
    let (plugin_discovery, mut fetched_files) =
        build_tree_plugin_discovery(client, manifest, pinned_repo, source_path, auth).await?;
    let manifests = discover_skill_manifests_from_paths_with_plugin_discovery(
        manifest.regular_paths(),
        source_path,
        &plugin_discovery,
    )?;
    let direct_endpoint = GITHUB_MIRROR_ENDPOINTS.first().expect("github endpoint");
    let mut valid_candidates = Vec::with_capacity(manifests.len());
    let mut invalid_candidates = Vec::new();
    let mut seen_names = HashSet::new();

    for skill_manifest in manifests {
        let raw = if let Some(bytes) = fetched_files.get(&skill_manifest.skill_md_path) {
            bytes.clone()
        } else {
            let bytes =
                fetch_raw_bytes(client, pinned_repo, &skill_manifest.skill_md_path, auth).await?;
            fetched_files.insert(skill_manifest.skill_md_path.clone(), bytes.clone());
            bytes
        };
        ResourceBudget::default_skill()
            .reject_file_read_size(&skill_manifest.skill_md_path, raw.len() as u64)
            .map_err(GithubImportError::Budget)?;

        match build_remote_skill_candidate(display_repo, &skill_manifest, raw, direct_endpoint) {
            Ok(candidate) => {
                if is_generic_remote_skill_candidate(&candidate) {
                    continue;
                }
                if seen_names.insert(candidate.skill_name.clone()) {
                    valid_candidates.push(candidate);
                }
            }
            Err(invalid) if skill_manifest.from_manifest_hint => {
                let _ = invalid;
            }
            Err(invalid) => invalid_candidates.push(invalid_candidate_from_manifest(
                &skill_manifest,
                &invalid.detail,
            )),
        }
    }

    Ok(TreeCandidateInspection {
        inspected: InspectedGitHubRepoSkills {
            repo: display_repo.clone(),
            valid_candidates,
            invalid_candidates,
        },
        fetched_files,
    })
}

/// Resolve plugin manifest discovery from raw bytes fetched through the tree
/// fast-path. Only fetches `.claude-plugin/plugin.json` and `marketplace.json`
/// when the tree manifest lists them; missing paths mean the repo has no
/// manifest (continue with no grouping, matching the archive path).
async fn build_tree_plugin_discovery(
    client: &reqwest::Client,
    manifest: &tree_manifest::RepositoryManifest,
    repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth: Option<&str>,
) -> Result<(PluginManifestDiscovery, HashMap<String, Vec<u8>>), GithubImportError> {
    let base_path = effective_source_root(source_path)?;
    let plugin_json_path = join_repo_path(&base_path, ".claude-plugin/plugin.json")?;
    let marketplace_json_path = join_repo_path(&base_path, ".claude-plugin/marketplace.json")?;

    let plugin_json =
        fetch_optional_manifest_bytes(client, manifest, &plugin_json_path, repo, auth).await?;
    let marketplace_json =
        fetch_optional_manifest_bytes(client, manifest, &marketplace_json_path, repo, auth).await?;

    let discovery = plugin_manifest_discovery_from_manifest_bytes(
        &base_path,
        plugin_json.as_deref(),
        marketplace_json.as_deref(),
    );
    let mut fetched_files = HashMap::new();
    if let Some(bytes) = plugin_json {
        fetched_files.insert(plugin_json_path, bytes);
    }
    if let Some(bytes) = marketplace_json {
        fetched_files.insert(marketplace_json_path, bytes);
    }
    Ok((discovery, fetched_files))
}

/// Fetch an optional plugin manifest file's raw bytes. Returns `None` when the
/// tree manifest does not list the path (the repo has no such manifest). Any
/// raw fetch failure (404 integrity gap, denial, transport, budget) propagates
/// so the dispatcher falls back to archive acquisition, preserving parity
/// with the archive path (which would read the manifest from the tarball).
async fn fetch_optional_manifest_bytes(
    client: &reqwest::Client,
    manifest: &tree_manifest::RepositoryManifest,
    path: &str,
    repo: &GitHubRepoRef,
    auth: Option<&str>,
) -> Result<Option<Vec<u8>>, GithubImportError> {
    let exists = manifest.regular_paths().any(|repo_path| repo_path == path);
    if !exists {
        return Ok(None);
    }
    let bytes = fetch_raw_bytes(client, repo, path, auth).await?;
    Ok(Some(bytes))
}

/// Acquire a complete, retained repository snapshot at a pinned commit.
///
/// The tree fast-path downloads only the discovered candidate subtrees; the
/// archive fallback retains the whole bounded tarball snapshot. Either way the
/// returned snapshot owns every byte the preview will display and the import
/// will write, so import never re-resolves the branch.
async fn acquire_pinned_preview_snapshot(
    client: &reqwest::Client,
    pinned_repo: &GitHubRepoRef,
    display_repo: &GitHubRepoRef,
    source_path: Option<&str>,
    auth: Option<&str>,
) -> Result<(GitHubRepoSnapshot, Vec<RemoteSkillCandidate>), GithubImportError> {
    match try_prepare_tree_import(
        client,
        pinned_repo,
        display_repo,
        source_path,
        TreeSelectionScope::AllCandidates,
        auth,
        false,
    )
    .await?
    {
        TreeImportOutcome::Ready {
            snapshot,
            inspected,
        } => Ok((snapshot, inspected.valid_candidates)),
        TreeImportOutcome::Fallback(_) => {
            let snapshot = download_repo_snapshot(client, pinned_repo, auth).await?;
            let candidates = build_repo_skill_candidates_from_snapshot_at_path(
                display_repo,
                &snapshot,
                source_path,
            )?;
            Ok((snapshot, candidates))
        }
    }
}

pub(crate) async fn acquire_pinned_repo_snapshot(
    resolved: ResolvedGitHubRepoSource,
    auth: Option<&str>,
) -> Result<PinnedGitHubRepoSnapshot, GithubImportError> {
    let client = github_client()?;
    let resolved_commit_sha = resolve_commit_sha(&client, &resolved.repo, auth).await?;
    let pinned_repo = pinned_repo_ref(&resolved.repo, &resolved_commit_sha);
    let (snapshot, candidates) = acquire_pinned_preview_snapshot(
        &client,
        &pinned_repo,
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth,
    )
    .await?;
    Ok(PinnedGitHubRepoSnapshot {
        resolved,
        resolved_commit_sha,
        snapshot,
        candidates,
    })
}

pub(crate) async fn preview_github_repo_import_with_auth(
    pool: &DbPool,
    repo_url: &str,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    preview_github_repo_import_with_branch_and_auth(pool, repo_url, None, auth).await
}

pub(crate) async fn preview_github_repo_import_with_branch_and_auth(
    pool: &DbPool,
    repo_url: &str,
    branch: Option<&str>,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source_with_branch(repo_url, branch, auth).await?;
    let client = github_client()?;
    let resolved_commit_sha = resolve_commit_sha(&client, &resolved.repo, auth).await?;
    let pinned_repo = pinned_repo_ref(&resolved.repo, &resolved_commit_sha);

    let (snapshot, candidates) = acquire_pinned_preview_snapshot(
        &client,
        &pinned_repo,
        &resolved.repo,
        resolved.source_path.as_deref(),
        auth,
    )
    .await?;

    let mut skills = build_preview_skills(pool, &candidates).await?;
    if skills.is_empty() {
        return Err(GithubImportError::NoImportableSkills);
    }
    let repository_files = snapshot_preview_repository_files(&snapshot);
    let snapshot_candidates = attach_preview_file_manifests(&mut skills, &repository_files)?;

    let now = Utc::now();
    let snapshot = PreviewSnapshot {
        id: new_preview_id(),
        target_id: ActiveTarget::Local.id().to_string(),
        target_kind: TargetKind::Local,
        repo: resolved.repo.clone(),
        source_path: resolved.source_path.clone(),
        resolved_commit_sha: resolved_commit_sha.clone(),
        snapshot_digest: repository_snapshot_digest(&repository_files),
        files: repository_files,
        candidates: snapshot_candidates,
        created_at: now,
        expires_at: now + Duration::minutes(REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES),
        storage: PreviewSnapshotStorage::Local(Arc::new(snapshot)),
    };
    let preview = preview_dto_from_snapshot(&snapshot, skills);
    register_preview_snapshot(snapshot)?;
    Ok(preview)
}

pub(crate) async fn preview_github_repo_import_remote_with_auth(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repo_url: &str,
    branch: Option<&str>,
    auth: Option<&str>,
) -> Result<GitHubRepoPreview, GithubImportError> {
    let resolved = resolve_repo_source_with_branch(repo_url, branch, auth).await?;
    let client = github_client()?;
    let resolved_commit_sha = resolve_commit_sha(&client, &resolved.repo, auth).await?;
    let pinned_repo = pinned_repo_ref(&resolved.repo, &resolved_commit_sha);

    let connection = connect_remote_target(active_target)
        .await
        .map_err(|e| GithubImportError::Remote(e.to_string()))?;
    cleanup_expired_preview_snapshots_for_connection(&connection).await;
    let mut reservation = loop {
        match reserve_remote_preview_snapshot(active_target.id(), active_target.kind(), Utc::now())?
        {
            RemoteReservationAttempt::Reserved(reservation) => break reservation,
            RemoteReservationAttempt::CleanupRequired(tickets) => {
                if !cleanup_preview_tickets_for_connection(&connection, tickets).await {
                    return Err(GithubImportError::PreviewCleanupPending);
                }
            }
            RemoteReservationAttempt::Capacity => return Err(GithubImportError::PreviewCapacity),
        }
    };

    let workspace = create_remote_preview_workspace(&connection, &pinned_repo, auth).await?;
    reservation.claim_workspace(&workspace)?;
    let preview_result = async {
        let candidates = build_remote_repo_skill_candidates_from_workspace(
            &connection,
            &resolved.repo,
            &workspace.remote_repo_dir,
            resolved.source_path.as_deref(),
        )
        .await?;
        let mut skills = build_preview_skills(pool, &candidates).await?;
        if skills.is_empty() {
            return Err(GithubImportError::NoImportableSkills);
        }
        let repository_files =
            remote_preview_repository_files(&connection, &workspace.remote_repo_dir).await?;
        let snapshot_candidates = attach_preview_file_manifests(&mut skills, &repository_files)?;
        Ok((skills, repository_files, snapshot_candidates))
    }
    .await;

    match preview_result {
        Ok((skills, repository_files, snapshot_candidates)) => {
            let now = Utc::now();
            let snapshot = PreviewSnapshot {
                id: reservation.preview_id().to_string(),
                target_id: active_target.id().to_string(),
                target_kind: active_target.kind(),
                repo: resolved.repo.clone(),
                source_path: resolved.source_path.clone(),
                resolved_commit_sha,
                snapshot_digest: repository_snapshot_digest(&repository_files),
                files: repository_files,
                candidates: snapshot_candidates,
                created_at: now,
                expires_at: now + Duration::minutes(REMOTE_PREVIEW_WORKSPACE_TTL_MINUTES),
                storage: PreviewSnapshotStorage::Remote(workspace.clone()),
            };
            let preview = preview_dto_from_snapshot(&snapshot, skills);
            if let Err(error) = reservation.fill(snapshot) {
                if connection
                    .remove_tree(&workspace.remote_workspace_dir)
                    .await
                    .is_err()
                {
                    let _ = reservation.retain_cleanup_pending(workspace);
                    return Err(GithubImportError::PreviewCleanupPending);
                }
                reservation.release_after_cleanup();
                return Err(error);
            }
            Ok(preview)
        }
        Err(error) => {
            if connection
                .remove_tree(&workspace.remote_workspace_dir)
                .await
                .is_err()
            {
                let _ = reservation.retain_cleanup_pending(workspace);
                return Err(GithubImportError::PreviewCleanupPending);
            }
            reservation.release_after_cleanup();
            Err(error)
        }
    }
}

pub(super) fn new_preview_id() -> String {
    format!("github-preview-{}", Uuid::new_v4())
}

pub(super) fn preview_dto_from_snapshot(
    snapshot: &PreviewSnapshot,
    skills: Vec<GitHubSkillPreview>,
) -> GitHubRepoPreview {
    GitHubRepoPreview {
        repo: snapshot.repo.clone(),
        skills,
        preview_id: snapshot.id.clone(),
        resolved_commit_sha: snapshot.resolved_commit_sha.clone(),
        snapshot_digest: snapshot.snapshot_digest.clone(),
        expires_at: snapshot.expires_at.to_rfc3339(),
    }
}
