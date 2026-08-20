use super::*;
use crate::db::{self, SkillInstallation, SkillUpdateInventoryEntry, SkillUpdateInventoryRun};
use crate::targets::ActiveTarget;
use crate::test_support::{
    file_pool, mem_pool, mem_pool_with_home, seed_central_skill, set_agent_dir, write_skill_md,
};
use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;

fn inventory_run(now: &str) -> SkillUpdateInventoryRun {
    SkillUpdateInventoryRun {
        inventory_id: "all:sync".to_string(),
        scope_kind: "all".to_string(),
        mode: "sync".to_string(),
        skill_ids_json: None,
        repository_ids_json: None,
        agent_ids_json: None,
        cache_policy: "bypass".to_string(),
        generated_at: now.to_string(),
    }
}

fn unsupported_entry(skill_id: &str, now: &str) -> SkillUpdateInventoryEntry {
    SkillUpdateInventoryEntry {
        inventory_id: "all:sync".to_string(),
        bucket: "unsupported".to_string(),
        entity_key: skill_id.to_string(),
        skill_id: Some(skill_id.to_string()),
        skill_name: Some(skill_id.to_string()),
        repository_id: None,
        source_type: None,
        source_url: None,
        ref_name: None,
        source_path: None,
        agent_id: None,
        local_hash: None,
        baseline_hash: None,
        remote_hash: None,
        local_version: None,
        remote_version: None,
        cache_policy: "bypass".to_string(),
        cache_hit: false,
        snapshot_fetched_at: None,
        generated_at: now.to_string(),
        payload_json: r#"{"reasonCode":"unknown_source"}"#.to_string(),
        error: None,
    }
}

async fn seed_unsupported_inventory(pool: &db::DbPool, skill_ids: &[&str]) {
    let now = Utc::now().to_rfc3339();
    let entries: Vec<_> = skill_ids
        .iter()
        .map(|skill_id| unsupported_entry(skill_id, &now))
        .collect();
    db::replace_skill_update_inventory(pool, &inventory_run(&now), &entries)
        .await
        .unwrap();
}

async fn assign_github_membership(pool: &db::DbPool, skill_id: &str) {
    let repository = db::create_or_update_skill_repository(
        pool,
        Some("github:owner-skills-main"),
        "owner/skills",
        "github",
        Some("owner"),
        Some("skills"),
        Some("main"),
        Some("https://github.com/owner/skills"),
        false,
    )
    .await
    .unwrap();
    db::assign_skills_to_repository(
        pool,
        &repository.id,
        &[skill_id.to_string()],
        Some("skills/github-skill"),
    )
    .await
    .unwrap();
}

async fn inventory_entry_count(pool: &db::DbPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM skill_update_inventory_entries")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn remote_central_skill(id: &str, dir: &str) -> crate::db::Skill {
    crate::db::Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: id.to_string(),
        description: Some(format!("Desc for {id}")),
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

async fn pending_addition_count(pool: &db::DbPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM skill_repository_pending_additions")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn list_unknown_source_ids_excludes_membership_and_non_central() {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let unknown_dir = temp.path().join("npx-skill");
    let github_dir = temp.path().join("github-skill");
    let platform_dir = temp.path().join("platform-only");
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    seed_central_skill(&pool, &github_dir, "github-skill", "github backed").await;
    assign_github_membership(&pool, "github-skill").await;

    write_skill_md(&platform_dir, "platform-only", Some("not central"));
    let mut platform = crate::test_support::central_skill_row("platform-only", &platform_dir);
    platform.is_central = false;
    db::upsert_skill(&pool, &platform).await.unwrap();

    let ids = list_unknown_source_central_skill_ids(&pool).await.unwrap();
    assert_eq!(ids, vec!["npx-skill".to_string()]);
}

#[tokio::test]
async fn preview_reset_is_empty_when_every_central_skill_has_membership() {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let github_dir = temp.path().join("github-skill");
    seed_central_skill(&pool, &github_dir, "github-skill", "github backed").await;
    assign_github_membership(&pool, "github-skill").await;
    set_agent_dir(&pool, "central", temp.path()).await;

    let preview = preview_reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local)
        .await
        .unwrap();
    assert!(preview.skill_ids.is_empty());
    assert!(preview.preview.previews.is_empty());
    assert!(preview.preview.failed.is_empty());
    assert!(github_dir.exists());
}

#[tokio::test]
async fn local_preview_does_not_mutate_fs_or_db() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    let github_dir = central_root.join("github-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    seed_central_skill(&pool, &github_dir, "github-skill", "github backed").await;
    assign_github_membership(&pool, "github-skill").await;
    seed_unsupported_inventory(&pool, &["npx-skill", "github-skill"]).await;

    let preview = preview_reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local)
        .await
        .unwrap();
    assert_eq!(preview.skill_ids, vec!["npx-skill".to_string()]);
    assert_eq!(preview.preview.previews.len(), 1);
    assert_eq!(preview.preview.previews[0].skill_id, "npx-skill");
    assert!(unknown_dir.exists());
    assert!(github_dir.exists());
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_some());
    assert_eq!(inventory_entry_count(&pool).await, 2);
}

#[tokio::test]
async fn local_apply_deletes_only_unknown_source_and_clears_inventory() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    let github_dir = central_root.join("github-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    seed_central_skill(&pool, &github_dir, "github-skill", "github backed").await;
    assign_github_membership(&pool, "github-skill").await;
    seed_unsupported_inventory(&pool, &["npx-skill", "github-skill"]).await;
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-skills-main".to_string(),
            source_path: "skills/extra".to_string(),
            skill_id: "extra".to_string(),
            skill_name: "extra".to_string(),
            conflict_existing_skill_id: None,
            resolved_commit_sha: None,
            snapshot_digest: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let result = reset_unknown_source_skills_impl(
        &pool,
        &ActiveTarget::Local,
        &["npx-skill".to_string()],
        &[],
    )
    .await
    .unwrap();
    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "npx-skill");
    assert!(result.failed.is_empty());
    assert!(!unknown_dir.exists());
    assert!(github_dir.exists());
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "github-skill")
        .await
        .unwrap()
        .is_some());
    assert_eq!(inventory_entry_count(&pool).await, 0);
    assert_eq!(pending_addition_count(&pool).await, 0);
}

#[tokio::test]
async fn local_apply_keeps_copy_installs_by_default() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    let copy_dir = temp.path().join("cursor").join("npx-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    set_agent_dir(&pool, "cursor", &temp.path().join("cursor")).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    write_skill_md(&copy_dir, "npx-skill", Some("npx leftover"));
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "npx-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: copy_dir.to_string_lossy().into_owned(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local, &["npx-skill".to_string()], &[])
        .await
        .unwrap();
    assert!(!unknown_dir.exists());
    assert!(copy_dir.exists());
}

#[tokio::test]
async fn local_apply_removes_selected_copy_installs() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    let copy_dir = temp.path().join("cursor").join("npx-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    set_agent_dir(&pool, "cursor", &temp.path().join("cursor")).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    write_skill_md(&copy_dir, "npx-skill", Some("npx leftover"));
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "npx-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: copy_dir.to_string_lossy().into_owned(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    reset_unknown_source_skills_impl(
        &pool,
        &ActiveTarget::Local,
        &["npx-skill".to_string()],
        &["cursor".to_string()],
    )
    .await
    .unwrap();
    assert!(!unknown_dir.exists());
    assert!(!copy_dir.exists());
}

#[tokio::test]
async fn reset_on_pool_a_does_not_mutate_pool_b() {
    let (pool_a, _db_a) = file_pool().await;
    let (pool_b, _db_b) = file_pool().await;
    let temp_a = TempDir::new().unwrap();
    let temp_b = TempDir::new().unwrap();
    let central_a = temp_a.path().join("central");
    let central_b = temp_b.path().join("central");
    let unknown_a = central_a.join("npx-skill");
    let unknown_b = central_b.join("npx-skill");
    set_agent_dir(&pool_a, "central", &central_a).await;
    set_agent_dir(&pool_b, "central", &central_b).await;
    seed_central_skill(&pool_a, &unknown_a, "npx-skill", "pool a").await;
    seed_central_skill(&pool_b, &unknown_b, "npx-skill", "pool b").await;
    seed_unsupported_inventory(&pool_a, &["npx-skill"]).await;
    seed_unsupported_inventory(&pool_b, &["npx-skill"]).await;

    reset_unknown_source_skills_impl(
        &pool_a,
        &ActiveTarget::Local,
        &["npx-skill".to_string()],
        &[],
    )
    .await
    .unwrap();

    assert!(!unknown_a.exists());
    assert!(unknown_b.exists());
    assert!(db::get_skill_by_id(&pool_b, "npx-skill")
        .await
        .unwrap()
        .is_some());
    assert_eq!(inventory_entry_count(&pool_a).await, 0);
    assert_eq!(inventory_entry_count(&pool_b).await, 1);
}

#[tokio::test]
async fn fake_ssh_reset_deletes_only_unknown_source_and_leaves_local_files() {
    use crate::targets::{
        ConnectedRemoteTarget, ConnectedSshTarget, RemoteTargetConfig, SshAuthMethod,
    };
    use crate::test_support::FakeRunner;

    let pool = mem_pool_with_home("/home/alice").await;
    let local_sentinel = TempDir::new().unwrap();
    let local_file = local_sentinel.path().join("do-not-touch.txt");
    std::fs::write(&local_file, "local").unwrap();

    db::upsert_skill(
        &pool,
        &remote_central_skill("npx-skill", "/home/alice/.skillsmanage/skills/npx-skill"),
    )
    .await
    .unwrap();
    db::upsert_skill(
        &pool,
        &remote_central_skill(
            "github-skill",
            "/home/alice/.skillsmanage/skills/github-skill",
        ),
    )
    .await
    .unwrap();
    assign_github_membership(&pool, "github-skill").await;
    seed_unsupported_inventory(&pool, &["npx-skill"]).await;

    let target = RemoteTargetConfig {
        id: "ssh-unknown-source-reset".to_string(),
        label: "SSH unknown source reset".to_string(),
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
    let digest = "a".repeat(64);
    runner.push_success("");
    runner.push_success(&digest);
    runner.push_success("STAGED\n");
    runner.push_success(&digest);
    runner.push_success("FINALIZED\n");

    let preview = preview_reset_unknown_source_skills_impl(&pool, &active_target)
        .await
        .unwrap();
    assert_eq!(preview.skill_ids, vec!["npx-skill".to_string()]);

    let result = super::delete::reset_unknown_source_skills_for_target_with_connection_for_tests(
        &pool,
        &active_target,
        connection,
        &["npx-skill".to_string()],
        &[],
    )
    .await
    .unwrap();
    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "npx-skill");
    assert!(result.failed.is_empty());
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "github-skill")
        .await
        .unwrap()
        .is_some());
    assert_eq!(inventory_entry_count(&pool).await, 0);
    assert_eq!(runner.calls().len(), 5);
    assert!(local_file.exists());
}

#[tokio::test]
async fn empty_apply_clears_stale_inventory_without_deleting_github_skills() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let github_dir = temp.path().join("central").join("github-skill");
    set_agent_dir(&pool, "central", &temp.path().join("central")).await;
    seed_central_skill(&pool, &github_dir, "github-skill", "github backed").await;
    assign_github_membership(&pool, "github-skill").await;
    seed_unsupported_inventory(&pool, &["stale-unsupported"]).await;

    let result = reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local, &[], &[])
        .await
        .unwrap();
    assert!(result.succeeded.is_empty());
    assert!(result.failed.is_empty());
    assert!(github_dir.exists());
    assert_eq!(inventory_entry_count(&pool).await, 0);
}

#[tokio::test]
async fn apply_skips_confirmed_ids_that_are_not_previewable() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    let broken_dir = temp.path().join("outside").join("broken-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    seed_central_skill(&pool, &broken_dir, "broken-skill", "outside central").await;
    seed_unsupported_inventory(&pool, &["npx-skill", "broken-skill"]).await;

    let preview = preview_reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local)
        .await
        .unwrap();
    assert_eq!(preview.skill_ids, vec!["broken-skill", "npx-skill"]);
    assert_eq!(preview.preview.previews.len(), 1);
    assert_eq!(preview.preview.previews[0].skill_id, "npx-skill");
    assert_eq!(preview.preview.failed.len(), 1);
    assert_eq!(preview.preview.failed[0].skill_id, "broken-skill");

    let result = reset_unknown_source_skills_impl(
        &pool,
        &ActiveTarget::Local,
        &["npx-skill".to_string(), "broken-skill".to_string()],
        &[],
    )
    .await
    .unwrap();
    assert_eq!(result.succeeded.len(), 1);
    assert_eq!(result.succeeded[0].skill_id, "npx-skill");
    assert!(result.failed.is_empty());
    assert!(!unknown_dir.exists());
    assert!(broken_dir.exists());
    assert!(db::get_skill_by_id(&pool, "broken-skill")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn apply_skips_confirmed_id_that_gained_membership() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;
    assign_github_membership(&pool, "npx-skill").await;
    seed_unsupported_inventory(&pool, &["npx-skill"]).await;

    let result = reset_unknown_source_skills_impl(
        &pool,
        &ActiveTarget::Local,
        &["npx-skill".to_string()],
        &[],
    )
    .await
    .unwrap();
    assert!(result.succeeded.is_empty());
    assert!(result.failed.is_empty());
    assert!(unknown_dir.exists());
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_some());
    assert_eq!(inventory_entry_count(&pool).await, 0);
}

#[tokio::test]
async fn deleted_skill_id_can_be_reimported_with_membership() {
    let (pool, _db_dir) = file_pool().await;
    let temp = TempDir::new().unwrap();
    let central_root = temp.path().join("central");
    let unknown_dir = central_root.join("npx-skill");
    set_agent_dir(&pool, "central", &central_root).await;
    seed_central_skill(&pool, &unknown_dir, "npx-skill", "npx leftover").await;

    reset_unknown_source_skills_impl(&pool, &ActiveTarget::Local, &["npx-skill".to_string()], &[])
        .await
        .unwrap();
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_none());

    seed_central_skill(&pool, &unknown_dir, "npx-skill", "reimported").await;
    assign_github_membership(&pool, "npx-skill").await;

    let ids = list_unknown_source_central_skill_ids(&pool).await.unwrap();
    assert!(ids.is_empty());
    assert!(db::get_skill_by_id(&pool, "npx-skill")
        .await
        .unwrap()
        .is_some());
}
