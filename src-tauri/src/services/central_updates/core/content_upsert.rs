use std::path::PathBuf;

use crate::db::repos::skills_repo;
use crate::db::{DbPool, Skill, SkillUpdateState};
use crate::services::github_import::{GitHubRepoRef, GitHubRepoSnapshot, RemoteSkillCandidate};
use crate::targets::ActiveTarget;

use super::batch::{update_skills_batch, SkillUpdatePlan};
use crate::services::central_updates::error::CentralUpdatesError;
use crate::services::central_updates::fs::{
    collect_remote_skill_files, ensure_remote_skill_manifest, hash_remote_files, CentralFs,
};
use crate::services::central_updates::types::{GitHubUpdateSource, RemoteSkillContent};

pub(crate) struct JournaledCentralContentUpsert<'a> {
    pub(crate) skill: Skill,
    pub(crate) repo: GitHubRepoRef,
    pub(crate) candidate: RemoteSkillCandidate,
    pub(crate) snapshot: &'a GitHubRepoSnapshot,
    pub(crate) target_dir: PathBuf,
    pub(crate) resolved_commit_sha: Option<String>,
    pub(crate) content_digest: Option<String>,
}

/// Durably upsert one complete GitHub-backed skill directory into Central.
///
/// Acquisition and candidate validation happen before this boundary. The
/// existing Central update batch then owns target locking, pending-operation
/// recovery, the journaled stage/swap, the DB/provenance transaction, and
/// rollback/finalization for Local, SSH, and WSL targets.
pub(crate) async fn journaled_central_content_upsert(
    pool: &DbPool,
    active_target: &ActiveTarget,
    input: JournaledCentralContentUpsert<'_>,
) -> Result<SkillUpdateState, CentralUpdatesError> {
    let fs = CentralFs::from_active_target(active_target.clone()).await?;
    journaled_central_content_upsert_with_fs(pool, &fs, input).await
}

pub(crate) async fn journaled_central_content_upsert_with_fs(
    pool: &DbPool,
    fs: &CentralFs,
    input: JournaledCentralContentUpsert<'_>,
) -> Result<SkillUpdateState, CentralUpdatesError> {
    let existing = skills_repo::get_skill_by_id(pool, &input.skill.id).await?;
    let local_hash = fs
        .hash_directories(std::slice::from_ref(&input.target_dir))
        .await?
        .remove(&input.target_dir)
        .ok_or_else(|| {
            CentralUpdatesError::Batch(
                "Central content upsert could not observe the target directory.".to_string(),
            )
        })?;
    let plan = content_upsert_plan(input, local_hash, existing)?;
    let skill_id = plan.skill.id.clone();
    let mut outcomes = update_skills_batch(pool, fs, vec![plan], None).await;
    let outcome = outcomes.pop().ok_or_else(|| {
        CentralUpdatesError::Batch(format!(
            "Central content upsert returned no outcome for skill '{skill_id}'."
        ))
    })?;
    outcome.result.map_err(|error| error.into_error())
}

fn content_upsert_plan(
    input: JournaledCentralContentUpsert<'_>,
    local_hash: String,
    existing: Option<Skill>,
) -> Result<SkillUpdatePlan, CentralUpdatesError> {
    let target_skill_id = input
        .target_dir
        .file_name()
        .map(|name| name.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    if target_skill_id != input.skill.id {
        return Err(CentralUpdatesError::Batch(
            "Central content upsert target identity does not match the skill id.".to_string(),
        ));
    }
    let files = collect_remote_skill_files(input.snapshot, &input.candidate.source_path)?;
    ensure_remote_skill_manifest(&files)?;
    let remote_hash = hash_remote_files(input.snapshot, &files)?;
    let existing_central = existing.filter(|skill| skill.is_central);
    let first_upsert = existing_central.is_none();
    let mut skill = input.skill;
    if let Some(existing) = existing_central {
        skill.uid = existing.uid;
    }
    let source_path = input.candidate.source_path.clone();
    let remote = RemoteSkillContent {
        source: GitHubUpdateSource {
            repo: input.repo,
            source_path,
        },
        candidate: input.candidate,
        files,
        remote_hash,
        local_hash,
        target_dir: input.target_dir,
        resolved_commit_sha: input.resolved_commit_sha,
        content_digest: input.content_digest,
    };
    Ok(SkillUpdatePlan {
        skill,
        remote,
        refresh_copies: false,
        first_upsert,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::io::Read;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::targets::{
        CommandRunner, ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget,
        ProcessRequest, RemoteTargetConfig, RunnerError, SshAuthMethod, WslTargetConfig,
    };

    fn repo() -> GitHubRepoRef {
        GitHubRepoRef {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/owner/repo".to_string(),
        }
    }

    fn snapshot() -> GitHubRepoSnapshot {
        GitHubRepoSnapshot {
            files: [
                (
                    "skills/safe-skill/SKILL.md".to_string(),
                    b"---\nname: '../escape'\n---\n".to_vec(),
                ),
                (
                    "skills/safe-skill/references/guide.md".to_string(),
                    b"# guide\n".to_vec(),
                ),
                (
                    "skills/safe-skill/scripts/run.ps1".to_string(),
                    b"Write-Output safe\n".to_vec(),
                ),
                (
                    "skills/safe-skill/assets/data.txt".to_string(),
                    b"asset\n".to_vec(),
                ),
            ]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        }
    }

    fn input<'a>(snapshot: &'a GitHubRepoSnapshot) -> JournaledCentralContentUpsert<'a> {
        let target_dir = PathBuf::from("/home/tester/.skillsmanage/skills/safe-skill");
        JournaledCentralContentUpsert {
            skill: Skill {
                id: "safe-skill".to_string(),
                uid: "uid-safe-skill".to_string(),
                name: "../escape".to_string(),
                description: None,
                file_path: "/home/tester/.skillsmanage/skills/safe-skill/SKILL.md".to_string(),
                canonical_path: Some(target_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("github:owner/repo".to_string()),
                content: None,
                scanned_at: chrono::Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
            repo: repo(),
            candidate: RemoteSkillCandidate {
                source_path: "skills/safe-skill".to_string(),
                skill_id: "safe-skill".to_string(),
                skill_name: "../escape".to_string(),
                description: None,
                plugin_name: None,
                root_directory: "skills".to_string(),
                skill_directory_name: "safe-skill".to_string(),
                download_url: "https://example.invalid/ignored".to_string(),
            },
            snapshot,
            target_dir,
            resolved_commit_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            content_digest: Some("sha256-v1:test".to_string()),
        }
    }

    struct SagaRunner {
        target: String,
        files: Vec<crate::services::central_updates::fs::RemoteSkillFile>,
        staging: Mutex<Option<String>>,
        stage_archive: Mutex<Option<Vec<u8>>>,
        scripts: Mutex<Vec<String>>,
        swapped: AtomicBool,
    }

    impl SagaRunner {
        fn new(
            target: &str,
            files: Vec<crate::services::central_updates::fs::RemoteSkillFile>,
        ) -> Self {
            Self {
                target: target.to_string(),
                files,
                staging: Mutex::new(None),
                stage_archive: Mutex::new(None),
                scripts: Mutex::new(Vec::new()),
                swapped: AtomicBool::new(false),
            }
        }
    }

    #[async_trait::async_trait]
    impl CommandRunner for SagaRunner {
        async fn run(
            &self,
            request: ProcessRequest<'_>,
        ) -> Result<std::process::Output, RunnerError> {
            let args = request
                .command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let Some(stdin) = request.stdin else {
                assert!(
                    args.iter().any(|arg| arg.contains("test -e")),
                    "unexpected no-stdin remote command: {args:?}"
                );
                return Ok(process_output(1, ""));
            };

            if stdin.starts_with(&[0x1f, 0x8b]) {
                let staging = stage_path_from_archive(&stdin);
                *self.staging.lock().unwrap() = Some(staging);
                *self.stage_archive.lock().unwrap() = Some(stdin);
                return Ok(process_output(0, "OK\tsafe-skill\n"));
            }

            let script = String::from_utf8(stdin).expect("remote script is UTF-8");
            self.scripts.lock().unwrap().push(script.clone());
            if script.contains("if command -v sha256sum") {
                let staging = self.staging.lock().unwrap().clone();
                let command = args.join(" ");
                let (root, files) = if staging
                    .as_ref()
                    .is_some_and(|staging| command.contains(staging))
                {
                    (staging.expect("staging path"), self.files.as_slice())
                } else if self.swapped.load(Ordering::SeqCst) {
                    (self.target.clone(), self.files.as_slice())
                } else {
                    (self.target.clone(), &[][..])
                };
                return Ok(process_output(0, &remote_hash_output(&root, files)));
            }
            if script.contains("printf 'SWAPPED\\n'") {
                self.swapped.store(true, Ordering::SeqCst);
                return Ok(process_output(0, "SWAPPED\n"));
            }
            if script.contains("printf 'FINALIZED\\n'") {
                return Ok(process_output(0, "FINALIZED\n"));
            }
            panic!("unexpected remote Saga script: {script}");
        }
    }

    fn process_output(code: i32, stdout: &str) -> std::process::Output {
        std::process::Output {
            status: crate::test_support::exit_status(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    fn stage_path_from_archive(bytes: &[u8]) -> String {
        let decoder = flate2::read::GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries().expect("archive entries") {
            let mut entry = entry.expect("archive entry");
            if entry.path().expect("archive path").to_string_lossy()
                != ".skillport-operation-manifest.tsv"
            {
                continue;
            }
            let mut manifest = String::new();
            entry.read_to_string(&mut manifest).expect("stage manifest");
            return manifest
                .lines()
                .next()
                .and_then(|line| line.split('\t').nth(3))
                .expect("stage path")
                .to_string();
        }
        panic!("stage manifest missing from archive");
    }

    fn fake_remote_filesystems(
        files: Vec<crate::services::central_updates::fs::RemoteSkillFile>,
    ) -> Vec<(Arc<SagaRunner>, CentralFs)> {
        let target = "/home/tester/.skillsmanage/skills/safe-skill";
        let ssh_runner = Arc::new(SagaRunner::new(target, files.clone()));
        let ssh = ConnectedSshTarget::for_tests_with_runner(
            RemoteTargetConfig {
                id: "marketplace-ssh".to_string(),
                label: "Marketplace SSH".to_string(),
                host: "example.invalid".to_string(),
                username: "tester".to_string(),
                port: 22,
                auth_method: SshAuthMethod::Key,
                key_path: "~/.ssh/id_ed25519".to_string(),
                credential_key: None,
                protected_password: None,
                password: None,
                remote_home: "/home/tester".to_string(),
                remote_os: "linux".to_string(),
                symlink_enabled: true,
            },
            ssh_runner.clone(),
        );
        let wsl_runner = Arc::new(SagaRunner::new(target, files));
        let wsl = ConnectedWslTarget::for_tests_with_runner(
            WslTargetConfig {
                id: "marketplace-wsl".to_string(),
                label: "Marketplace WSL".to_string(),
                distribution: "TestDistro".to_string(),
                remote_home: "/home/tester".to_string(),
                remote_os: "linux".to_string(),
                symlink_enabled: true,
            },
            wsl_runner.clone(),
        );
        vec![
            (
                ssh_runner,
                CentralFs::Remote(Arc::new(ConnectedRemoteTarget::Ssh(ssh))),
            ),
            (
                wsl_runner,
                CentralFs::Remote(Arc::new(ConnectedRemoteTarget::Wsl(wsl))),
            ),
        ]
    }

    async fn insert_pending_delete(pool: &DbPool, fs: &CentralFs, skill_id: &str) -> String {
        let operation_id = format!("pending-delete-{}-{skill_id}", fs.target_kind());
        let manifest = crate::services::central_operation::OperationManifest::Delete(
            crate::services::central_operation::DeleteManifest {
                version: crate::services::central_operation::MANIFEST_VERSION,
                operation_id: operation_id.clone(),
                paths: vec![crate::services::central_operation::ManagedPath {
                    original: format!("/home/tester/.skillsmanage/skills/{skill_id}"),
                    backup: format!("/home/tester/.skillsmanage/skills/.{skill_id}-backup"),
                    marker: format!("/home/tester/.skillsmanage/skills/.{skill_id}-marker"),
                    expected_present: true,
                    fingerprint: None,
                }],
            },
        );
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        crate::db::repos::fs_db_operations_repo::insert_fs_db_operation(
            pool,
            crate::db::NewFsDbOperation {
                id: &operation_id,
                batch_id: None,
                target_id: fs.target_id(),
                target_kind: fs.target_kind(),
                operation_kind: "central_delete",
                skill_id,
                manifest_version: crate::services::central_operation::MANIFEST_VERSION,
                manifest_json: &manifest_json,
                old_fingerprint: None,
                new_fingerprint: None,
            },
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE fs_db_operations
             SET updated_at = '2000-01-01T00:00:00Z'
             WHERE id = ?",
        )
        .bind(&operation_id)
        .execute(pool)
        .await
        .unwrap();
        operation_id
    }

    fn remote_hash_output(
        root: &str,
        files: &[crate::services::central_updates::fs::RemoteSkillFile],
    ) -> String {
        let mut output = format!("ROOT\t{root}\n");
        for file in files {
            output.push_str(&format!(
                "{}\t{}\n",
                crate::hashing::encode_lower_hex(Sha256::digest(&file.bytes).as_ref()),
                file.relative_path
            ));
        }
        output.push_str(&format!("END\t{root}\n"));
        output
    }

    fn archive_files(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
        let decoder = flate2::read::GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(decoder);
        let mut files = BTreeMap::new();
        for entry in archive.entries().expect("archive entries") {
            let mut entry = entry.expect("archive entry");
            let path = entry
                .path()
                .expect("archive path")
                .to_string_lossy()
                .replace('\\', "/");
            if !path.starts_with("0000/") {
                continue;
            }
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).expect("archive contents");
            files.insert(path.trim_start_matches("0000/").to_string(), contents);
        }
        files
    }

    #[tokio::test]
    async fn marketplace_content_upsert_completes_full_saga_for_fake_ssh_and_wsl() {
        let snapshot = snapshot();
        let expected = snapshot
            .files
            .iter()
            .filter_map(|(path, bytes)| {
                path.strip_prefix("skills/safe-skill/")
                    .map(|relative| (relative.to_string(), bytes.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let files =
            collect_remote_skill_files(&snapshot, "skills/safe-skill").expect("remote skill files");

        for (runner, fs) in fake_remote_filesystems(files.clone()) {
            let pool = crate::test_support::mem_pool().await;
            let state =
                journaled_central_content_upsert_with_fs(&pool, &fs, input(&snapshot)).await;
            assert!(state.is_ok(), "{state:?}");

            let archive = runner
                .stage_archive
                .lock()
                .unwrap()
                .clone()
                .expect("durable stage archive");
            assert_eq!(archive_files(&archive), expected);
            assert!(runner.swapped.load(Ordering::SeqCst));
            let scripts = runner.scripts.lock().unwrap().join("\n");
            assert!(scripts.contains("printf 'SWAPPED\\n'"));
            assert!(scripts.contains("printf 'FINALIZED\\n'"));
            assert!(!scripts.contains("escape"));

            let stored = crate::db::repos::skills_repo::get_skill_by_id(&pool, "safe-skill")
                .await
                .expect("skill query")
                .expect("skill row");
            assert_eq!(stored.uid, "uid-safe-skill");
            assert_eq!(
                stored.canonical_path.as_deref(),
                Some("/home/tester/.skillsmanage/skills/safe-skill")
            );
            assert_eq!(
                stored.file_path,
                "/home/tester/.skillsmanage/skills/safe-skill/SKILL.md"
            );
            assert!(!stored.file_path.contains('\\'));
            let assignment = crate::db::repos::repositories_repo::get_skill_repository_assignment(
                &pool,
                "safe-skill",
            )
            .await
            .expect("repository assignment");
            assert_eq!(assignment.repository.owner.as_deref(), Some("owner"));
            assert_eq!(assignment.repository.repo.as_deref(), Some("repo"));
            assert_eq!(assignment.repository.branch.as_deref(), Some("main"));
            assert_eq!(assignment.source_path.as_deref(), Some("skills/safe-skill"));
            assert!(!assignment.is_source_unknown);
            let provenance = crate::db::repos::repositories_repo::get_skill_repository_provenance(
                &pool,
                "safe-skill",
            )
            .await
            .expect("provenance query")
            .expect("provenance row");
            assert_eq!(
                provenance.0.as_deref(),
                Some("0123456789abcdef0123456789abcdef01234567")
            );
            assert_eq!(provenance.1.as_deref(), Some("sha256-v1:test"));
            let journal = sqlx::query(
                "SELECT operation_kind, phase, manifest_json
                 FROM fs_db_operations WHERE skill_id = 'safe-skill'",
            )
            .fetch_one(&pool)
            .await
            .expect("journal row");
            use sqlx::Row;
            assert_eq!(journal.get::<String, _>("operation_kind"), "central_update");
            assert_eq!(journal.get::<String, _>("phase"), "completed");
            let manifest: serde_json::Value =
                serde_json::from_str(&journal.get::<String, _>("manifest_json"))
                    .expect("journal manifest");
            assert_eq!(manifest["payload"]["hadTarget"], false);
        }
    }

    #[tokio::test]
    async fn fake_ssh_and_wsl_skip_unrelated_pending_recovery_rows() {
        let snapshot = snapshot();
        let files =
            collect_remote_skill_files(&snapshot, "skills/safe-skill").expect("remote skill files");

        for (runner, fs) in fake_remote_filesystems(files.clone()) {
            let pool = crate::test_support::mem_pool().await;
            let operation_id = insert_pending_delete(&pool, &fs, "unrelated").await;

            let state =
                journaled_central_content_upsert_with_fs(&pool, &fs, input(&snapshot)).await;

            assert!(state.is_ok(), "{state:?}");
            let row =
                crate::db::repos::fs_db_operations_repo::get_fs_db_operation(&pool, &operation_id)
                    .await
                    .unwrap()
                    .unwrap();
            assert_eq!(row.phase, "prepared");
            assert_eq!(row.updated_at, "2000-01-01T00:00:00Z");
            assert!(row.last_error_code.is_none());
            let scripts = runner.scripts.lock().unwrap().join("\n");
            assert!(!scripts.contains("unrelated"));
        }
    }

    #[tokio::test]
    async fn fake_ssh_and_wsl_report_selected_pending_recovery_per_skill() {
        let snapshot = snapshot();
        let files =
            collect_remote_skill_files(&snapshot, "skills/safe-skill").expect("remote skill files");

        for (runner, fs) in fake_remote_filesystems(files) {
            let pool = crate::test_support::mem_pool().await;
            let operation_id = insert_pending_delete(&pool, &fs, "safe-skill").await;
            let plan =
                content_upsert_plan(input(&snapshot), "local-before".to_string(), None).unwrap();

            let outcomes = update_skills_batch(&pool, &fs, vec![plan], None).await;

            let error = outcomes[0].result.as_ref().unwrap_err();
            assert_eq!(
                error.phase,
                crate::services::central_updates::CentralUpdateFailurePhase::Recovery
            );
            assert_eq!(
                error.error().stable_error_code(),
                "central_operation.remote_fingerprint_protocol"
            );
            let row =
                crate::db::repos::fs_db_operations_repo::get_fs_db_operation(&pool, &operation_id)
                    .await
                    .unwrap()
                    .unwrap();
            assert_eq!(row.target_id, fs.target_id());
            assert_eq!(row.target_kind, fs.target_kind());
            assert_eq!(row.phase, "prepared");
            assert_eq!(
                row.last_error_code.as_deref(),
                Some("remote_fingerprint_protocol")
            );
            assert!(runner.stage_archive.lock().unwrap().is_none());
            assert!(!runner.swapped.load(Ordering::SeqCst));
        }
    }

    fn local_input<'a>(
        snapshot: &'a GitHubRepoSnapshot,
        target_dir: PathBuf,
        uid: &str,
    ) -> JournaledCentralContentUpsert<'a> {
        let mut value = input(snapshot);
        value.skill.uid = uid.to_string();
        value.skill.file_path = target_dir.join("SKILL.md").to_string_lossy().into_owned();
        value.skill.canonical_path = Some(target_dir.to_string_lossy().into_owned());
        value.target_dir = target_dir;
        value
    }

    #[tokio::test]
    async fn local_first_upsert_uses_had_target_false_and_creates_uid_once() {
        let pool = crate::test_support::mem_pool().await;
        let temp = tempfile::TempDir::new().unwrap();
        let target_dir = temp.path().join("safe-skill");
        let snapshot = snapshot();
        let created_uid = "uid-first-local";
        let state = journaled_central_content_upsert(
            &pool,
            &ActiveTarget::Local,
            local_input(&snapshot, target_dir.clone(), created_uid),
        )
        .await
        .expect("first upsert");
        assert_eq!(state.skill_id, "safe-skill");
        assert!(target_dir.join("SKILL.md").is_file());
        assert_eq!(
            std::fs::read(target_dir.join("references/guide.md")).unwrap(),
            b"# guide\n"
        );
        let stored = crate::db::repos::skills_repo::get_skill_by_id(&pool, "safe-skill")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.uid, created_uid);
        let journal = sqlx::query(
            "SELECT operation_kind, phase, manifest_json, old_fingerprint, new_fingerprint
             FROM fs_db_operations WHERE skill_id = 'safe-skill'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        use sqlx::Row;
        assert_eq!(journal.get::<String, _>("operation_kind"), "central_update");
        assert_eq!(journal.get::<String, _>("phase"), "completed");
        let manifest: serde_json::Value =
            serde_json::from_str(&journal.get::<String, _>("manifest_json")).unwrap();
        assert_eq!(manifest["payload"]["hadTarget"], false);
        assert!(manifest["payload"]["target"]
            .as_str()
            .unwrap()
            .ends_with("safe-skill"));
        assert!(manifest["payload"]["marker"]
            .as_str()
            .unwrap()
            .contains("operation-marker"));
        assert!(journal
            .get::<Option<String>, _>("old_fingerprint")
            .is_none());
        assert!(journal
            .get::<Option<String>, _>("new_fingerprint")
            .unwrap()
            .starts_with("sha256"));
    }

    #[tokio::test]
    async fn local_overwrite_preserves_uid_and_uses_had_target_true() {
        let pool = crate::test_support::mem_pool().await;
        let temp = tempfile::TempDir::new().unwrap();
        let target_dir = temp.path().join("safe-skill");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("SKILL.md"), b"---\nname: old\n---\n").unwrap();
        std::fs::write(target_dir.join("old-only.txt"), b"keep").unwrap();
        let persisted_uid = "uid-persisted-overwrite";
        crate::db::repos::skills_repo::upsert_skill(
            &pool,
            &Skill {
                id: "safe-skill".to_string(),
                uid: persisted_uid.to_string(),
                name: "old".to_string(),
                description: None,
                file_path: target_dir.join("SKILL.md").to_string_lossy().into_owned(),
                canonical_path: Some(target_dir.to_string_lossy().into_owned()),
                is_central: true,
                source: Some("local".to_string()),
                content: None,
                scanned_at: chrono::Utc::now().to_rfc3339(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .unwrap();

        let snapshot = snapshot();
        journaled_central_content_upsert(
            &pool,
            &ActiveTarget::Local,
            local_input(&snapshot, target_dir.clone(), "uid-must-not-win"),
        )
        .await
        .expect("overwrite upsert");

        let stored = crate::db::repos::skills_repo::get_skill_by_id(&pool, "safe-skill")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.uid, persisted_uid);
        assert!(!target_dir.join("old-only.txt").exists());
        assert_eq!(
            std::fs::read(target_dir.join("scripts/run.ps1")).unwrap(),
            b"Write-Output safe\n"
        );
        let manifest: serde_json::Value = serde_json::from_str(
            &sqlx::query_scalar::<_, String>(
                "SELECT manifest_json FROM fs_db_operations
                 WHERE skill_id = 'safe-skill' ORDER BY created_at DESC, rowid DESC LIMIT 1",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["payload"]["hadTarget"], true);
        assert!(manifest["payload"]["oldFingerprint"].as_str().is_some());
        assert!(manifest["payload"]["newFingerprint"].as_str().is_some());
    }

    #[tokio::test]
    async fn mismatched_target_identity_creates_no_journal_row() {
        let snapshot = snapshot();
        let mut value = input(&snapshot);
        value.skill.id = "other-skill".to_string();
        let error = content_upsert_plan(value, "hash".to_string(), None).unwrap_err();
        assert!(matches!(error, CentralUpdatesError::Batch(_)));
    }
}
