use super::*;
use crate::db::{self, DbPool, Skill, SkillInstallation};
use crate::test_support::mem_pool as setup;
use sqlx::Row;
use tempfile::TempDir;

async fn set_central_root(pool: &DbPool, root: &Path) {
    let stored = stored_path_string(root);
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
        .bind(&stored)
        .bind("central")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT OR IGNORE INTO scan_directories (path, label, is_active, is_builtin, added_at)
         VALUES (?, 'Central Skills', 1, 1, ?)",
    )
    .bind(&stored)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .unwrap();
}

fn write_skill(root: &Path, id: &str, marker: &str) {
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {id}\ndescription: {marker}\n---\n\n# {id}\n"),
    )
    .unwrap();
}

fn skill_md_bytes(root: &Path, id: &str) -> Vec<u8> {
    std::fs::read(root.join(id).join("SKILL.md")).unwrap()
}

fn skill_row(root: &Path, id: &str) -> Skill {
    let dir = root.join(id);
    Skill {
        id: id.to_string(),
        uid: format!("{id}-uid"),
        name: id.to_string(),
        description: None,
        file_path: stored_path_string(&dir.join("SKILL.md")),
        canonical_path: Some(stored_path_string(&dir)),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

fn native_installation(skill_id: &str, installed_path: &Path) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: "central".to_string(),
        installed_path: stored_path_string(installed_path),
        link_type: "native".to_string(),
        symlink_target: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CentralPathSnapshot {
    agents: Vec<(String, String)>,
    scan_directories: Vec<(String, bool)>,
    skills: Vec<(String, String, Option<String>)>,
    skill_installations: Vec<(String, String, String, Option<String>)>,
}

async fn central_path_snapshot(pool: &DbPool) -> CentralPathSnapshot {
    let agent_rows = sqlx::query("SELECT id, global_skills_dir FROM agents ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();
    let scan_rows = sqlx::query("SELECT path, is_builtin FROM scan_directories ORDER BY path, id")
        .fetch_all(pool)
        .await
        .unwrap();
    let skill_rows = sqlx::query("SELECT id, file_path, canonical_path FROM skills ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();
    let installation_rows = sqlx::query(
        "SELECT skill_id, agent_id, installed_path, symlink_target
         FROM skill_installations
         ORDER BY skill_id, agent_id, installed_path",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    CentralPathSnapshot {
        agents: agent_rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("id"),
                    row.get::<String, _>("global_skills_dir"),
                )
            })
            .collect(),
        scan_directories: scan_rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("path"),
                    row.get::<bool, _>("is_builtin"),
                )
            })
            .collect(),
        skills: skill_rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("id"),
                    row.get::<String, _>("file_path"),
                    row.get::<Option<String>, _>("canonical_path"),
                )
            })
            .collect(),
        skill_installations: installation_rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("skill_id"),
                    row.get::<String, _>("agent_id"),
                    row.get::<String, _>("installed_path"),
                    row.get::<Option<String>, _>("symlink_target"),
                )
            })
            .collect(),
    }
}

async fn seed_relocate_roots(pool: &DbPool, source: &Path, target: &Path) {
    write_skill(source, "copy-me", "from-source");
    write_skill(source, "overwrite-me", "from-source");
    write_skill(target, "overwrite-me", "from-target");
    write_skill(target, "target-only", "from-target");
    set_central_root(pool, source).await;
    db::upsert_skill(pool, &skill_row(source, "copy-me"))
        .await
        .unwrap();
    db::upsert_skill(pool, &skill_row(source, "overwrite-me"))
        .await
        .unwrap();
    db::upsert_skill_installation(
        pool,
        &native_installation("copy-me", &source.join("copy-me")),
    )
    .await
    .unwrap();
}

async fn create_relocate_trigger(pool: &DbPool, name: &str, sql: &str) {
    sqlx::query(&format!("CREATE TRIGGER {name} {sql}"))
        .execute(pool)
        .await
        .unwrap();
}

async fn drop_relocate_trigger(pool: &DbPool, name: &str) {
    sqlx::query(&format!("DROP TRIGGER IF EXISTS {name}"))
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn central_store_location_preview_counts_copy_overwrite_and_target_only() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    let target = temp.path().join("new");
    write_skill(&source, "copy-me", "source");
    write_skill(&source, "overwrite-me", "source");
    write_skill(&target, "overwrite-me", "target");
    write_skill(&target, "target-only", "target");
    set_central_root(&pool, &source).await;

    let preview = preview_central_store_location_change_impl(&pool, &target.to_string_lossy())
        .await
        .unwrap();

    assert_eq!(preview.skills_to_copy, 1);
    assert_eq!(preview.skills_to_overwrite, 1);
    assert_eq!(preview.target_only_skills, 1);
}

#[tokio::test]
async fn central_store_location_apply_overwrites_preserves_old_and_imports_target_only() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    let target = temp.path().join("new");
    write_skill(&source, "same", "from-source");
    write_skill(&source, "source-only", "from-source");
    write_skill(&target, "same", "from-target");
    write_skill(&target, "target-only", "from-target");
    set_central_root(&pool, &source).await;
    db::upsert_skill(&pool, &skill_row(&source, "same"))
        .await
        .unwrap();

    let result = apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
        .await
        .unwrap();

    assert_eq!(result.copied, 1);
    assert_eq!(result.overwritten, 1);
    assert_eq!(result.target_only_imported, 1);
    assert!(source.join("same").join("SKILL.md").exists());
    let overwritten = std::fs::read_to_string(target.join("same").join("SKILL.md")).unwrap();
    assert!(overwritten.contains("from-source"));
    assert!(target.join("target-only").join("SKILL.md").exists());

    let central = db::get_agent_by_id(&pool, "central")
        .await
        .unwrap()
        .unwrap();
    assert!(crate::paths::paths_equivalent(
        Path::new(&central.global_skills_dir),
        &target,
    ));
    let skills = db::get_central_skills(&pool).await.unwrap();
    assert!(skills.iter().any(|skill| skill.id == "target-only"));
}

#[tokio::test]
async fn central_store_location_rejects_nested_and_same_paths() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    std::fs::create_dir_all(&source).unwrap();
    set_central_root(&pool, &source).await;

    let same = preview_central_store_location_change_impl(&pool, &source.to_string_lossy())
        .await
        .unwrap_err();
    assert_eq!(same.to_string(), "central_store_location_same_path");
    assert!(matches!(same, CentralStoreLocationError::SamePath));

    let nested = source.join("child");
    let nested_err = preview_central_store_location_change_impl(&pool, &nested.to_string_lossy())
        .await
        .unwrap_err();
    assert_eq!(nested_err.to_string(), "central_store_location_nested_path");
    assert!(matches!(nested_err, CentralStoreLocationError::NestedPath));
}

#[tokio::test]
async fn central_store_location_updates_existing_native_installation_paths() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    let target = temp.path().join("new");
    write_skill(&source, "native-skill", "source");
    set_central_root(&pool, &source).await;
    db::upsert_skill(&pool, &skill_row(&source, "native-skill"))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &native_installation("native-skill", &source.join("native-skill")),
    )
    .await
    .unwrap();

    apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
        .await
        .unwrap();

    let rows = db::get_skill_installations(&pool, "native-skill")
        .await
        .unwrap();
    assert!(rows.iter().any(|row| {
        row.agent_id == "central"
            && crate::paths::paths_equivalent(
                Path::new(&row.installed_path),
                &target.join("native-skill"),
            )
    }));
}

#[tokio::test]
async fn central_store_location_agents_update_failure_preserves_paths_and_compensates_created_dirs()
{
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    let target = temp.path().join("new");
    seed_relocate_roots(&pool, &source, &target).await;
    let source_copy = skill_md_bytes(&source, "copy-me");
    let source_overwrite = skill_md_bytes(&source, "overwrite-me");
    let target_only = skill_md_bytes(&target, "target-only");
    let before = central_path_snapshot(&pool).await;

    create_relocate_trigger(
        &pool,
        "fail_central_agents_global_skills_dir",
        "BEFORE UPDATE OF global_skills_dir ON agents
         WHEN NEW.id = 'central'
         BEGIN SELECT RAISE(ABORT, 'injected agents global_skills_dir failure'); END",
    )
    .await;

    let err = apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
        .await
        .expect_err("agents UPDATE trigger must fail relocate");
    assert!(
        matches!(err, CentralStoreLocationError::Db(_)),
        "unexpected relocate error: {err}"
    );

    assert_eq!(skill_md_bytes(&source, "copy-me"), source_copy);
    assert_eq!(skill_md_bytes(&source, "overwrite-me"), source_overwrite);
    assert_eq!(skill_md_bytes(&target, "target-only"), target_only);
    assert!(
        !target.join("copy-me").exists(),
        "newly copied skill dirs must be removed after DB failure"
    );
    assert_eq!(central_path_snapshot(&pool).await, before);

    drop_relocate_trigger(&pool, "fail_central_agents_global_skills_dir").await;
    apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
        .await
        .unwrap();

    let central = db::get_agent_by_id(&pool, "central")
        .await
        .unwrap()
        .unwrap();
    assert!(crate::paths::paths_equivalent(
        Path::new(&central.global_skills_dir),
        &target,
    ));
    assert!(source.join("copy-me").join("SKILL.md").exists());
    assert!(target.join("target-only").join("SKILL.md").exists());
    assert!(target.join("copy-me").join("SKILL.md").exists());
}

#[tokio::test]
async fn central_store_location_skills_rewrite_failure_rolls_back_all_four_tables() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("old");
    let target = temp.path().join("new");
    seed_relocate_roots(&pool, &source, &target).await;
    let source_copy = skill_md_bytes(&source, "copy-me");
    let target_only = skill_md_bytes(&target, "target-only");
    let before = central_path_snapshot(&pool).await;

    create_relocate_trigger(
        &pool,
        "fail_central_skills_path_rewrite",
        "BEFORE UPDATE OF file_path ON skills
         WHEN NEW.is_central = 1
         BEGIN SELECT RAISE(ABORT, 'injected skills path rewrite failure'); END",
    )
    .await;

    let err = apply_central_store_location_change_impl(&pool, &target.to_string_lossy(), true)
        .await
        .expect_err("skills path rewrite trigger must fail relocate");
    assert!(
        matches!(err, CentralStoreLocationError::Db(_)),
        "unexpected relocate error: {err}"
    );
    assert_eq!(skill_md_bytes(&source, "copy-me"), source_copy);
    assert_eq!(skill_md_bytes(&target, "target-only"), target_only);
    assert!(
        !target.join("copy-me").exists(),
        "newly copied skill dirs must be removed after later-table DB failure"
    );
    assert_eq!(
        central_path_snapshot(&pool).await,
        before,
        "agents, scan_directories, skills, and skill_installations must roll back together"
    );
}

#[tokio::test]
async fn central_store_location_prefix_collision_does_not_rewrite_store_extra() {
    let pool = setup().await;
    let temp = TempDir::new().unwrap();
    let old_root = temp.path().join("store");
    let new_root = temp.path().join("relocated");
    let extra_root = temp.path().join("store-extra");
    std::fs::create_dir_all(&old_root).unwrap();
    std::fs::create_dir_all(&new_root).unwrap();
    set_central_root(&pool, &old_root).await;

    db::upsert_skill(&pool, &skill_row(&old_root, "child"))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &native_installation("child", &old_root.join("child")),
    )
    .await
    .unwrap();

    let extra_skill = Skill {
        id: "extra-skill".to_string(),
        uid: "extra-skill-uid".to_string(),
        name: "extra-skill".to_string(),
        description: None,
        file_path: stored_path_string(&extra_root.join("extra-skill").join("SKILL.md")),
        canonical_path: Some(stored_path_string(&extra_root.join("extra-skill"))),
        is_central: true,
        source: Some("native".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    let extra_file_path = extra_skill.file_path.clone();
    let extra_canonical = extra_skill.canonical_path.clone();
    db::upsert_skill(&pool, &extra_skill).await.unwrap();
    db::upsert_skill_installation(
        &pool,
        &native_installation("extra-skill", &extra_root.join("extra-skill")),
    )
    .await
    .unwrap();

    update_central_root(&pool, &old_root, &new_root)
        .await
        .unwrap();

    let extra_after = db::get_skill_by_id(&pool, "extra-skill")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(extra_after.file_path, extra_file_path);
    assert_eq!(extra_after.canonical_path, extra_canonical);

    let child_after = db::get_skill_by_id(&pool, "child").await.unwrap().unwrap();
    assert!(crate::paths::paths_equivalent(
        Path::new(&child_after.file_path),
        &new_root.join("child").join("SKILL.md"),
    ));
    assert!(
        !child_after.file_path.contains("store-extra"),
        "child path must not pick up the sibling prefix: {}",
        child_after.file_path
    );

    let extra_install = db::get_skill_installations(&pool, "extra-skill")
        .await
        .unwrap();
    assert_eq!(extra_install.len(), 1);
    assert_eq!(
        extra_install[0].installed_path,
        stored_path_string(&extra_root.join("extra-skill"))
    );

    let child_install = db::get_skill_installations(&pool, "child").await.unwrap();
    assert!(child_install.iter().any(|row| {
        row.agent_id == "central"
            && crate::paths::paths_equivalent(
                Path::new(&row.installed_path),
                &new_root.join("child"),
            )
    }));
}
