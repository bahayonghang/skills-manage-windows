use super::*;
use crate::db::{self, DbPool, Skill, SkillInstallation};
use crate::test_support::mem_pool as setup;
use tempfile::TempDir;

async fn set_central_root(pool: &DbPool, root: &Path) {
    crate::test_support::set_agent_dir(pool, "central", root).await;
    sqlx::query(
        "INSERT OR IGNORE INTO scan_directories (path, label, is_active, is_builtin, added_at)
         VALUES (?, 'Central Skills', 1, 1, ?)",
    )
    .bind(root.to_string_lossy().into_owned())
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

fn skill_row(root: &Path, id: &str) -> Skill {
    let dir = root.join(id);
    Skill {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
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
        &SkillInstallation {
            skill_id: "native-skill".to_string(),
            agent_id: "central".to_string(),
            installed_path: source.join("native-skill").to_string_lossy().into_owned(),
            link_type: "native".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
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
