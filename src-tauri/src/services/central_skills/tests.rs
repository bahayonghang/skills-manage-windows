use super::delete::ensure_remote_child_path;
use super::files::{open_in_file_manager_checked_impl, read_file_by_path_impl};
use super::query::{get_skill_detail_with_row_impl, get_skills_by_agent_impl};
use super::types::{BatchDeleteCentralSkillRequest, SkillWithLinks};
use super::*;
use crate::db::{self, AgentSkillObservation, Skill, SkillInstallation};
use chrono::Utc;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

use crate::test_support::mem_pool as setup_test_db;

async fn set_test_central_root(pool: &SqlitePool, root: &Path) {
    crate::test_support::set_agent_dir(pool, "central", root).await;
}

fn write_test_skill_dir(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: test skill\n---\n",
    )
    .unwrap();
}

fn make_directory_tree_fixture(root: &Path) {
    fs::create_dir_all(root.join("examples").join("nested")).unwrap();
    fs::write(root.join("SKILL.md"), "# Demo").unwrap();
    fs::write(root.join("examples").join("demo.md"), "demo").unwrap();
    fs::write(
        root.join("examples").join("nested").join("deep.txt"),
        "deep",
    )
    .unwrap();
}

fn make_central_skill_at(id: &str, name: &str, dir: &Path) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: name.to_string(),
        description: Some(format!("Desc for {}", name)),
        file_path: dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

fn make_installation_at(
    skill_id: &str,
    agent_id: &str,
    dir: &Path,
    link_type: &str,
    symlink_target: Option<&Path>,
) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: dir.to_string_lossy().into_owned(),
        link_type: link_type.to_string(),
        symlink_target: symlink_target.map(|path| path.to_string_lossy().into_owned()),
        created_at: Utc::now().to_rfc3339(),
    }
}

fn make_skill(id: &str, name: &str, is_central: bool) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: name.to_string(),
        description: Some(format!("Desc for {}", name)),
        file_path: format!("/tmp/{}/SKILL.md", id),
        canonical_path: if is_central {
            Some(format!("/tmp/central/{}", id))
        } else {
            None
        },
        is_central,
        source: if is_central {
            Some("native".to_string())
        } else {
            Some("copy".to_string())
        },
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

fn make_remote_central_skill(id: &str, dir: &str) -> Skill {
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: id.to_string(),
        description: Some(format!("Desc for {}", id)),
        file_path: format!("{}/SKILL.md", dir.trim_end_matches('/')),
        canonical_path: Some(dir.to_string()),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

fn make_remote_installation(
    skill_id: &str,
    agent_id: &str,
    installed_path: &str,
    link_type: &str,
) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: installed_path.to_string(),
        link_type: link_type.to_string(),
        symlink_target: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

async fn insert_pending_delete_collision(pool: &SqlitePool, root: &Path, skill_id: &str) -> String {
    let operation_id = format!("pending-delete-{skill_id}");
    let manifest = crate::services::central_operation::OperationManifest::Delete(
        crate::services::central_operation::DeleteManifest {
            version: crate::services::central_operation::MANIFEST_VERSION,
            operation_id: operation_id.clone(),
            paths: vec![crate::services::central_operation::ManagedPath {
                original: root
                    .join(format!("{skill_id}-original"))
                    .to_string_lossy()
                    .into_owned(),
                backup: root
                    .join(format!("{skill_id}-backup"))
                    .to_string_lossy()
                    .into_owned(),
                marker: root
                    .join(format!("{skill_id}-marker"))
                    .to_string_lossy()
                    .into_owned(),
                expected_present: true,
                fingerprint: None,
            }],
        },
    );
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: &operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
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

async fn insert_pending_update(pool: &SqlitePool, root: &Path, skill_id: &str) -> String {
    let operation_id = format!("pending-update-{skill_id}");
    let target = root.join(skill_id);
    let old_fingerprint = crate::services::central_updates::CentralFs::Local
        .hash_directories(std::slice::from_ref(&target))
        .await
        .unwrap()
        .remove(&target)
        .expect("pending update target fingerprint");
    let manifest = crate::services::central_operation::UpdateManifest {
        version: crate::services::central_operation::MANIFEST_VERSION,
        operation_id: operation_id.clone(),
        target: target.to_string_lossy().into_owned(),
        staging: root
            .join(format!("{skill_id}-staging"))
            .to_string_lossy()
            .into_owned(),
        backup: root
            .join(format!("{skill_id}-backup"))
            .to_string_lossy()
            .into_owned(),
        marker: root
            .join(format!("{skill_id}-marker"))
            .to_string_lossy()
            .into_owned(),
        had_target: true,
        old_fingerprint: Some(old_fingerprint.clone()),
        new_fingerprint: "sha256-manifest:selected-update".to_string(),
        copies: Vec::new(),
    };
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Update(manifest.clone()),
    )
    .unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: &operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
            operation_kind: "central_update",
            skill_id,
            manifest_version: crate::services::central_operation::MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: Some(&old_fingerprint),
            new_fingerprint: Some(&manifest.new_fingerprint),
        },
    )
    .await
    .unwrap();
    operation_id
}

async fn insert_remote_pending_delete_collision(
    pool: &SqlitePool,
    target_id: &str,
    skill_id: &str,
) -> String {
    let operation_id = format!("pending-delete-{skill_id}");
    let root = "/home/alice/.skillsmanage/skills";
    let manifest = crate::services::central_operation::OperationManifest::Delete(
        crate::services::central_operation::DeleteManifest {
            version: crate::services::central_operation::MANIFEST_VERSION,
            operation_id: operation_id.clone(),
            paths: vec![crate::services::central_operation::ManagedPath {
                original: format!("{root}/{skill_id}-original"),
                backup: format!("{root}/{skill_id}-backup"),
                marker: format!("{root}/{skill_id}-marker"),
                expected_present: true,
                fingerprint: None,
            }],
        },
    );
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: &operation_id,
            batch_id: None,
            target_id,
            target_kind: "ssh",
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
    operation_id
}

async fn insert_remote_pending_update(
    pool: &SqlitePool,
    target_id: &str,
    target_kind: &str,
    skill_id: &str,
) -> String {
    let operation_id = format!("pending-update-{target_kind}-{skill_id}");
    let root = "/home/alice/.skillsmanage/skills";
    let target = format!("{root}/{skill_id}");
    let empty_fingerprint = concat!(
        "sha256-manifest:",
        "e3b0c44298fc1c149afbf4c8996fb924",
        "27ae41e4649b934ca495991b7852b855"
    );
    let manifest = crate::services::central_operation::UpdateManifest {
        version: crate::services::central_operation::MANIFEST_VERSION,
        operation_id: operation_id.clone(),
        target,
        staging: format!("{root}/.{skill_id}-staging"),
        backup: format!("{root}/.{skill_id}-backup"),
        marker: format!("{root}/.{skill_id}-marker"),
        had_target: true,
        old_fingerprint: Some(empty_fingerprint.to_string()),
        new_fingerprint: "sha256-manifest:selected-update".to_string(),
        copies: Vec::new(),
    };
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Update(manifest.clone()),
    )
    .unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: &operation_id,
            batch_id: None,
            target_id,
            target_kind,
            operation_kind: "central_update",
            skill_id,
            manifest_version: crate::services::central_operation::MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: Some(empty_fingerprint),
            new_fingerprint: Some(&manifest.new_fingerprint),
        },
    )
    .await
    .unwrap();
    operation_id
}

#[tokio::test]
async fn skill_reference_resolution_is_deterministic() {
    let pool = setup_test_db().await;
    let alpha = make_skill("alpha-slug", "Shared Name", true);
    let beta = make_skill("beta-slug", "Shared Name", true);
    let unique = make_skill("unique-slug", "Unique Name", true);
    let platform_shadow = make_skill("platform-shadow", "Unique Name", false);
    db::upsert_skill(&pool, &alpha).await.unwrap();
    db::upsert_skill(&pool, &beta).await.unwrap();
    db::upsert_skill(&pool, &unique).await.unwrap();
    db::upsert_skill(&pool, &platform_shadow).await.unwrap();

    assert_eq!(
        resolve_skill_ref_impl(&pool, &alpha.uid).await.unwrap().id,
        alpha.id
    );
    assert_eq!(
        resolve_skill_ref_impl(&pool, &beta.id).await.unwrap().uid,
        beta.uid
    );
    assert_eq!(
        resolve_skill_ref_impl(&pool, "Unique Name")
            .await
            .unwrap()
            .id,
        unique.id
    );
    assert!(matches!(
        resolve_skill_ref_impl(&pool, "Shared Name").await,
        Err(CentralSkillsError::AmbiguousSkillReference(_))
    ));
    assert!(matches!(
        resolve_skill_ref_impl(&pool, "missing").await,
        Err(CentralSkillsError::SkillNotFound(_))
    ));
}

fn make_observation(
    row_id: &str,
    skill_id: &str,
    name: &str,
    dir_path: &str,
    source_kind: &str,
    read_only: bool,
) -> AgentSkillObservation {
    make_observation_for_agent(
        "claude-code",
        row_id,
        skill_id,
        name,
        dir_path,
        source_kind,
        read_only,
    )
}

fn make_observation_for_agent(
    agent_id: &str,
    row_id: &str,
    skill_id: &str,
    name: &str,
    dir_path: &str,
    source_kind: &str,
    read_only: bool,
) -> AgentSkillObservation {
    AgentSkillObservation {
        row_id: row_id.to_string(),
        agent_id: agent_id.to_string(),
        skill_id: skill_id.to_string(),
        name: name.to_string(),
        description: Some(format!("{source_kind} copy")),
        file_path: format!("{dir_path}/SKILL.md"),
        dir_path: dir_path.to_string(),
        source_kind: source_kind.to_string(),
        source_root: if source_kind == "user" && agent_id == "claude-code" {
            "/tmp/.claude/skills".to_string()
        } else if source_kind == "user" {
            format!("/tmp/.agents/skills/{agent_id}")
        } else if agent_id == "codex" {
            "/tmp/.codex/plugins/cache/openai/example/1.0.0".to_string()
        } else {
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0".to_string()
        },
        link_type: "copy".to_string(),
        symlink_target: None,
        is_read_only: read_only,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

#[test]
fn test_remote_child_path_guard_normalizes_and_rejects_unsafe_paths() {
    assert_eq!(
        ensure_remote_child_path(
            "/home/alice/.skillsmanage/skills/",
            "/home/alice/.skillsmanage/skills/demo",
            "demo",
        )
        .unwrap(),
        "/home/alice/.skillsmanage/skills/demo"
    );

    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/.skillsmanage/skills",
        "root",
    )
    .is_err());
    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/other/demo",
        "outside",
    )
    .is_err());
    assert!(ensure_remote_child_path(
        "/home/alice/.skillsmanage/skills",
        "/home/alice/.skillsmanage/skills/../other",
        "traversal",
    )
    .is_err());
}

#[tokio::test]
async fn test_preview_remote_delete_uses_remote_paths_and_installations() {
    let pool = setup_test_db().await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind("/home/alice/.skillsmanage/skills")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind("/home/alice/.agents/skills")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind("/home/alice/.claude/skills")
        .execute(&pool)
        .await
        .unwrap();

    db::upsert_skill(
        &pool,
        &make_remote_central_skill(
            "remote-delete",
            "/home/alice/.skillsmanage/skills/remote-delete",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "cursor",
            "/home/alice/.agents/skills/remote-delete",
            "copy",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "claude-code",
            "/home/alice/.claude/skills/remote-delete",
            "symlink",
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_remote_installation(
            "remote-delete",
            "central",
            "/home/alice/.skillsmanage/skills/remote-delete",
            "native",
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_ssh_impl(&pool, &["remote-delete".to_string()])
        .await
        .unwrap();

    assert!(result.failed.is_empty());
    assert_eq!(
        result.previews[0].central_path,
        "/home/alice/.skillsmanage/skills/remote-delete"
    );
    assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
    assert_eq!(
        result.previews[0].auto_removed_agent_ids,
        vec!["claude-code"]
    );
}

#[tokio::test]
async fn test_preview_remote_delete_rejects_central_path_outside_remote_root() {
    let pool = setup_test_db().await;
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind("/home/alice/.skillsmanage/skills")
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill(
        &pool,
        &make_remote_central_skill("outside-remote", "/tmp/outside-remote"),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_ssh_impl(&pool, &["outside-remote".to_string()])
        .await
        .unwrap();

    assert!(result.previews.is_empty());
    let failure = &result.failed[0];
    assert_eq!(failure.skill_id, "outside-remote");
    assert_eq!(failure.phase.as_deref(), Some("prepare"));
    assert_eq!(
        failure.error_code.as_deref(),
        Some("central_skills.delete_preview_failed")
    );
    assert_eq!(
        failure.error_category.as_deref(),
        Some("central_skills.validation")
    );
    assert_eq!(failure.error, "This Central skill could not be deleted.");
    let serialized = serde_json::to_string(failure).unwrap();
    assert!(!serialized.contains("/tmp/outside-remote"));
}

// ── get_skills_by_agent ───────────────────────────────────────────────────

#[tokio::test]
async fn test_get_skills_by_agent_returns_correct_skills() {
    let pool = setup_test_db().await;

    let skill_a = make_skill("skill-a", "Skill A", false);
    let skill_b = make_skill("skill-b", "Skill B", false);
    db::upsert_skill(&pool, &skill_a).await.unwrap();
    db::upsert_skill(&pool, &skill_b).await.unwrap();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "skill-a".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/skill-a/SKILL.md".to_string(),
            link_type: "symlink".to_string(),
            symlink_target: Some("/tmp/central/skill-a".to_string()),
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let skills = db::get_skills_by_agent(&pool, "claude-code").await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "skill-a");
}

#[tokio::test]
async fn test_get_skills_by_agent_empty_for_unknown_agent() {
    let pool = setup_test_db().await;
    let skills = db::get_skills_by_agent(&pool, "nonexistent-agent")
        .await
        .unwrap();
    assert!(skills.is_empty());
}

// ── get_central_skills ────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_central_skills_includes_linked_agents() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("central-a", "Central A", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();

    // Install to claude-code and cursor.
    for agent_id in &["claude-code", "cursor"] {
        db::upsert_skill_installation(
            &pool,
            &SkillInstallation {
                skill_id: "central-a".to_string(),
                agent_id: agent_id.to_string(),
                installed_path: format!("/tmp/{}/central-a/SKILL.md", agent_id),
                link_type: "symlink".to_string(),
                symlink_target: Some("/tmp/central/central-a".to_string()),
                created_at: Utc::now().to_rfc3339(),
            },
        )
        .await
        .unwrap();
    }

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);

    let mut linked = skills_with_links[0].linked_agents.clone();
    linked.sort();
    let mut expected_linked: Vec<String> = vec!["claude-code".to_string(), "cursor".to_string()];
    expected_linked.sort();
    assert_eq!(linked, expected_linked);

    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_no_links() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("central-solo", "Solo Central", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);
    let mut linked = skills_with_links[0].linked_agents.clone();
    linked.sort();
    assert!(linked.is_empty());

    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_ignores_claude_plugin_observations() {
    let pool = setup_test_db().await;

    let central_skill = make_skill("shared-skill", "Shared Skill", true);
    db::upsert_skill(&pool, &central_skill).await.unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(skills_with_links.len(), 1);
    assert_eq!(
        {
            let mut linked = skills_with_links[0].linked_agents.clone();
            linked.sort();
            linked
        },
        Vec::<String>::new(),
        "plugin observations must not pollute linked_agents state"
    );
    let mut shared = skills_with_links[0].shared_root_agents.clone();
    shared.sort();
    assert!(shared.is_empty());
}

#[tokio::test]
async fn test_get_central_skills_excludes_non_central() {
    let pool = setup_test_db().await;

    let central = make_skill("c-skill", "Central", true);
    let non_central = make_skill("nc-skill", "Non-Central", false);
    db::upsert_skill(&pool, &central).await.unwrap();
    db::upsert_skill(&pool, &non_central).await.unwrap();

    let skills_with_links = get_central_skills_impl(&pool).await.unwrap();
    assert_eq!(
        skills_with_links.len(),
        1,
        "only central skills should be returned"
    );
    assert_eq!(skills_with_links[0].id, "c-skill");
}

#[tokio::test]
async fn test_get_central_skills_page_filters_sorts_and_counts_total() {
    let pool = setup_test_db().await;

    let mut alpha = make_skill("alpha", "Alpha Tool", true);
    alpha.fs_updated_at = Some("2026-05-17T01:00:00Z".to_string());
    let mut beta = make_skill("beta", "Beta Tool", true);
    beta.fs_updated_at = Some("2026-05-18T01:00:00Z".to_string());
    let ignored = make_skill("gamma", "Gamma Tool", false);
    db::upsert_skill(&pool, &alpha).await.unwrap();
    db::upsert_skill(&pool, &beta).await.unwrap();
    db::upsert_skill(&pool, &ignored).await.unwrap();

    let page = get_central_skills_page_impl(
        &pool,
        CentralSkillsPageRequest {
            query: Some("tool".to_string()),
            sort: Some("updatedAt:desc".to_string()),
            limit: Some(1),
            offset: Some(0),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "beta");
}

// ── get_skill_detail ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_central_skill_rejects_non_central_skill() {
    let pool = setup_test_db().await;
    let skill = make_skill("plain-skill", "Plain Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let error = delete_central_skill_impl(&pool, "plain-skill", &[])
        .await
        .unwrap_err();

    assert!(error.to_string().contains("is not a Central skill"));
}

#[tokio::test]
async fn test_delete_central_skill_rejects_path_outside_central_root() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let outside_dir = temp.path().join("outside").join("outside-skill");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("outside-skill", "Outside Skill", &outside_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let error = delete_central_skill_impl(&pool, "outside-skill", &[])
        .await
        .unwrap_err();

    assert!(error.to_string().contains("outside Central Skills root"));
    assert!(outside_dir.exists());
    assert!(db::get_skill_by_id(&pool, "outside-skill")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_delete_central_skill_removes_selected_copy_and_retains_unselected_copy() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("central-delete");
    let removed_copy_dir = temp.path().join("cursor").join("central-delete");
    let retained_copy_dir = temp.path().join("claude").join("central-delete");
    let missing_symlink_path = temp.path().join("codex").join("central-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&removed_copy_dir);
    write_test_skill_dir(&retained_copy_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("central-delete", "Central Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("central-delete", "cursor", &removed_copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "central-delete",
            "claude-code",
            &retained_copy_dir,
            "copy",
            None,
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "central-delete",
            "codex",
            &missing_symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = delete_central_skill_impl(&pool, "central-delete", &["cursor".to_string()])
        .await
        .unwrap();

    assert_eq!(
        result.removed_central_path,
        central_dir.to_string_lossy().into_owned()
    );
    let mut removed_agent_ids = result.removed_agent_ids;
    removed_agent_ids.sort();
    assert_eq!(
        removed_agent_ids,
        vec!["codex".to_string(), "cursor".to_string()]
    );
    assert_eq!(result.retained_agent_ids, vec!["claude-code".to_string()]);
    assert!(!central_dir.exists());
    assert!(!removed_copy_dir.exists());
    assert!(retained_copy_dir.exists());
    assert!(db::get_skill_by_id(&pool, "central-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_installations(&pool, "central-delete")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_preview_delete_central_skills_reports_copies_and_preview_failures() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("preview-delete");
    let copy_dir = temp.path().join("cursor").join("preview-delete");
    let missing_symlink_path = temp.path().join("codex").join("preview-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&copy_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("preview-delete", "Preview Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("preview-delete", "cursor", &copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "preview-delete",
            "codex",
            &missing_symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_impl(
        &pool,
        &["preview-delete".to_string(), "missing-delete".to_string()],
    )
    .await
    .unwrap();

    assert_eq!(result.previews.len(), 1);
    assert_eq!(result.previews[0].skill_id, "preview-delete");
    assert_eq!(result.previews[0].copy_installations.len(), 1);
    assert_eq!(result.previews[0].copy_installations[0].agent_id, "cursor");
    assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "missing-delete");
}

#[tokio::test]
async fn test_preview_delete_central_skills_reports_auto_links_without_central_self() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("linked-delete");
    let symlink_path = temp.path().join("codex").join("linked-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    set_test_central_root(&pool, &central_root).await;

    let skill = make_central_skill_at("linked-delete", "Linked Delete", &central_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("linked-delete", "central", &central_dir, "native", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "linked-delete",
            "codex",
            &symlink_path,
            "symlink",
            Some(&central_dir),
        ),
    )
    .await
    .unwrap();

    let result = preview_delete_central_skills_impl(&pool, &["linked-delete".to_string()])
        .await
        .unwrap();

    assert!(result.failed.is_empty());
    assert_eq!(result.previews.len(), 1);
    assert!(result.previews[0].copy_installations.is_empty());
    assert_eq!(result.previews[0].auto_removed_agent_ids, vec!["codex"]);
}

#[tokio::test]
async fn test_batch_delete_central_skills_keeps_partial_failures_isolated() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let valid_dir = central_root.join("valid-delete");
    let outside_dir = temp.path().join("outside").join("unsafe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&valid_dir);
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("valid-delete", "Valid Delete", &valid_dir),
    )
    .await
    .unwrap();
    db::upsert_skill(
        &pool,
        &make_central_skill_at("unsafe-delete", "Unsafe Delete", &outside_dir),
    )
    .await
    .unwrap();

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "valid-delete".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "unsafe-delete".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "valid-delete");
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "unsafe-delete");
    assert!(!valid_dir.exists());
    assert!(outside_dir.exists());
    assert!(db::get_skill_by_id(&pool, "valid-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "unsafe-delete")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_batch_delete_central_skills_dedupes_and_merges_copy_agents() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("dedupe-delete");
    let cursor_copy_dir = temp.path().join("cursor").join("dedupe-delete");
    let claude_copy_dir = temp.path().join("claude").join("dedupe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    write_test_skill_dir(&cursor_copy_dir);
    write_test_skill_dir(&claude_copy_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("dedupe-delete", "Dedupe Delete", &central_dir),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at("dedupe-delete", "cursor", &cursor_copy_dir, "copy", None),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &make_installation_at(
            "dedupe-delete",
            "claude-code",
            &claude_copy_dir,
            "copy",
            None,
        ),
    )
    .await
    .unwrap();

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "dedupe-delete".to_string(),
                remove_agent_ids: vec!["cursor".to_string()],
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "dedupe-delete".to_string(),
                remove_agent_ids: vec!["claude-code".to_string(), "cursor".to_string()],
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 1);
    assert!(result.failed.is_empty());
    let mut removed_agent_ids = result.succeeded[0].removed_agent_ids.clone();
    removed_agent_ids.sort();
    assert_eq!(
        removed_agent_ids,
        vec!["claude-code".to_string(), "cursor".to_string()]
    );
    assert!(!central_dir.exists());
    assert!(!cursor_copy_dir.exists());
    assert!(!claude_copy_dir.exists());
}

#[tokio::test]
async fn delete_selected_skill_ignores_unrelated_pending_collision() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let selected_dir = central_root.join("claude-md-improver");
    write_test_skill_dir(&selected_dir);
    set_test_central_root(&pool, &central_root).await;
    db::upsert_skill(
        &pool,
        &make_central_skill_at("claude-md-improver", "Claude MD Improver", &selected_dir),
    )
    .await
    .unwrap();
    let unrelated_operation = insert_pending_delete_collision(&pool, temp.path(), "yao-meta").await;
    let before = db::get_fs_db_operation(&pool, &unrelated_operation)
        .await
        .unwrap()
        .unwrap();

    let result = delete_central_skill_impl(&pool, "claude-md-improver", &[]).await;

    assert!(result.is_ok(), "{result:?}");
    assert!(!selected_dir.exists());
    assert!(db::get_skill_by_id(&pool, "claude-md-improver")
        .await
        .unwrap()
        .is_none());
    let after = db::get_fs_db_operation(&pool, &unrelated_operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.phase, before.phase);
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(after.last_error_code, before.last_error_code);
    assert_eq!(after.last_error_message, before.last_error_message);
}

#[tokio::test]
async fn batch_delete_reports_selected_recovery_collision_and_continues_in_request_order() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    set_test_central_root(&pool, &central_root).await;
    for skill_id in ["skill-a", "skill-b"] {
        let skill_dir = central_root.join(skill_id);
        write_test_skill_dir(&skill_dir);
        db::upsert_skill(
            &pool,
            &make_central_skill_at(skill_id, skill_id, &skill_dir),
        )
        .await
        .unwrap();
    }
    insert_pending_delete_collision(&pool, temp.path(), "skill-a").await;

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-a".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-b".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.failed.len(), 1);
    let failure = &result.failed[0];
    assert_eq!(failure.skill_id, "skill-a");
    assert_eq!(failure.phase.as_deref(), Some("recovery"));
    assert_eq!(
        failure.error_code.as_deref(),
        Some("central_operation.delete_restore_collision")
    );
    assert_eq!(
        failure.error_category.as_deref(),
        Some("central_skills.central_operation")
    );
    assert_eq!(failure.error, "This Central skill could not be deleted.");
    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "skill-b");
    assert!(central_root.join("skill-a").exists());
    assert!(!central_root.join("skill-b").exists());
    let serialized = serde_json::to_string(failure).unwrap();
    assert!(!serialized.contains(temp.path().to_string_lossy().as_ref()));
    assert!(!serialized.contains("manifest"));
}

#[tokio::test]
async fn batch_delete_recovers_selected_pending_update_before_deleting() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    set_test_central_root(&pool, &central_root).await;
    for skill_id in ["skill-with-update", "skill-clear"] {
        let skill_dir = central_root.join(skill_id);
        write_test_skill_dir(&skill_dir);
        db::upsert_skill(
            &pool,
            &make_central_skill_at(skill_id, skill_id, &skill_dir),
        )
        .await
        .unwrap();
    }
    let operation_id = insert_pending_update(&pool, &central_root, "skill-with-update").await;
    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-with-update".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-clear".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert!(result.failed.is_empty(), "{:?}", result.failed);
    assert_eq!(result.succeeded.len(), 2);
    assert_eq!(result.succeeded[0].skill_id, "skill-with-update");
    assert_eq!(result.succeeded[1].skill_id, "skill-clear");
    assert!(!central_root.join("skill-with-update").exists());
    assert!(!central_root.join("skill-clear").exists());
    let after = db::get_fs_db_operation(&pool, &operation_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.phase, "rolled_back");
    let completed_delete = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fs_db_operations
         WHERE skill_id = 'skill-with-update'
           AND operation_kind = 'central_delete'
           AND phase = 'completed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(completed_delete, 1);
}

#[tokio::test]
async fn batch_delete_reports_selected_update_recovery_collision_and_continues() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    set_test_central_root(&pool, &central_root).await;
    for skill_id in ["skill-with-update", "skill-clear"] {
        let skill_dir = central_root.join(skill_id);
        write_test_skill_dir(&skill_dir);
        db::upsert_skill(
            &pool,
            &make_central_skill_at(skill_id, skill_id, &skill_dir),
        )
        .await
        .unwrap();
    }
    let operation_id = insert_pending_update(&pool, &central_root, "skill-with-update").await;
    fs::write(
        central_root.join("skill-with-update").join("SKILL.md"),
        "changed after journal preparation",
    )
    .unwrap();

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-with-update".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "skill-clear".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.failed.len(), 1);
    let failure = &result.failed[0];
    assert_eq!(failure.skill_id, "skill-with-update");
    assert_eq!(failure.phase.as_deref(), Some("recovery"));
    assert_eq!(
        failure.error_code.as_deref(),
        Some("central_operation.update_rollback_target_fingerprint")
    );
    assert_eq!(
        failure.error_category.as_deref(),
        Some("central_updates.central_operation")
    );
    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "skill-clear");
    assert!(central_root.join("skill-with-update").exists());
    assert!(!central_root.join("skill-clear").exists());
    let pending = db::get_fs_db_operation(&pool, &operation_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.phase, "prepared");
    assert_eq!(
        pending.last_error_code.as_deref(),
        Some("update_rollback_target_fingerprint")
    );
}

#[tokio::test]
async fn fake_ssh_batch_delete_filters_pending_rows_and_reuses_one_target_connection() {
    use crate::targets::{
        ActiveTarget, ConnectedRemoteTarget, ConnectedSshTarget, RemoteTargetConfig, SshAuthMethod,
    };
    use crate::test_support::FakeRunner;

    let pool = crate::test_support::mem_pool_with_home("/home/alice").await;
    let target = RemoteTargetConfig {
        id: "ssh-delete-batch".to_string(),
        label: "SSH delete batch".to_string(),
        host: "example.invalid".to_string(),
        username: "alice".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: "/home/alice".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let active_target = ActiveTarget::Ssh(Box::new(target.clone()));
    let runner = Arc::new(FakeRunner::new());
    let connection = Arc::new(ConnectedRemoteTarget::Ssh(
        ConnectedSshTarget::for_tests_with_runner(target, runner.clone()),
    ));
    let selected_skill_id = "selected-remote";
    let selected_dir = "/home/alice/.skillsmanage/skills/selected-remote";
    db::upsert_skill(
        &pool,
        &make_remote_central_skill(selected_skill_id, selected_dir),
    )
    .await
    .unwrap();
    let unrelated_operation =
        insert_remote_pending_delete_collision(&pool, active_target.id(), "unrelated-remote").await;
    let before = db::get_fs_db_operation(&pool, &unrelated_operation)
        .await
        .unwrap()
        .unwrap();

    let digest = "a".repeat(64);
    runner.push_success("");
    runner.push_success(&digest);
    runner.push_success("STAGED\n");
    runner.push_success(&digest);
    runner.push_success("FINALIZED\n");

    let result = super::delete::delete_central_skills_for_target_with_connection_for_tests(
        &pool,
        &active_target,
        Arc::clone(&connection),
        &[BatchDeleteCentralSkillRequest {
            skill_id: selected_skill_id.to_string(),
            remove_agent_ids: Vec::new(),
        }],
        Some("remote-delete-batch"),
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, selected_skill_id);
    assert!(result.failed.is_empty());
    assert_eq!(connection.target_id(), active_target.id());
    assert_eq!(runner.calls().len(), 5);
    let after = db::get_fs_db_operation(&pool, &unrelated_operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.phase, before.phase);
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(after.last_error_code, before.last_error_code);
    assert_eq!(after.last_error_message, before.last_error_message);

    let selected_operation = sqlx::query_as::<_, (String, String)>(
        "SELECT target_id, target_kind FROM fs_db_operations WHERE skill_id = ?",
    )
    .bind(selected_skill_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        selected_operation,
        (active_target.id().to_string(), "ssh".to_string())
    );
}

#[tokio::test]
async fn fake_ssh_and_wsl_delete_recover_selected_update_with_one_connection() {
    use crate::targets::{
        ActiveTarget, ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget,
        RemoteTargetConfig, SshAuthMethod, WslTargetConfig,
    };
    use crate::test_support::FakeRunner;

    let ssh_runner = Arc::new(FakeRunner::new());
    let ssh_target = RemoteTargetConfig {
        id: "ssh-delete-update-recovery".to_string(),
        label: "SSH delete update recovery".to_string(),
        host: "example.invalid".to_string(),
        username: "alice".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: "/home/alice".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let ssh_active = ActiveTarget::Ssh(Box::new(ssh_target.clone()));
    let ssh_connection = Arc::new(ConnectedRemoteTarget::Ssh(
        ConnectedSshTarget::for_tests_with_runner(ssh_target, ssh_runner.clone()),
    ));

    let wsl_runner = Arc::new(FakeRunner::new());
    let wsl_target = WslTargetConfig {
        id: "wsl-delete-update-recovery".to_string(),
        label: "WSL delete update recovery".to_string(),
        distribution: "TestDistro".to_string(),
        remote_home: "/home/alice".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let wsl_active = ActiveTarget::Wsl(Box::new(wsl_target.clone()));
    let wsl_connection = Arc::new(ConnectedRemoteTarget::Wsl(
        ConnectedWslTarget::for_tests_with_runner(wsl_target, wsl_runner.clone()),
    ));

    for (active_target, connection, runner) in [
        (ssh_active, ssh_connection, ssh_runner),
        (wsl_active, wsl_connection, wsl_runner),
    ] {
        let pool = crate::test_support::mem_pool_with_home("/home/alice").await;
        let skill_id = "selected-update";
        let selected_dir = "/home/alice/.skillsmanage/skills/selected-update";
        db::upsert_skill(&pool, &make_remote_central_skill(skill_id, selected_dir))
            .await
            .unwrap();
        let operation_id = insert_remote_pending_update(
            &pool,
            active_target.id(),
            match active_target.kind() {
                crate::targets::TargetKind::Ssh => "ssh",
                crate::targets::TargetKind::Wsl => "wsl",
                crate::targets::TargetKind::Local => unreachable!(),
            },
            skill_id,
        )
        .await;

        runner.push_output(1, "", "");
        runner.push_success("");
        runner.push_output(1, "", "");
        runner.push_success(&format!("ROOT\t{selected_dir}\nEND\t{selected_dir}\n"));
        runner.push_success("ROLLED_BACK\n");
        let digest = "a".repeat(64);
        runner.push_success("");
        runner.push_success(&digest);
        runner.push_success("STAGED\n");
        runner.push_success(&digest);
        runner.push_success("FINALIZED\n");

        let result = super::delete::delete_central_skills_for_target_with_connection_for_tests(
            &pool,
            &active_target,
            Arc::clone(&connection),
            &[BatchDeleteCentralSkillRequest {
                skill_id: skill_id.to_string(),
                remove_agent_ids: Vec::new(),
            }],
            Some("remote-delete-update-recovery"),
        )
        .await
        .unwrap();

        assert!(result.failed.is_empty(), "{:?}", result.failed);
        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(connection.target_id(), active_target.id());
        assert_eq!(runner.calls().len(), 10);
        assert_eq!(
            db::get_fs_db_operation(&pool, &operation_id)
                .await
                .unwrap()
                .unwrap()
                .phase,
            "rolled_back"
        );
        let delete_row = sqlx::query_as::<_, (String, String)>(
            "SELECT target_id, target_kind FROM fs_db_operations
             WHERE skill_id = ? AND operation_kind = 'central_delete'",
        )
        .bind(skill_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(delete_row.0, active_target.id());
        let expected_kind = match active_target.kind() {
            crate::targets::TargetKind::Ssh => "ssh",
            crate::targets::TargetKind::Wsl => "wsl",
            crate::targets::TargetKind::Local => unreachable!(),
        };
        assert_eq!(delete_row.1, expected_kind);
    }
}

#[tokio::test]
async fn test_batch_delete_journal_rows_share_one_batch_id() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    fs::create_dir_all(&central_root).unwrap();
    set_test_central_root(&pool, &central_root).await;
    for skill_id in ["batch-delete-a", "batch-delete-b"] {
        let central_dir = central_root.join(skill_id);
        write_test_skill_dir(&central_dir);
        db::upsert_skill(
            &pool,
            &make_central_skill_at(skill_id, skill_id, &central_dir),
        )
        .await
        .unwrap();
    }

    let result = delete_central_skills_impl(
        &pool,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "batch-delete-a".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "batch-delete-b".to_string(),
                remove_agent_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(result.succeeded.len(), 2);
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT skill_id, batch_id FROM fs_db_operations ORDER BY skill_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, rows[1].1);
    assert!(rows[0].1.is_some());
}

#[tokio::test]
async fn test_delete_skill_repository_removes_repository_skills_and_record() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let central_dir = central_root.join("repo-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&central_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-delete", "Repo Delete", &central_dir),
    )
    .await
    .unwrap();
    let repository = db::create_or_update_skill_repository(
        &pool,
        Some("github-repo-delete"),
        "owner/repo-delete",
        "github",
        Some("owner"),
        Some("repo-delete"),
        Some("main"),
        Some("https://github.com/owner/repo-delete"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        &pool,
        &repository.id,
        &["repo-delete".to_string()],
        Some("skills/repo-delete"),
    )
    .await
    .unwrap();

    let preview = preview_delete_skill_repository_impl(&pool, &repository.id)
        .await
        .unwrap();
    assert_eq!(preview.repository.skill_count, 1);
    assert_eq!(preview.delete_preview.previews.len(), 1);

    let result = delete_skill_repository_impl(&pool, &repository.id, &[])
        .await
        .unwrap();

    assert!(result.deleted_repository);
    assert!(result.delete_result.failed.is_empty());
    assert_eq!(result.delete_result.succeeded.len(), 1);
    assert!(!central_dir.exists());
    assert!(db::get_skill_by_id(&pool, "repo-delete")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_delete_skill_repository_rejects_unknown_repository() {
    let pool = setup_test_db().await;

    let error = delete_skill_repository_impl(&pool, db::LOCAL_UNKNOWN_REPOSITORY_ID, &[])
        .await
        .unwrap_err();

    assert!(error.to_string().contains("cannot be deleted"));
    assert!(
        db::get_skill_repository_by_id(&pool, db::LOCAL_UNKNOWN_REPOSITORY_ID)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_delete_skill_repository_keeps_record_on_partial_failure() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let valid_dir = central_root.join("repo-valid-delete");
    let outside_dir = temp.path().join("outside").join("repo-unsafe-delete");
    fs::create_dir_all(&central_root).unwrap();
    write_test_skill_dir(&valid_dir);
    write_test_skill_dir(&outside_dir);
    set_test_central_root(&pool, &central_root).await;

    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-valid-delete", "Repo Valid Delete", &valid_dir),
    )
    .await
    .unwrap();
    db::upsert_skill(
        &pool,
        &make_central_skill_at("repo-unsafe-delete", "Repo Unsafe Delete", &outside_dir),
    )
    .await
    .unwrap();
    let repository = db::create_or_update_skill_repository(
        &pool,
        Some("github-repo-partial"),
        "owner/repo-partial",
        "github",
        Some("owner"),
        Some("repo-partial"),
        Some("main"),
        Some("https://github.com/owner/repo-partial"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        &pool,
        &repository.id,
        &[
            "repo-valid-delete".to_string(),
            "repo-unsafe-delete".to_string(),
        ],
        Some("skills/repo-partial"),
    )
    .await
    .unwrap();

    let result = delete_skill_repository_impl(&pool, &repository.id, &[])
        .await
        .unwrap();

    assert!(!result.deleted_repository);
    assert_eq!(result.delete_result.succeeded.len(), 1);
    assert_eq!(result.delete_result.failed.len(), 1);
    assert!(!valid_dir.exists());
    assert!(outside_dir.exists());
    assert!(db::get_skill_repository_by_id(&pool, &repository.id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_get_skill_detail_returns_installations() {
    let pool = setup_test_db().await;

    let skill = make_skill("detail-skill", "Detail Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let now = Utc::now().to_rfc3339();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "detail-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/detail-skill/SKILL.md".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: now.clone(),
        },
    )
    .await
    .unwrap();

    let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
    assert_eq!(detail.id, "detail-skill");
    assert_eq!(detail.installations.len(), 1);
    assert_eq!(detail.installations[0].agent_id, "claude-code");
    // installed_at should be populated from created_at
    assert!(
        !detail.installations[0].installed_at.is_empty(),
        "installed_at must be set"
    );
    assert!(
        detail.collections.is_empty(),
        "skill should have no collections by default"
    );
}

#[tokio::test]
async fn test_get_skill_detail_returns_collections() {
    let pool = setup_test_db().await;

    let skill = make_skill("detail-skill", "Detail Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    let alpha = db::create_collection(&pool, "Alpha", Some("First collection"))
        .await
        .unwrap();
    let beta = db::create_collection(&pool, "Beta", None).await.unwrap();

    db::add_skill_to_collection(&pool, &alpha.id, "detail-skill")
        .await
        .unwrap();
    db::add_skill_to_collection(&pool, &beta.id, "detail-skill")
        .await
        .unwrap();

    let detail = get_skill_detail_impl(&pool, "detail-skill").await.unwrap();
    let collection_names: Vec<&str> = detail.collections.iter().map(|c| c.name.as_str()).collect();

    assert_eq!(collection_names, vec!["Alpha", "Beta"]);
}

#[tokio::test]
async fn test_get_skill_detail_not_found() {
    let pool = setup_test_db().await;
    let result = get_skill_detail_impl(&pool, "nonexistent").await;
    assert!(result.is_err(), "should error for unknown skill_id");
}

// ── read_skill_content ────────────────────────────────────────────────────

#[tokio::test]
async fn test_read_skill_content_returns_file_content() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let skill_dir = tmp.path().join("my-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    let skill_md_path = skill_dir.join("SKILL.md");
    let expected_content = "---\nname: My Skill\n---\n\n# My Skill\n\nContent here.";
    fs::write(&skill_md_path, expected_content).unwrap();

    let skill = Skill {
        id: "my-skill".to_string(),
        uid: "my-skill-uid".to_string(),
        name: "My Skill".to_string(),
        description: None,
        file_path: skill_md_path.to_string_lossy().into_owned(),
        canonical_path: None,
        is_central: false,
        source: None,
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let content = read_skill_content_impl(&pool, "my-skill").await.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_skill_content_file_not_found() {
    let pool = setup_test_db().await;

    let skill = Skill {
        id: "missing-file-skill".to_string(),
        uid: "missing-file-skill-uid".to_string(),
        name: "Missing File".to_string(),
        description: None,
        file_path: "/nonexistent/SKILL.md".to_string(),
        canonical_path: None,
        is_central: false,
        source: None,
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let result = read_skill_content_impl(&pool, "missing-file-skill").await;
    assert!(result.is_err(), "should error when file does not exist");
}

// ── Testable core implementations (without Tauri State) ───────────────────

async fn get_central_skills_impl(
    pool: &SqlitePool,
) -> Result<Vec<SkillWithLinks>, CentralSkillsError> {
    super::get_central_skills_impl(pool).await
}

async fn get_skill_detail_impl(
    pool: &SqlitePool,
    skill_id: &str,
) -> Result<SkillDetail, CentralSkillsError> {
    super::get_skill_detail_with_row_impl(pool, skill_id, None, None).await
}

async fn read_skill_content_impl(pool: &SqlitePool, skill_id: &str) -> Result<String, String> {
    let skill = db::get_skill_by_id(pool, skill_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;
    std::fs::read_to_string(&skill.file_path)
        .map_err(|e| format!("Failed to read '{}': {}", skill.file_path, e))
}

// ── Regression: get_skills_by_agent_impl returns installation metadata ─────

/// `get_skills_by_agent_impl` must return `SkillForAgent` objects that
/// include `link_type`, `dir_path`, and `symlink_target` from the
/// installation record so the frontend `SkillCard` can show the correct
/// source indicator.
#[tokio::test]
async fn test_get_skills_by_agent_impl_includes_installation_metadata() {
    let pool = setup_test_db().await;

    let skill = make_skill("meta-skill", "Meta Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    let installed_at = Utc::now().to_rfc3339();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "meta-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/claude/meta-skill".to_string(),
            link_type: "symlink".to_string(),
            symlink_target: Some("/tmp/central/meta-skill".to_string()),
            created_at: installed_at.clone(),
        },
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(skills.len(), 1, "should find one skill for claude-code");

    let s = &skills[0];
    assert_eq!(s.id, "meta-skill");
    assert_eq!(
        s.link_type, "symlink",
        "link_type must come from installation record"
    );
    assert_eq!(
        s.dir_path, "/tmp/claude/meta-skill",
        "dir_path must be installed_path from installation record"
    );
    assert_eq!(
        s.symlink_target.as_deref(),
        Some("/tmp/central/meta-skill"),
        "symlink_target must be forwarded from installation record"
    );
    assert_eq!(
        s.installed_at.as_deref(),
        Some(installed_at.as_str()),
        "installed_at must expose skill_installations.created_at"
    );
    assert_eq!(s.scanned_at, skill.scanned_at);
    assert_eq!(
        s.created_at.as_deref(),
        Some(skill.scanned_at.as_str()),
        "missing filesystem metadata falls back to scanned_at"
    );
    assert_eq!(
        s.updated_at.as_deref(),
        Some(skill.scanned_at.as_str()),
        "missing filesystem metadata falls back to scanned_at"
    );
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_includes_repository_metadata_for_installations() {
    let pool = setup_test_db().await;

    let skill = make_skill("repo-skill", "Repo Skill", true);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "repo-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: "/tmp/cursor/repo-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();
    let repository = db::create_or_update_skill_repository(
        &pool,
        Some("github-owner-repo-main"),
        "owner/repo",
        "github",
        Some("owner"),
        Some("repo"),
        Some("main"),
        Some("https://github.com/owner/repo"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        &pool,
        &repository.id,
        &["repo-skill".to_string()],
        Some("skills/repo-skill"),
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "cursor").await.unwrap();
    assert_eq!(skills.len(), 1);
    let row = &skills[0];
    let repository = row.repository.as_ref().expect("repository metadata");
    assert_eq!(repository.id, "github-owner-repo-main");
    assert_eq!(repository.name, "owner/repo");
    assert_eq!(row.source_path.as_deref(), Some("skills/repo-skill"));
    assert!(!row.is_source_unknown);
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_empty_for_unknown_agent() {
    let pool = setup_test_db().await;
    let skills = get_skills_by_agent_impl(&pool, "nobody").await.unwrap();
    assert!(
        skills.is_empty(),
        "no skills for an agent with no installations"
    );
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_claude_uses_observations_for_duplicate_rows() {
    let pool = setup_test_db().await;

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

    assert_eq!(
        skills.len(),
        2,
        "Claude queries should surface duplicate logical skills from different sources"
    );
    assert_eq!(skills[0].id, "shared-skill");
    assert_eq!(skills[1].id, "shared-skill");
    assert_ne!(skills[0].dir_path, skills[1].dir_path);
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_claude_includes_source_identity_and_conflict_grouping() {
    let pool = setup_test_db().await;

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let mut skills = get_skills_by_agent_impl(&pool, "claude-code")
        .await
        .unwrap();
    skills.sort_by(|a, b| a.dir_path.cmp(&b.dir_path));

    assert_eq!(skills.len(), 2);
    assert_eq!(
        skills[0].row_id,
        "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        skills[1].row_id,
        "claude-code::/tmp/.claude/skills/shared-skill"
    );
    assert_eq!(skills[0].source_kind.as_deref(), Some("plugin"));
    assert_eq!(skills[1].source_kind.as_deref(), Some("user"));
    assert_eq!(
        skills[0].source_root.as_deref(),
        Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
    );
    assert_eq!(
        skills[1].source_root.as_deref(),
        Some("/tmp/.claude/skills")
    );
    assert!(skills[0].is_read_only);
    assert!(!skills[1].is_read_only);
    assert_eq!(
        skills[0].conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert_eq!(
        skills[1].conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert_eq!(skills[0].conflict_count, 2);
    assert_eq!(skills[1].conflict_count, 2);
    assert!(skills[0].installed_at.is_none());
    assert_eq!(skills[0].created_at, Some(skills[0].scanned_at.clone()));
    assert_eq!(skills[0].updated_at, Some(skills[0].scanned_at.clone()));
    assert!(skills[0].repository.is_none());
}

#[tokio::test]
async fn test_get_skill_detail_with_row_impl_claude_plugin_row_uses_selected_observation() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
    db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
        .await
        .unwrap();

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let detail = get_skill_detail_with_row_impl(
        &pool,
        "shared-skill",
        Some("claude-code"),
        Some("claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"),
    )
    .await
    .unwrap();

    assert_eq!(
        detail.row_id,
        "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        detail.dir_path,
        "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill"
    );
    assert_eq!(
        detail.file_path,
        "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill/SKILL.md"
    );
    assert_eq!(detail.source_kind.as_deref(), Some("plugin"));
    assert_eq!(
        detail.source_root.as_deref(),
        Some("/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0")
    );
    assert!(detail.is_read_only);
    assert_eq!(detail.conflict_count, 2);
    assert_eq!(
        detail.conflict_group.as_deref(),
        Some("claude-code::shared-skill")
    );
    assert!(
        detail.installations.is_empty(),
        "plugin detail should not expose manageable installations"
    );
    assert!(
        detail.collections.is_empty(),
        "plugin detail should not expose collection management state"
    );
}

#[tokio::test]
async fn test_get_skill_detail_with_row_impl_claude_user_row_keeps_manageable_state() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "claude-code".to_string(),
            installed_path: "/tmp/.claude/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let collection = db::create_collection(&pool, "Alpha", None).await.unwrap();
    db::add_skill_to_collection(&pool, &collection.id, "shared-skill")
        .await
        .unwrap();

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/skills/shared-skill",
            "user",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code::/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let detail = get_skill_detail_with_row_impl(
        &pool,
        "shared-skill",
        Some("claude-code"),
        Some("claude-code::/tmp/.claude/skills/shared-skill"),
    )
    .await
    .unwrap();

    assert_eq!(
        detail.row_id,
        "claude-code::/tmp/.claude/skills/shared-skill"
    );
    assert_eq!(detail.dir_path, "/tmp/.claude/skills/shared-skill");
    assert_eq!(detail.source_kind.as_deref(), Some("user"));
    assert!(!detail.is_read_only);
    assert_eq!(detail.conflict_count, 2);
    assert_eq!(detail.installations.len(), 1);
    assert_eq!(detail.collections.len(), 1);
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_codex_merges_platform_and_plugin_rows() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "codex".to_string(),
            installed_path: "/tmp/.agents/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation_for_agent(
            "codex",
            "codex::/tmp/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill",
            "shared-skill",
            "Shared Skill",
            "/tmp/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "codex").await.unwrap();
    let platform_row = skills
        .iter()
        .find(|skill| !skill.is_read_only)
        .expect("platform row");
    let plugin_row = skills
        .iter()
        .find(|skill| skill.is_read_only)
        .expect("plugin row");

    assert_eq!(skills.len(), 2);
    assert_eq!(platform_row.row_id, "shared-skill");
    assert_eq!(platform_row.source_kind, None);
    assert!(platform_row.installed_at.is_some());
    assert_eq!(plugin_row.source_kind.as_deref(), Some("plugin"));
    assert!(plugin_row.installed_at.is_none());
    assert_eq!(plugin_row.created_at, Some(plugin_row.scanned_at.clone()));
    assert_eq!(plugin_row.updated_at, Some(plugin_row.scanned_at.clone()));
    assert!(plugin_row.repository.is_none());
    assert_eq!(platform_row.conflict_count, 2);
    assert_eq!(plugin_row.conflict_count, 2);
    assert_eq!(
        platform_row.conflict_group.as_deref(),
        Some("codex::shared-skill")
    );
    assert_eq!(
        plugin_row.conflict_group.as_deref(),
        Some("codex::shared-skill")
    );
}

#[tokio::test]
async fn test_get_skill_detail_with_row_impl_codex_plugin_row_uses_selected_observation() {
    let pool = setup_test_db().await;

    let skill = make_skill("shared-skill", "Shared Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "shared-skill".to_string(),
            agent_id: "codex".to_string(),
            installed_path: "/tmp/.agents/skills/shared-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let row_id = "codex::/tmp/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill";
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation_for_agent(
            "codex",
            row_id,
            "shared-skill",
            "Shared Skill",
            "/tmp/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill",
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let detail = get_skill_detail_with_row_impl(&pool, "shared-skill", Some("codex"), Some(row_id))
        .await
        .unwrap();

    assert_eq!(detail.row_id, row_id);
    assert_eq!(detail.source_kind.as_deref(), Some("plugin"));
    assert_eq!(
        detail.source_root.as_deref(),
        Some("/tmp/.codex/plugins/cache/openai/example/1.0.0")
    );
    assert!(detail.is_read_only);
    assert_eq!(detail.conflict_count, 2);
    assert_eq!(
        detail.conflict_group.as_deref(),
        Some("codex::shared-skill")
    );
    assert!(detail.installations.is_empty());
    assert!(detail.collections.is_empty());
}

#[tokio::test]
async fn test_get_skills_by_agent_impl_copy_link_type() {
    let pool = setup_test_db().await;

    let skill = make_skill("copy-skill", "Copy Skill", false);
    db::upsert_skill(&pool, &skill).await.unwrap();

    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "copy-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: "/tmp/cursor/copy-skill".to_string(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let skills = get_skills_by_agent_impl(&pool, "cursor").await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "copy");
    assert!(
        skills[0].symlink_target.is_none(),
        "copy skills have no symlink target"
    );
}

// ── read_file_by_path ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_read_file_by_path_success() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    let file_path = skill_dir.join("SKILL.md");
    let content = "---\nname: Test\n---\n\n# Test Skill";
    fs::write(&file_path, content).unwrap();

    let result = read_file_by_path_impl(&file_path.to_string_lossy(), &skill_dir.to_string_lossy());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
}

#[tokio::test]
async fn test_read_file_by_path_not_found() {
    let tmp = TempDir::new().unwrap();
    let result = read_file_by_path_impl("/nonexistent/file.md", &tmp.path().to_string_lossy());
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_directory_tree_reads_nested_local_structure() {
    let tmp = TempDir::new().unwrap();
    make_directory_tree_fixture(tmp.path());

    let result = super::files::list_directory_tree_impl(
        &tmp.path().to_string_lossy(),
        &tmp.path().to_string_lossy(),
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "examples");
    assert_eq!(result[0].file_type, "dir");
    assert_eq!(result[0].children.len(), 2);
    assert_eq!(result[1].name, "SKILL.md");
    assert_eq!(result[1].file_type, "file");
}

#[tokio::test]
async fn test_list_directory_tree_rejects_missing_path() {
    let tmp = TempDir::new().unwrap();
    let result = super::files::list_directory_tree_impl(
        "/definitely/missing/path",
        &tmp.path().to_string_lossy(),
    );
    assert!(result.is_err());
}

// ── open_in_file_manager ───────────────────────────────────────────────────

#[tokio::test]
async fn test_open_in_file_manager_nonexistent_path() {
    let tmp = TempDir::new().unwrap();
    let result = open_in_file_manager_checked_impl(
        "/nonexistent/path/that/does/not/exist",
        &tmp.path().to_string_lossy(),
    );
    assert!(result.is_err());
}
