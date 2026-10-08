use std::cell::RefCell;
use std::rc::Rc;

use super::*;

type Observations = Rc<RefCell<Vec<GitHubImportProgressPayload>>>;

thread_local! {
    static OBSERVATIONS: RefCell<Option<Observations>> = const { RefCell::new(None) };
}

struct ObservationGuard(Option<Observations>);

impl ObservationGuard {
    fn start(observations: Observations) -> Self {
        Self(OBSERVATIONS.with(|slot| slot.replace(Some(observations))))
    }
}

impl Drop for ObservationGuard {
    fn drop(&mut self) {
        OBSERVATIONS.with(|slot| slot.replace(self.0.take()));
    }
}

pub(super) fn observe(payload: &GitHubImportProgressPayload) {
    OBSERVATIONS.with(|slot| {
        if let Some(observations) = slot.borrow().as_ref() {
            observations.borrow_mut().push(payload.clone());
        }
    });
}

/// Counts source emission attempts. AppNone does not establish IPC delivery.
#[tokio::test(flavor = "current_thread")]
#[ignore = "offline pinned-snapshot source probe; creates 13,016 temporary files"]
async fn pinned_snapshot_source_progress_probe() {
    let root = tempfile::Builder::new()
        .prefix("skillport-progress-probe-")
        .tempdir()
        .unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let _lock = crate::services::central_mutation::use_test_mutation_lock_path(
        root_path.join("locks/central.lock"),
    );
    let pool = crate::test_support::mem_pool().await;
    let central = root_path.join("central");
    crate::test_support::set_agent_dir(&pool, "central", &central).await;
    let count = std::env::var("SKILLPORT_PROGRESS_PROBE_FILES")
        .map(|value| value.parse::<usize>().unwrap())
        .unwrap_or(13_016);
    assert!((1..=18_000).contains(&count));
    let source = "skills/progress-probe";
    let mut files = HashMap::from([(
        format!("{source}/SKILL.md"),
        b"---\nname: progress-probe\ndescription: Offline source probe\n---\n".to_vec(),
    )]);
    for index in 1..count {
        files.insert(
            format!(
                "{source}/assets/group-{:04}/file-{index:05}.bin",
                index / 64
            ),
            vec![(index % 251) as u8; 1_024],
        );
    }
    let snapshot = GitHubRepoSnapshot { files };
    let bytes: u64 = snapshot
        .files
        .values()
        .map(|bytes| bytes.len() as u64)
        .sum();
    let observations = Observations::default();
    let observation_guard = ObservationGuard::start(observations.clone());
    let result = import_github_repo_skills_from_pinned_snapshot(
        &pool,
        &GitHubRepoRef {
            owner: "fixture".to_string(),
            repo: "offline".to_string(),
            branch: "main".to_string(),
            normalized_url: "https://github.com/fixture/offline".to_string(),
        },
        "0123456789abcdef0123456789abcdef01234567",
        &snapshot,
        vec![GitHubSkillImportSelection {
            source_path: source.to_string(),
            resolution: DuplicateResolution::Overwrite,
            renamed_skill_id: None,
        }],
        None,
    )
    .await
    .expect("offline pinned import");
    drop(observation_guard);
    assert_eq!(result.imported_skills.len(), 1);
    let target = central.join("progress-probe");
    assert!(target.starts_with(&root_path));
    for (path, expected) in &snapshot.files {
        let relative = repo_file_relative_to_source(path, source).unwrap();
        assert_eq!(std::fs::read(target.join(relative)).unwrap(), *expected);
    }
    assert!(db::get_skill_by_id(&pool, "progress-probe")
        .await
        .unwrap()
        .is_some());
    {
        let observations = observations.borrow();
        let last = observations.last().expect("final progress");
        assert_eq!((last.completed_files, last.completed_bytes), (count, bytes));
        assert!(observations.windows(2).all(|pair| {
            pair[0].completed_files <= pair[1].completed_files
                && pair[0].completed_bytes <= pair[1].completed_bytes
        }));
        println!(
            "SOURCE_PROGRESS {}",
            serde_json::json!({
                "app_handle": "none", "files": count, "bytes": bytes,
                "source_calls": observations.len(),
                "preparing": observations.iter().filter(|value| value.phase == GitHubImportProgressPhase::Preparing).count(),
                "writing": observations.iter().filter(|value| value.phase == GitHubImportProgressPhase::Writing).count(),
                "finalizing": observations.iter().filter(|value| value.phase == GitHubImportProgressPhase::Finalizing).count(),
                "completed_files": last.completed_files, "completed_bytes": last.completed_bytes,
                "disk_db": "PASS", "ipc_emit_count": "NOT_RUN", "native_webview": "NOT_RUN",
            })
        );
    }
    pool.close().await;
}

#[tokio::test]
async fn observer_flushes_partial_error_after_the_writer_closes() {
    let (sender, receiver) = tokio::sync::watch::channel(StageWriteProgress::default());
    let mut observed = Vec::new();
    let result: Result<(), &str> = observe_stage_write_progress(
        async move {
            sender.send_replace(StageWriteProgress {
                completed_files: 2,
                completed_bytes: 9,
                current_path: Some("second.txt".to_string()),
            });
            drop(sender);
            tokio::time::sleep(TokioDuration::from_millis(15)).await;
            Err("write failed")
        },
        receiver,
        |progress| observed.push(progress),
    )
    .await;
    assert_eq!(result, Err("write failed"));
    assert_eq!(observed.len(), 1);
    assert_eq!(
        (observed[0].completed_files, observed[0].completed_bytes),
        (2, 9)
    );
}

#[tokio::test]
async fn all_written_files_do_not_complete_the_service_before_settlement() {
    let (sender, receiver) = tokio::sync::watch::channel(StageWriteProgress::default());
    let settled = std::cell::Cell::new(false);
    let (allow_settle, wait_for_observer) = tokio::sync::oneshot::channel();
    let mut allow_settle = Some(allow_settle);
    let mut observed = Vec::new();
    let started = Instant::now();
    observe_stage_write_progress(
        async {
            for index in 1..=13_016 {
                sender.send_replace(StageWriteProgress {
                    completed_files: index,
                    completed_bytes: index as u64,
                    current_path: Some("last.bin".to_string()),
                });
            }
            drop(sender);
            wait_for_observer.await.unwrap();
            settled.set(true);
        },
        receiver,
        |progress| {
            if progress.completed_files == 13_016 && !settled.get() {
                if let Some(allow_settle) = allow_settle.take() {
                    allow_settle.send(()).unwrap();
                }
            }
            observed.push((progress, settled.get()));
        },
    )
    .await;
    assert!(observed
        .iter()
        .any(|(progress, settled)| progress.completed_files == 13_016 && !settled));
    assert_eq!(observed.last().unwrap().0.completed_files, 13_016);
    assert!(observed.last().unwrap().1);
    assert!(observed.len() <= started.elapsed().as_millis().div_ceil(100) as usize + 2);
}

#[tokio::test(flavor = "current_thread")]
async fn source_validation_does_not_advance_write_progress() {
    let root = tempfile::tempdir().unwrap();
    let pool = crate::test_support::mem_pool().await;
    let repo = GitHubRepoRef {
        owner: "fixture".to_string(),
        repo: "offline".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/fixture/offline".to_string(),
    };
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/progress-case/SKILL.md".to_string(),
            b"---\nname: test\n---\n".to_vec(),
        )]),
    };
    let candidate = build_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, None)
        .unwrap()
        .pop()
        .unwrap();
    let op = StagedImport {
        candidate,
        final_skill_id: "progress-case".to_string(),
        resolution: DuplicateResolution::Overwrite,
        source_files: vec![SnapshotSourceFile {
            repo_path: "skills/progress-case/../escape".to_string(),
            relative_path: "../escape".to_string(),
            byte_len: 10,
        }],
    };
    let mut state = GitHubImportProgressState {
        total_files: 1,
        total_bytes: 10,
        ..Default::default()
    };
    let error = import_single_staged_skill(
        &pool,
        &crate::services::central_updates::CentralFs::Local,
        &repo,
        &snapshot,
        root.path(),
        &op,
        None,
        &mut state,
        None,
        true,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        GithubImportError::RepoContainsUnsupportedPath(_)
    ));
    assert_eq!((state.completed_files, state.completed_bytes), (0, 0));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn remote_service_failure_does_not_count_a_completed_skill() {
    let pool = crate::test_support::mem_pool().await;
    let repo = GitHubRepoRef {
        owner: "fixture".to_string(),
        repo: "offline".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/fixture/offline".to_string(),
    };
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/progress-case/SKILL.md".to_string(),
            b"---\nname: test\n---\n".to_vec(),
        )]),
    };
    let candidate = build_repo_skill_candidates_from_snapshot_at_path(&repo, &snapshot, None)
        .unwrap()
        .pop()
        .unwrap();
    let op = StagedImport {
        candidate,
        final_skill_id: "progress-case".to_string(),
        resolution: DuplicateResolution::Overwrite,
        source_files: collect_snapshot_source_files(&snapshot, "skills/progress-case").unwrap(),
    };
    let runner = Arc::new(crate::test_support::FakeRunner::new());
    runner.push_error(
        crate::targets::RunnerPhase::Start,
        "synthetic remote failure",
    );
    let connection = crate::targets::ConnectedSshTarget::for_tests_with_runner(
        crate::targets::RemoteTargetConfig {
            id: "progress-failed-remote".to_string(),
            label: "Offline progress test".to_string(),
            host: "example.invalid".to_string(),
            username: "fixture".to_string(),
            port: 22,
            auth_method: crate::targets::SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/fixture".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        },
        runner.clone(),
    );
    let fs = crate::services::central_updates::CentralFs::Remote(Arc::new(
        ConnectedRemoteTarget::Ssh(connection),
    ));
    let mut state = GitHubImportProgressState {
        total_files: 1,
        ..Default::default()
    };
    assert!(import_single_staged_skill(
        &pool,
        &fs,
        &repo,
        &snapshot,
        Path::new("/home/fixture/central"),
        &op,
        None,
        &mut state,
        None,
        false
    )
    .await
    .is_err());
    assert_eq!((state.completed_files, state.completed_bytes), (0, 0));
    assert_eq!(runner.calls().len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn pinned_import_write_failure_flushes_partial_counts_and_rolls_back() {
    let root = tempfile::tempdir().unwrap();
    let _lock = crate::services::central_mutation::use_test_mutation_lock_path(
        root.path().join("locks/central.lock"),
    );
    let pool = crate::test_support::mem_pool().await;
    let central = root.path().join("central");
    crate::test_support::set_agent_dir(&pool, "central", &central).await;
    let manifest = b"---\nname: test\n---\n".to_vec();
    let successful_bytes = manifest.len() as u64 + 5;
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            ("skills/progress-case/SKILL.md".to_string(), manifest),
            ("skills/progress-case/assets".to_string(), b"first".to_vec()),
            (
                "skills/progress-case/assets/child".to_string(),
                b"must fail".to_vec(),
            ),
        ]),
    };
    let repo = GitHubRepoRef {
        owner: "fixture".to_string(),
        repo: "offline".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/fixture/offline".to_string(),
    };
    let observations = Observations::default();
    let observation_guard = ObservationGuard::start(observations.clone());
    let result = import_github_repo_skills_from_pinned_snapshot(
        &pool,
        &repo,
        "0123456789abcdef0123456789abcdef01234567",
        &snapshot,
        vec![GitHubSkillImportSelection {
            source_path: "skills/progress-case".to_string(),
            resolution: DuplicateResolution::Overwrite,
            renamed_skill_id: None,
        }],
        None,
    )
    .await;
    drop(observation_guard);
    assert!(result.is_err());
    {
        let observations = observations.borrow();
        let last = observations.last().unwrap();
        assert_eq!(last.phase, GitHubImportProgressPhase::Writing);
        assert_eq!(
            (last.completed_files, last.completed_bytes),
            (2, successful_bytes)
        );
        assert_eq!(last.total_files, 3);
        assert_eq!(last.current_path.as_deref(), Some("assets"));
        assert!(observations
            .windows(2)
            .all(|pair| pair[0].completed_files <= pair[1].completed_files
                && pair[0].completed_bytes <= pair[1].completed_bytes));
    }
    assert!(!central.join("progress-case").exists());
    assert_eq!(std::fs::read_dir(&central).unwrap().count(), 0);
    assert!(db::get_skill_by_id(&pool, "progress-case")
        .await
        .unwrap()
        .is_none());
    let phase: String =
        sqlx::query_scalar("SELECT phase FROM fs_db_operations WHERE skill_id = 'progress-case'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(phase, "rolled_back");
}
