//! Skills CLI update-center tests. GitHub is always a fake; HOME is a temp dir.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

use crate::services::github_import::GitHubRepoSnapshot;
use crate::services::skills_cli::SkillsCliError;
use crate::test_support::mem_pool;

use super::apply::{
    apply_updates_at, retry_update_recovery_at, set_apply_fault, ApplyContext, ApplyFault,
};
use super::capability::{
    apply_argv_preview, argv_contains_forbidden_flags, update_capability_plan, CapabilitySupport,
};
use super::detect::{check_updates_at, load_update_inventory, verify_update_baseline_at};
use super::digest::digest_skill_directory;
use super::github::{FakeSkillsCliGithub, GithubObserveResult};
use super::source::parse_github_update_identity;
use super::status::classify_successful_check;
use super::{
    NoopProgress, SkillsCliApplySelection, SkillsCliApplyUpdateRequest, SkillsCliUpdateStatus,
};

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn lock_json(name: &str, source: &str) -> String {
    format!(
        r#"{{"version":3,"skills":{{"{name}":{{"sourceUrl":"{source}","sourceType":"github"}}}}}}"#
    )
}

fn write_skill(dir: &std::path::Path, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("SKILL.md"), body).unwrap();
}

fn snapshot_with(path: &str, body: &str) -> GitHubRepoSnapshot {
    let mut files = HashMap::new();
    files.insert(path.to_string(), body.as_bytes().to_vec());
    GitHubRepoSnapshot { files }
}

struct Harness {
    _temp: TempDir,
    pool: crate::db::DbPool,
    canonical: PathBuf,
    lock: PathBuf,
    recovery: PathBuf,
    mutation: PathBuf,
}

impl Harness {
    async fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let canonical = temp.path().join("universal");
        fs::create_dir_all(&canonical).unwrap();
        let lock = temp.path().join(".skill-lock.json");
        let recovery = temp.path().join("update-recovery");
        fs::create_dir_all(&recovery).unwrap();
        let mutation = temp.path().join("mutation.lock");
        let pool = mem_pool().await;
        Self {
            _temp: temp,
            pool,
            canonical,
            lock,
            recovery,
            mutation,
        }
    }
}

#[test]
fn capability_plan_is_fail_closed() {
    let plan = update_capability_plan();
    assert_eq!(plan.force_flag, CapabilitySupport::VerifiedUnsupported);
    assert_eq!(plan.keep_links_flag, CapabilitySupport::VerifiedUnsupported);
    assert_eq!(plan.pinned_full_sha_source, CapabilitySupport::Unverified);
    assert_eq!(plan.direct_copy_refresh, CapabilitySupport::Unverified);
    let preview = apply_argv_preview(&["demo".to_string()]);
    assert!(!argv_contains_forbidden_flags(&preview));
    assert!(!preview
        .iter()
        .any(|item| item == "--force" || item == "--keep-links"));
    assert!(!preview.iter().any(|item| item.contains('@')));
}

#[test]
fn update_check_failed_is_retryable() {
    assert!(SkillsCliError::UpdateCheckFailed.retryable());
    assert!(!SkillsCliError::UpdateBaselineRequired.retryable());
    assert!(!SkillsCliError::UpdateUnsupported.retryable());
}

#[test]
fn source_groups_owner_repo_branch() {
    let identity = parse_github_update_identity("https://github.com/Owner/Repo", None).unwrap();
    assert_eq!(identity.repository_key, "owner/repo@main");
    assert!(!identity.normalized_source.is_empty());
    let shorthand = parse_github_update_identity("owner/repo@demo", None).unwrap();
    assert_eq!(shorthand.skill_path, "demo");
    assert!(parse_github_update_identity("C:\\tmp\\skill", None).is_err());
}

#[test]
fn classify_new_install_is_baseline_required() {
    let result = classify_successful_check(
        false,
        None,
        None,
        None,
        Some("sha256-v1:abc"),
        SHA_A,
        "sha256-v1:abc",
    );
    assert_eq!(result.status.as_str(), "baseline_required");
}

#[test]
fn digest_ignores_mtime_and_sees_content() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("demo");
    write_skill(&root, "hello");
    let first = digest_skill_directory(&root).unwrap();
    {
        let file = fs::OpenOptions::new()
            .write(true)
            .open(root.join("SKILL.md"))
            .unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(3600))
            .unwrap();
    }
    let same = digest_skill_directory(&root).unwrap();
    assert_eq!(first, same);
    write_skill(&root, "changed");
    let second = digest_skill_directory(&root).unwrap();
    assert_ne!(first, second);
}

#[test]
fn digest_skips_operation_marker_dir() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("demo");
    write_skill(&root, "hello");
    let marker = root.join(".skillport-update-op-1");
    fs::create_dir_all(&marker).unwrap();
    fs::write(marker.join("backup.txt"), "nope").unwrap();
    let with_marker = digest_skill_directory(&root).unwrap();
    fs::remove_dir_all(&marker).unwrap();
    let without = digest_skill_directory(&root).unwrap();
    assert_eq!(with_marker, without);
}

#[cfg(unix)]
#[test]
fn digest_rejects_symlink() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("demo");
    write_skill(&root, "hello");
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "secret").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
    assert!(matches!(
        digest_skill_directory(&root),
        Err(SkillsCliError::UpdateIntegrity)
    ));
}

#[tokio::test]
async fn grouped_check_calls_github_once_per_repo() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("one"), "one");
    write_skill(&harness.canonical.join("two"), "two");
    fs::write(
        &harness.lock,
        r#"{"version":3,"skills":{"one":{"sourceUrl":"https://github.com/owner/repo","sourceType":"github"},"two":{"sourceUrl":"https://github.com/owner/repo","sourceType":"github"}}}"#,
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    let mut files = HashMap::new();
    files.insert("one/SKILL.md".into(), b"one".to_vec());
    files.insert("two/SKILL.md".into(), b"two-new".to_vec());
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: GitHubRepoSnapshot { files },
            etag: Some("etag-1".into()),
            rate_limit_remaining: Some(10),
            rate_limit_reset_at: None,
        },
    );
    let inventory = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-1",
        None,
    )
    .await
    .unwrap();
    assert_eq!(github.call_keys(), vec!["owner/repo@main".to_string()]);
    assert_eq!(inventory.repositories.len(), 1);
    assert!(inventory.skills.iter().any(
        |row| row.skill_name == "one" && row.status == SkillsCliUpdateStatus::BaselineRequired
    ));
    assert!(inventory.capability.force_flag == CapabilitySupport::VerifiedUnsupported);
}

#[tokio::test]
async fn rate_limit_skips_remaining_repos() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("a"), "a");
    write_skill(&harness.canonical.join("b"), "b");
    fs::write(
        &harness.lock,
        r#"{"version":3,"skills":{"a":{"sourceUrl":"https://github.com/owner/one","sourceType":"github"},"b":{"sourceUrl":"https://github.com/owner/two","sourceType":"github"}}}"#,
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    github.set_rate_limited("owner/one@main", None);
    github.set_result(
        "owner/two@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("b/SKILL.md", "b"),
            etag: None,
            rate_limit_remaining: Some(10),
            rate_limit_reset_at: None,
        },
    );
    let inventory = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-2",
        None,
    )
    .await
    .unwrap();
    assert!(github.call_keys().len() <= 2);
    assert_eq!(github.call_keys(), vec!["owner/one@main".to_string()]);
    assert!(inventory
        .skills
        .iter()
        .any(|row| { row.status == SkillsCliUpdateStatus::RateLimited && row.is_stale }));
}

#[tokio::test]
async fn verify_writes_baseline_only_on_exact_match() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("demo"), "same");
    fs::write(
        &harness.lock,
        lock_json("demo", "https://github.com/owner/repo"),
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "same"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-3",
        None,
    )
    .await
    .unwrap();
    let after_mismatch_prep = {
        github.set_result(
            "owner/repo@main",
            GithubObserveResult {
                revision_sha: SHA_B.to_string(),
                snapshot: snapshot_with("demo/SKILL.md", "other"),
                etag: None,
                rate_limit_remaining: Some(5),
                rate_limit_reset_at: None,
            },
        );
        check_updates_at(
            &harness.pool,
            &harness.canonical,
            &harness.lock,
            &github,
            &NoopProgress,
            "job-3b",
            None,
        )
        .await
        .unwrap()
    };
    assert!(after_mismatch_prep
        .skills
        .iter()
        .any(|row| row.status == SkillsCliUpdateStatus::BaselineRequired));
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "same"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-3c",
        None,
    )
    .await
    .unwrap();
    let verified = verify_update_baseline_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &["demo".to_string()],
        None,
    )
    .await
    .unwrap();
    assert!(verified
        .skills
        .iter()
        .any(|row| row.skill_name == "demo" && row.status == SkillsCliUpdateStatus::Current));
}

#[tokio::test]
async fn apply_refreshes_canonical_without_forbidden_flags() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("demo"), "old");
    fs::write(
        &harness.lock,
        lock_json("demo", "https://github.com/owner/repo"),
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "old"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-4",
        None,
    )
    .await
    .unwrap();
    verify_update_baseline_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &["demo".to_string()],
        None,
    )
    .await
    .unwrap();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_B.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "new"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    let inventory = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-4b",
        None,
    )
    .await
    .unwrap();
    let row = inventory
        .skills
        .iter()
        .find(|row| row.skill_name == "demo")
        .unwrap();
    assert_eq!(row.status, SkillsCliUpdateStatus::UpdateAvailable);
    github.set_sha_snapshot(SHA_B, snapshot_with("demo/SKILL.md", "new"));
    let pending_digest = row.pending_upstream_digest.clone().unwrap();
    let result = apply_updates_at(ApplyContext {
        pool: &harness.pool,
        canonical_root: &harness.canonical,
        lock_path: &harness.lock,
        recovery_root: &harness.recovery,
        github: &github,
        progress: &NoopProgress,
        request: &SkillsCliApplyUpdateRequest {
            job_id: "job-apply".into(),
            repository_key: "owner/repo@main".into(),
            selections: vec![SkillsCliApplySelection {
                skill_name: "demo".into(),
                skill_path: "demo".into(),
                expected_installed_revision: row.installed_revision_sha.clone(),
                expected_installed_local_digest: row.installed_local_digest.clone(),
                expected_pending_revision: SHA_B.to_string(),
                expected_pending_digest: pending_digest,
            }],
        },
        cancel: None,
        mutation_lock_path: Some(harness.mutation.clone()),
    })
    .await
    .unwrap();
    assert_eq!(result.applied_skill_names, vec!["demo".to_string()]);
    let body = fs::read_to_string(harness.canonical.join("demo/SKILL.md")).unwrap();
    assert_eq!(body, "new");
    let after = load_update_inventory(&harness.pool).await.unwrap();
    let applied = after
        .skills
        .iter()
        .find(|row| row.skill_name == "demo")
        .unwrap();
    assert_eq!(applied.status, SkillsCliUpdateStatus::Current);
    assert_eq!(applied.installed_revision_sha.as_deref(), Some(SHA_B));
    assert!(applied.pending_revision_sha.is_none());
    assert!(!github
        .call_keys()
        .iter()
        .any(|item| item.contains("--force") || item.contains("--keep-links")));
}

#[tokio::test]
async fn apply_stale_is_zero_write() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("demo"), "old");
    fs::write(
        &harness.lock,
        lock_json("demo", "https://github.com/owner/repo"),
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    let err = apply_updates_at(ApplyContext {
        pool: &harness.pool,
        canonical_root: &harness.canonical,
        lock_path: &harness.lock,
        recovery_root: &harness.recovery,
        github: &github,
        progress: &NoopProgress,
        request: &SkillsCliApplyUpdateRequest {
            job_id: "job-stale".into(),
            repository_key: "owner/repo@main".into(),
            selections: vec![SkillsCliApplySelection {
                skill_name: "demo".into(),
                skill_path: "demo".into(),
                expected_installed_revision: None,
                expected_installed_local_digest: None,
                expected_pending_revision: SHA_B.to_string(),
                expected_pending_digest: "sha256-v1:nope".into(),
            }],
        },
        cancel: None,
        mutation_lock_path: Some(harness.mutation.clone()),
    })
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        SkillsCliError::UpdateStale | SkillsCliError::SkillNotOwned
    ));
    assert_eq!(
        fs::read_to_string(harness.canonical.join("demo/SKILL.md")).unwrap(),
        "old"
    );
    assert!(github.call_keys().is_empty());
    let inventory = load_update_inventory(&harness.pool).await.unwrap();
    assert!(inventory.pending_recovery.is_none());
}

#[tokio::test]
async fn apply_fault_after_prepared_is_recoverable() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("demo"), "old");
    fs::write(
        &harness.lock,
        lock_json("demo", "https://github.com/owner/repo"),
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "old"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-5",
        None,
    )
    .await
    .unwrap();
    verify_update_baseline_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &["demo".to_string()],
        None,
    )
    .await
    .unwrap();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_B.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "new"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    let inventory = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-5b",
        None,
    )
    .await
    .unwrap();
    let row = inventory
        .skills
        .iter()
        .find(|row| row.skill_name == "demo")
        .unwrap();
    github.set_sha_snapshot(SHA_B, snapshot_with("demo/SKILL.md", "new"));
    set_apply_fault(Some(ApplyFault::Prepared));
    let err = apply_updates_at(ApplyContext {
        pool: &harness.pool,
        canonical_root: &harness.canonical,
        lock_path: &harness.lock,
        recovery_root: &harness.recovery,
        github: &github,
        progress: &NoopProgress,
        request: &SkillsCliApplyUpdateRequest {
            job_id: "job-fault".into(),
            repository_key: "owner/repo@main".into(),
            selections: vec![SkillsCliApplySelection {
                skill_name: "demo".into(),
                skill_path: "demo".into(),
                expected_installed_revision: row.installed_revision_sha.clone(),
                expected_installed_local_digest: row.installed_local_digest.clone(),
                expected_pending_revision: SHA_B.to_string(),
                expected_pending_digest: row.pending_upstream_digest.clone().unwrap(),
            }],
        },
        cancel: None,
        mutation_lock_path: Some(harness.mutation.clone()),
    })
    .await
    .unwrap_err();
    set_apply_fault(None);
    assert!(matches!(err, SkillsCliError::UpdateRecoveryRequired));
    assert_eq!(
        fs::read_to_string(harness.canonical.join("demo/SKILL.md")).unwrap(),
        "old"
    );
    let pending = load_update_inventory(&harness.pool)
        .await
        .unwrap()
        .pending_recovery
        .expect("prepared journal");
    let recovered = retry_update_recovery_at(
        &harness.pool,
        &pending.operation_id,
        &harness.canonical,
        &harness.lock,
        &harness.recovery,
        harness.mutation.clone(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(recovered.phase, "rolled_back");
}

#[tokio::test]
async fn failed_check_keeps_pending_and_never_reports_current() {
    let harness = Harness::new().await;
    write_skill(&harness.canonical.join("demo"), "same");
    fs::write(
        &harness.lock,
        lock_json("demo", "https://github.com/owner/repo"),
    )
    .unwrap();
    let github = FakeSkillsCliGithub::new();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "same"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-pending-a",
        None,
    )
    .await
    .unwrap();
    verify_update_baseline_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &["demo".to_string()],
        None,
    )
    .await
    .unwrap();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_B.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", "new"),
            etag: None,
            rate_limit_remaining: Some(5),
            rate_limit_reset_at: None,
        },
    );
    let available = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-pending-b",
        None,
    )
    .await
    .unwrap();
    let pending = available
        .skills
        .iter()
        .find(|row| row.skill_name == "demo")
        .unwrap();
    assert_eq!(pending.status, SkillsCliUpdateStatus::UpdateAvailable);
    assert_eq!(pending.pending_revision_sha.as_deref(), Some(SHA_B));
    github.set_failed("owner/repo@main");
    let failed = check_updates_at(
        &harness.pool,
        &harness.canonical,
        &harness.lock,
        &github,
        &NoopProgress,
        "job-pending-fail",
        None,
    )
    .await
    .unwrap();
    let row = failed
        .skills
        .iter()
        .find(|row| row.skill_name == "demo")
        .unwrap();
    assert_eq!(row.status, SkillsCliUpdateStatus::Failed);
    assert!(row.is_stale);
    assert_eq!(row.pending_revision_sha.as_deref(), Some(SHA_B));
    assert_eq!(row.installed_revision_sha.as_deref(), Some(SHA_A));
    assert_ne!(row.status, SkillsCliUpdateStatus::Current);
}
