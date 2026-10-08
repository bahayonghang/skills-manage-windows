//! Opt-in, offline service baseline. Ordinary test runs skip the fixture matrix.
//! Keep the same test-only spans and build profile for before/after runs.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{Layer, Registry};

use super::*;
use crate::db::repos::{agents_repo, installations_repo, skills_repo};
use crate::services::central_operation;
use crate::services::central_updates::fs::{
    collect_remote_skill_files, hash_remote_files, CentralSkillWrite, CopyRefreshRequest,
    OperationUpdateStage,
};
use crate::services::central_updates::inventory::{
    apply_remove_deleted_platform_copies_step, DeletedPlatformCopyRemoval, SkillUpdateApplyResult,
};
use crate::services::central_updates::snapshots::{
    repo_cache_key, CentralUpdateRepositorySnapshot,
};
use crate::services::github_import::{
    candidate_content_digest_from_snapshot, repository_snapshot_digest_from_local, GitHubRepoRef,
    GitHubRepoSnapshot, RemoteSkillCandidate,
};
use crate::targets::ActiveTarget;

const SKILL_ID: &str = "benchmark-skill";
const SOURCE_PATH: &str = "skills/benchmark-skill";
const MANIFEST: &[u8] =
    b"---\nname: benchmark-skill\ndescription: Synthetic offline fixture\n---\n";
const TARGET_SMALL_FILES: usize = 13_016;

#[derive(Default)]
struct PhaseField(Option<String>);

impl Visit for PhaseField {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "phase" {
            self.0 = Some(format!("{value:?}").trim_matches('"').to_string());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "phase" {
            self.0 = Some(value.to_string());
        }
    }
}

#[derive(Serialize, Default)]
struct PhaseSample {
    calls: usize,
    await_ms: f64,
}

type PhaseSamples = Arc<Mutex<BTreeMap<String, PhaseSample>>>;

struct PhaseLayer(PhaseSamples);
struct PhaseStart(String, Instant);

impl Layer<Registry> for PhaseLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, Registry>) {
        let mut field = PhaseField::default();
        attrs.record(&mut field);
        if let Some(phase) = field.0 {
            ctx.span(id)
                .expect("new span")
                .extensions_mut()
                .insert(PhaseStart(phase, Instant::now()));
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, Registry>) {
        let span = ctx.span(&id).expect("closing span");
        let extensions = span.extensions();
        if let Some(start) = extensions.get::<PhaseStart>() {
            let mut phases = self.0.lock().unwrap();
            let value = phases.entry(start.0.clone()).or_default();
            value.calls += 1;
            value.await_ms += start.1.elapsed().as_secs_f64() * 1_000.0;
        }
    }
}

#[derive(Clone, Copy, Serialize)]
struct Fixture {
    files: usize,
    directories: usize,
    bytes: u64,
}

struct Measurement {
    fixture: Fixture,
    sample: usize,
    phases: PhaseSamples,
}

impl Measurement {
    async fn run<T>(&self, operation: &str, future: impl Future<Output = T>) -> T {
        self.phases.lock().unwrap().clear();
        let started = Instant::now();
        let result = future.await;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "BENCH {}",
            serde_json::json!({
                "fixture": self.fixture, "sample": self.sample,
                "operation": operation, "elapsed_ms": elapsed_ms,
                "phases": &*self.phases.lock().unwrap(),
            })
        );
        result
    }
}

fn repository() -> GitHubRepoRef {
    GitHubRepoRef {
        owner: "fixture".to_string(),
        repo: "offline".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/fixture/offline".to_string(),
    }
}

fn snapshot(file_count: usize) -> (Arc<GitHubRepoSnapshot>, Fixture) {
    let total_bytes = MANIFEST.len() + (TARGET_SMALL_FILES - 1) * 1_024;
    let mut files = HashMap::from([(format!("{SOURCE_PATH}/SKILL.md"), MANIFEST.to_vec())]);
    for index in 1..file_count {
        let size = if file_count == 32 {
            let payload = total_bytes - MANIFEST.len();
            payload / 31 + usize::from(index <= payload % 31)
        } else {
            1_024
        };
        let mut bytes = vec![(index % 251) as u8; size];
        bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
        files.insert(
            format!(
                "{SOURCE_PATH}/assets/group-{:04}/file-{index:05}.bin",
                index / 64
            ),
            bytes,
        );
    }
    let mut directories = HashSet::new();
    for name in files.keys() {
        let relative = name.strip_prefix(&format!("{SOURCE_PATH}/")).unwrap();
        let mut parent = Path::new(relative).parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    let fixture = Fixture {
        files: files.len(),
        directories: directories.len() + 1,
        bytes: files.values().map(|bytes| bytes.len() as u64).sum(),
    };
    assert!(fixture.files + fixture.directories <= 20_000);
    assert!(fixture.bytes <= 256 * 1_024 * 1_024);
    assert!(files
        .values()
        .all(|bytes| bytes.len() <= 32 * 1_024 * 1_024));
    (Arc::new(GitHubRepoSnapshot { files }), fixture)
}

fn upsert_input<'a>(
    snapshot: &'a GitHubRepoSnapshot,
    target: &Path,
) -> JournaledCentralContentUpsert<'a> {
    JournaledCentralContentUpsert {
        skill: crate::db::Skill {
            source: Some("github:fixture/offline".to_string()),
            ..crate::test_support::central_skill_row(SKILL_ID, target)
        },
        repo: repository(),
        candidate: RemoteSkillCandidate {
            source_path: SOURCE_PATH.to_string(),
            skill_id: SKILL_ID.to_string(),
            skill_name: SKILL_ID.to_string(),
            description: Some("Synthetic offline fixture".to_string()),
            plugin_name: None,
            root_directory: "skills".to_string(),
            skill_directory_name: SKILL_ID.to_string(),
            download_url: "https://example.invalid/not-requested".to_string(),
        },
        snapshot,
        target_dir: target.to_path_buf(),
        resolved_commit_sha: Some("a".repeat(40)),
        content_digest: Some(
            candidate_content_digest_from_snapshot(snapshot, SOURCE_PATH).unwrap(),
        ),
    }
}

fn change_files(current: &GitHubRepoSnapshot, changed: usize) -> Arc<GitHubRepoSnapshot> {
    let mut files = current.files.clone();
    let mut names = files
        .keys()
        .filter(|path| !path.ends_with("/SKILL.md"))
        .cloned()
        .collect::<Vec<_>>();
    names.sort();
    for name in names.into_iter().take(changed) {
        let bytes = files.get_mut(&name).unwrap();
        *bytes.last_mut().unwrap() ^= 0x80;
    }
    Arc::new(GitHubRepoSnapshot { files })
}

async fn cache_snapshot(
    measurement: &Measurement,
    cache: &CentralUpdateSnapshotCache,
    snapshot: Arc<GitHubRepoSnapshot>,
) {
    let digest = measurement
        .run("snapshot_digest", async {
            repository_snapshot_digest_from_local(&snapshot)
        })
        .await;
    cache
        .insert(
            repo_cache_key(&repository()),
            Arc::new(CentralUpdateRepositorySnapshot::new(
                "a".repeat(40),
                digest,
                snapshot,
            )),
        )
        .unwrap();
}

fn assert_tree(root: &Path, snapshot: &GitHubRepoSnapshot, expected: Fixture) {
    let mut files = 0;
    let mut directories = 0;
    let mut bytes = 0;
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            directories += 1;
        } else {
            assert!(entry.file_type().is_file());
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let actual = std::fs::read(entry.path()).unwrap();
            assert_eq!(
                &actual,
                snapshot
                    .files
                    .get(&format!("{SOURCE_PATH}/{relative}"))
                    .unwrap()
            );
            files += 1;
            bytes += actual.len() as u64;
        }
    }
    assert_eq!(
        (files, directories, bytes),
        (expected.files, expected.directories, expected.bytes)
    );
}

async fn run_sample(snapshot: Arc<GitHubRepoSnapshot>, measurement: Measurement) {
    let root = tempfile::Builder::new()
        .prefix("skillport-io-benchmark-")
        .tempdir()
        .unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let _isolated_lock = crate::services::central_mutation::use_test_mutation_lock_path(
        root_path.join("locks/central.lock"),
    );
    let (pool, db_root) = crate::test_support::file_pool().await;
    let agents = agents_repo::get_all_agents(&pool).await.unwrap();
    let central = root_path.join("central");
    for agent in &agents {
        let path = if agent.id == "central" {
            central.clone()
        } else {
            root_path.join("agents").join(&agent.id)
        };
        assert!(path.starts_with(&root_path));
        crate::test_support::set_agent_dir(&pool, &agent.id, &path).await;
    }
    let copy_agents = agents
        .iter()
        .filter(|agent| agent.id != "central")
        .take(3)
        .map(|agent| agent.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(copy_agents.len(), 3);
    let target = central.join(SKILL_ID);
    let fs = CentralFs::Local;
    let cancel = AtomicBool::new(false);
    let client = reqwest::Client::new();
    let cache = CentralUpdateSnapshotCache::default();
    let files = measurement
        .run("component_snapshot_collect_clone", async {
            collect_remote_skill_files(&snapshot, SOURCE_PATH).unwrap()
        })
        .await;
    measurement
        .run("component_snapshot_hash_manifest", async {
            hash_remote_files(&snapshot, &files).unwrap()
        })
        .await;
    drop(files);
    let input = upsert_input(&snapshot, &target);
    measurement
        .run(
            "import_first",
            journaled_central_content_upsert_with_fs(&pool, &fs, input),
        )
        .await
        .unwrap();
    assert_tree(&target, &snapshot, measurement.fixture);
    let initial_uid = skills_repo::get_skill_by_id(&pool, SKILL_ID)
        .await
        .unwrap()
        .unwrap()
        .uid;
    let input = upsert_input(&snapshot, &target);
    measurement
        .run(
            "import_overwrite",
            journaled_central_content_upsert_with_fs(&pool, &fs, input),
        )
        .await
        .unwrap();
    cache_snapshot(&measurement, &cache, snapshot.clone()).await;
    let states = measurement
        .run(
            "check_unchanged",
            check_central_skill_updates_impl(
                None,
                "benchmark",
                &pool,
                &fs,
                &cancel,
                None,
                &client,
                &cache,
                Some(vec![SKILL_ID.to_string()]),
            ),
        )
        .await
        .unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].status, SkillUpdateStatus::UpToDate);
    let modified = std::fs::metadata(target.join("SKILL.md"))
        .unwrap()
        .modified()
        .unwrap();
    let outcome = measurement
        .run(
            "update_noop",
            update_central_skills_impl(
                None,
                "benchmark",
                &pool,
                &fs,
                &cancel,
                None,
                &client,
                &cache,
                vec![SKILL_ID.to_string()],
            ),
        )
        .await
        .unwrap();
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.failed.is_empty());
    assert_eq!(
        std::fs::metadata(target.join("SKILL.md"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    let mut current = snapshot;
    for (name, changed) in [
        ("update_1_file_0_copy", 1),
        (
            "update_1_percent_0_copy",
            measurement.fixture.files.div_ceil(100),
        ),
        ("update_all_0_copy", measurement.fixture.files - 1),
    ] {
        current = change_files(&current, changed);
        cache_snapshot(&measurement, &cache, current.clone()).await;
        let outcome = measurement
            .run(
                name,
                update_central_skills_impl(
                    None,
                    "benchmark",
                    &pool,
                    &fs,
                    &cancel,
                    None,
                    &client,
                    &cache,
                    vec![SKILL_ID.to_string()],
                ),
            )
            .await
            .unwrap();
        assert!(outcome.failed.is_empty(), "{name}: {:?}", outcome.failed);
        assert_eq!(outcome.succeeded, vec![SKILL_ID]);
    }
    let mut copy_targets = Vec::new();
    for (index, agent_id) in copy_agents.iter().enumerate() {
        let path = root_path.join("agents").join(agent_id).join(SKILL_ID);
        installations_repo::upsert_skill_installation(
            &pool,
            &crate::db::SkillInstallation {
                skill_id: SKILL_ID.to_string(),
                agent_id: agent_id.clone(),
                installed_path: path.to_string_lossy().into_owned(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();
        let setup = fs
            .refresh_copy_installs_cancellable(
                vec![CopyRefreshRequest {
                    skill_id: SKILL_ID.to_string(),
                    source_dir: target.clone(),
                    target: path.to_string_lossy().into_owned(),
                }],
                None,
            )
            .await;
        assert_eq!(setup.len(), 1);
        assert!(setup[0].result.is_ok());
        copy_targets.push(path);
        if index == 0 || index == 2 {
            current = change_files(&current, measurement.fixture.files - 1);
            cache_snapshot(&measurement, &cache, current.clone()).await;
            let name = if index == 0 {
                "update_all_1_copy"
            } else {
                "update_all_3_copy"
            };
            let outcome = measurement
                .run(
                    name,
                    update_central_skills_impl(
                        None,
                        "benchmark",
                        &pool,
                        &fs,
                        &cancel,
                        None,
                        &client,
                        &cache,
                        vec![SKILL_ID.to_string()],
                    ),
                )
                .await
                .unwrap();
            assert!(outcome.failed.is_empty(), "{name}: {:?}", outcome.failed);
            assert_eq!(outcome.succeeded, vec![SKILL_ID]);
        }
    }
    assert_eq!(
        skills_repo::get_skill_by_id(&pool, SKILL_ID)
            .await
            .unwrap()
            .unwrap()
            .uid,
        initial_uid
    );
    assert_tree(&target, &current, measurement.fixture);
    for copy in &copy_targets {
        assert_tree(copy, &current, measurement.fixture);
    }
    let deletion = measurement
        .run(
            "delete_retain_3_copy",
            crate::services::central_skills::delete_central_skills_impl(
                &pool,
                &[
                    crate::services::central_skills::BatchDeleteCentralSkillRequest {
                        skill_id: SKILL_ID.to_string(),
                        remove_agent_ids: Vec::new(),
                        force: false,
                    },
                ],
            ),
        )
        .await
        .unwrap();
    assert!(deletion.failed.is_empty(), "{:?}", deletion.failed);
    assert_eq!(deletion.succeeded.len(), 1);
    assert!(!target.exists());
    assert!(copy_targets.iter().all(|path| path.exists()));
    // A scan rebuilds these non-Central rows after the Central FK cascade.
    let mut platform_skill = crate::test_support::central_skill_row(SKILL_ID, &copy_targets[0]);
    platform_skill.is_central = false;
    platform_skill.canonical_path = None;
    skills_repo::upsert_skill(&pool, &platform_skill)
        .await
        .unwrap();
    for (agent_id, path) in copy_agents.iter().zip(&copy_targets) {
        installations_repo::upsert_skill_installation(
            &pool,
            &crate::db::SkillInstallation {
                skill_id: SKILL_ID.to_string(),
                agent_id: agent_id.clone(),
                installed_path: path.to_string_lossy().into_owned(),
                link_type: "copy".to_string(),
                symlink_target: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();
    }
    let removals = copy_agents
        .iter()
        .zip(&copy_targets)
        .map(|(agent_id, path)| DeletedPlatformCopyRemoval {
            agent_id: agent_id.clone(),
            skill_id: SKILL_ID.to_string(),
            paths: vec![path.to_string_lossy().into_owned()],
        })
        .collect::<Vec<_>>();
    for name in ["leftover_cleanup_3_copy", "leftover_cleanup_repeat"] {
        let mut outcome = SkillUpdateApplyResult::default();
        measurement
            .run(
                name,
                apply_remove_deleted_platform_copies_step(
                    &pool,
                    &ActiveTarget::Local,
                    removals.clone(),
                    &mut outcome,
                    None,
                    None,
                ),
            )
            .await;
        assert!(
            outcome.failures.is_empty(),
            "{name}: {:?}",
            outcome.failures
        );
        assert!(copy_targets.iter().all(|path| !path.exists()));
    }
    let pending = measurement
        .run(
            "recovery_repeat",
            central_operation::recover_pending_operations(&pool, &ActiveTarget::Local),
        )
        .await
        .unwrap();
    assert!(pending.is_empty());
    let delete_manifest_json: String = sqlx::query_scalar("SELECT manifest_json FROM fs_db_operations WHERE operation_kind = 'central_delete' ORDER BY rowid DESC LIMIT 1").fetch_one(&pool).await.unwrap();
    let central_operation::OperationManifest::Delete(manifest) =
        serde_json::from_str(&delete_manifest_json).unwrap()
    else {
        panic!("delete manifest")
    };
    measurement
        .run(
            "delete_finalize_repeat",
            central_operation::finalize_delete_local(&manifest),
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM fs_db_operations WHERE phase NOT IN ('completed', 'rolled_back')"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert!(!std::fs::read_dir(&central).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".skillport-")));
    pool.close().await;
    drop(db_root);
    // TempDir only removes its verified, freshly created fixture root.
    assert!(root_path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    root.close().unwrap();
}

async fn run_stage_sample(snapshot: Arc<GitHubRepoSnapshot>, measurement: Measurement) {
    let root = tempfile::Builder::new()
        .prefix("skillport-stage-benchmark-")
        .tempdir()
        .unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let _isolated_lock = crate::services::central_mutation::use_test_mutation_lock_path(
        root_path.join("locks/central.lock"),
    );
    let mutation_guard = crate::services::central_mutation::acquire_target_mutation_guard(
        &ActiveTarget::Local,
        "benchmark owned staging",
        std::time::Duration::from_secs(10),
    )
    .await
    .unwrap();
    let fs = CentralFs::Local;
    let files = collect_remote_skill_files(&snapshot, SOURCE_PATH).unwrap();
    let write = CentralSkillWrite {
        skill_id: SKILL_ID.to_string(),
        target_dir: root_path.join("central").join(SKILL_ID),
        files,
    };
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manifest = fs
        .build_operation_update_manifest(&operation_id, &write, Vec::new())
        .await
        .unwrap();
    assert!(!manifest.had_target);
    for path in [&manifest.staging, &manifest.backup, &manifest.marker] {
        assert!(Path::new(path).starts_with(&root_path));
    }
    let stages = vec![OperationUpdateStage {
        manifest: manifest.clone(),
        write,
    }];
    let outcome = measurement
        .run(
            "component_durable_stage",
            fs.stage_operation_updates(stages, None),
        )
        .await;
    assert_eq!(outcome.len(), 1);
    assert!(outcome[0].result.is_ok(), "{:?}", outcome[0].result);
    let staging = Path::new(&manifest.staging);
    assert_tree(staging, &snapshot, measurement.fixture);
    let delete_id = uuid::Uuid::new_v4().to_string();
    let deletion = measurement
        .run(
            "component_delete_manifest_fingerprint",
            central_operation::build_local_delete_manifest(&delete_id, vec![staging.to_path_buf()]),
        )
        .await
        .unwrap();
    measurement
        .run(
            "component_delete_stage",
            central_operation::stage_delete_local(&deletion),
        )
        .await
        .unwrap();
    measurement
        .run(
            "component_delete_finalize",
            central_operation::finalize_delete_local(&deletion),
        )
        .await
        .unwrap();
    measurement
        .run(
            "component_delete_finalize_repeat",
            central_operation::finalize_delete_local(&deletion),
        )
        .await
        .unwrap();
    assert!(!staging.exists());
    // This helper probe has no DB journal. Production rollback consumes only
    // the update marker owned by this fixture after delete has removed staging.
    fs.rollback_operation_update(&manifest, central_operation::OperationPhase::Prepared)
        .await
        .unwrap();
    assert!(!Path::new(&manifest.marker).exists());
    drop(mutation_guard);
    assert!(root_path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    root.close().unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "Opt-in filesystem service benchmark; generates up to 18,000 files per tree"]
async fn small_files_service_baseline() {
    let counts = std::env::var("SKILLPORT_BENCH_FILES")
        .unwrap_or_else(|_| "128,4096,13016,18000,32".to_string());
    let counts = counts
        .split(',')
        .map(|value| value.parse::<usize>().expect("file count"))
        .collect::<Vec<_>>();
    assert!(counts
        .iter()
        .all(|value| [128, 4_096, 13_016, 18_000, 32].contains(value)));
    let samples = std::env::var("SKILLPORT_BENCH_SAMPLES")
        .map(|value| value.parse::<usize>().expect("sample count"))
        .unwrap_or(7);
    assert!((1..=7).contains(&samples));
    let mode = std::env::var("SKILLPORT_BENCH_MODE").unwrap_or_else(|_| "service".to_string());
    assert!(["service", "stage"].contains(&mode.as_str()));
    let phases = PhaseSamples::default();
    tracing::subscriber::set_global_default(Registry::default().with(PhaseLayer(phases.clone())))
        .expect("isolated benchmark subscriber");
    println!(
        "BENCH_META {}",
        serde_json::json!({"schema":1,"profile":if cfg!(debug_assertions) {"debug"} else {"release"},"mode":mode,"files":counts,"samples":samples,"network":"not_requested","app_handle":false,"phase_time":"inclusive wrapper await wall ms","os_cache":"not_cleared","security_scanner":"not_modified"})
    );
    for count in counts {
        let (snapshot, fixture) = snapshot(count);
        for sample in 0..samples {
            let measurement = Measurement {
                fixture,
                sample,
                phases: phases.clone(),
            };
            if mode == "stage" {
                run_stage_sample(snapshot.clone(), measurement).await;
            } else {
                run_sample(snapshot.clone(), measurement).await;
            }
        }
    }
}
