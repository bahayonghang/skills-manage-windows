use std::path::PathBuf;

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

async fn journaled_central_content_upsert_with_fs(
    pool: &DbPool,
    fs: &CentralFs,
    input: JournaledCentralContentUpsert<'_>,
) -> Result<SkillUpdateState, CentralUpdatesError> {
    let local_hash = fs
        .hash_directories(std::slice::from_ref(&input.target_dir))
        .await?
        .remove(&input.target_dir)
        .ok_or_else(|| {
            CentralUpdatesError::Batch(
                "Central content upsert could not observe the target directory.".to_string(),
            )
        })?;
    let plan = content_upsert_plan(input, local_hash)?;
    let skill_id = plan.skill.id.clone();
    let mut outcomes = update_skills_batch(pool, fs, vec![plan], None).await;
    let outcome = outcomes.pop().ok_or_else(|| {
        CentralUpdatesError::Batch(format!(
            "Central content upsert returned no outcome for skill '{skill_id}'."
        ))
    })?;
    outcome.result
}

fn content_upsert_plan(
    input: JournaledCentralContentUpsert<'_>,
    local_hash: String,
) -> Result<SkillUpdatePlan, CentralUpdatesError> {
    let files = collect_remote_skill_files(input.snapshot, &input.candidate.source_path)?;
    ensure_remote_skill_manifest(&files)?;
    let remote_hash = hash_remote_files(input.snapshot, &files)?;
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
        skill: input.skill,
        remote,
        refresh_copies: false,
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
                CentralFs::Remote(Box::new(ConnectedRemoteTarget::Ssh(ssh))),
            ),
            (
                wsl_runner,
                CentralFs::Remote(Box::new(ConnectedRemoteTarget::Wsl(wsl))),
            ),
        ]
    }

    fn remote_hash_output(
        root: &str,
        files: &[crate::services::central_updates::fs::RemoteSkillFile],
    ) -> String {
        let mut output = format!("ROOT\t{root}\n");
        for file in files {
            output.push_str(&format!(
                "{:x}\t{}\n",
                Sha256::digest(&file.bytes),
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

            let stored = crate::db::get_skill_by_id(&pool, "safe-skill")
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
            let assignment = crate::db::get_skill_repository_assignment(&pool, "safe-skill")
                .await
                .expect("repository assignment");
            assert_eq!(assignment.repository.owner.as_deref(), Some("owner"));
            assert_eq!(assignment.repository.repo.as_deref(), Some("repo"));
            assert_eq!(assignment.repository.branch.as_deref(), Some("main"));
            assert_eq!(assignment.source_path.as_deref(), Some("skills/safe-skill"));
            assert!(!assignment.is_source_unknown);
            let provenance = crate::db::get_skill_repository_provenance(&pool, "safe-skill")
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
}
