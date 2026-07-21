//! 项目级 skill 管理端到端集成测试。
//!
//! 单元测试 (`src/services/projects/tests.rs`) 已覆盖每条命令独立逻辑；本文件
//! 验证「add → scan → install → uninstall → remove」串成的真实使用链路，
//! 在临时目录里走完一遍 symlink 和 copy 两种安装方式。
//!
//! 引用路径走 `skillport_lib::*`（crate `[lib]` name），与 IPC 命令在生产环境
//! 看到的 API 表面一致。

mod common;

use tempfile::TempDir;

use skillport_lib::services::projects::{
    add_project_impl, get_project_skills_impl, install_skill_to_project_impl, list_projects_impl,
    remove_project_impl, rescan_project_impl, uninstall_skill_from_project_impl,
};

use common::{fresh_db, seed_central_skill};

/// 完整链路 (copy 模式)：add → install → uninstall → remove。
#[tokio::test]
async fn e2e_copy_full_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let pool = fresh_db().await;

    // 中央 skill 准备
    let canonical = tmp.path().join(".agents/skills/copykid");
    seed_central_skill(&pool, &canonical, "copykid", "copykid").await;

    // 1. add
    let project_root = tmp.path().join("proj-copy");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(list_projects_impl(&pool).await.unwrap().len(), 1);

    // 2. scan empty
    let initial = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(initial, 0, "fresh project should scan zero skills");

    // 3. install copy
    let psi = install_skill_to_project_impl(&pool, &project.id, "copykid", "claude-code", "copy")
        .await
        .unwrap();
    assert_eq!(psi.link_type, "copy");
    let installed_path = project_root.join(".claude/skills/copykid");
    assert!(installed_path.exists(), "copy must materialise dir");
    assert!(installed_path.join("SKILL.md").exists());

    // 4. rescan picks it up
    let after_install = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(after_install, 1);
    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "copy");

    // 5. uninstall
    uninstall_skill_from_project_impl(&pool, &project.id, "copykid", "claude-code")
        .await
        .unwrap();
    assert!(!installed_path.exists(), "uninstall must remove copy dir");
    assert_eq!(
        get_project_skills_impl(&pool, &project.id)
            .await
            .unwrap()
            .len(),
        0,
        "psi cleared after uninstall"
    );

    // 6. remove project
    remove_project_impl(&pool, &project.id, false)
        .await
        .unwrap();
    assert!(
        list_projects_impl(&pool).await.unwrap().is_empty(),
        "project must be gone"
    );
}

/// 完整链路 (symlink 模式)：Windows 非开发者模式下 create_symlink 失败时跳过。
#[tokio::test]
async fn e2e_symlink_full_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let pool = fresh_db().await;

    let canonical = tmp.path().join(".agents/skills/linkkid");
    seed_central_skill(&pool, &canonical, "linkkid", "linkkid").await;

    let project_root = tmp.path().join("proj-link");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let install_result =
        install_skill_to_project_impl(&pool, &project.id, "linkkid", "claude-code", "symlink")
            .await;
    let psi = match install_result {
        Ok(p) => p,
        Err(e) if e.to_string().to_lowercase().contains("symlink") => return, // Windows 非开发者模式
        Err(e) => panic!("unexpected install error: {e}"),
    };
    assert_eq!(psi.link_type, "symlink");
    let installed_path = project_root.join(".claude/skills/linkkid");
    let meta = std::fs::symlink_metadata(&installed_path).unwrap();
    assert!(meta.file_type().is_symlink());

    // rescan 识别 symlink
    rescan_project_impl(&pool, &project.id).await.unwrap();
    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].link_type, "symlink");

    // uninstall 清掉 symlink
    uninstall_skill_from_project_impl(&pool, &project.id, "linkkid", "claude-code")
        .await
        .unwrap();
    assert!(
        std::fs::symlink_metadata(&installed_path).is_err(),
        "symlink should be gone"
    );

    // remove project（保留磁盘）
    remove_project_impl(&pool, &project.id, false)
        .await
        .unwrap();
    assert!(list_projects_impl(&pool).await.unwrap().is_empty());
}

/// remove_project(uninstall_skills=true) 必须把磁盘上 psi 记录的目录全删掉。
#[tokio::test]
async fn e2e_remove_with_uninstall_clears_disk() {
    let tmp = TempDir::new().unwrap();
    let pool = fresh_db().await;

    let canonical_a = tmp.path().join(".agents/skills/a");
    let canonical_b = tmp.path().join(".agents/skills/b");
    seed_central_skill(&pool, &canonical_a, "a", "a").await;
    seed_central_skill(&pool, &canonical_b, "b", "b").await;

    let project_root = tmp.path().join("proj-cleanup");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    install_skill_to_project_impl(&pool, &project.id, "a", "claude-code", "copy")
        .await
        .unwrap();
    install_skill_to_project_impl(&pool, &project.id, "b", "claude-code", "copy")
        .await
        .unwrap();

    let a_path = project_root.join(".claude/skills/a");
    let b_path = project_root.join(".claude/skills/b");
    assert!(a_path.exists() && b_path.exists());

    remove_project_impl(&pool, &project.id, true).await.unwrap();
    assert!(!a_path.exists(), "skill a dir must be cleared");
    assert!(!b_path.exists(), "skill b dir must be cleared");
    assert!(list_projects_impl(&pool).await.unwrap().is_empty());
}

/// remove_project(uninstall_skills=false) 必须保留磁盘上的安装目录。
#[tokio::test]
async fn e2e_remove_without_uninstall_preserves_disk() {
    let tmp = TempDir::new().unwrap();
    let pool = fresh_db().await;

    let canonical = tmp.path().join(".agents/skills/keepme");
    seed_central_skill(&pool, &canonical, "keepme", "keepme").await;

    let project_root = tmp.path().join("proj-keep");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    install_skill_to_project_impl(&pool, &project.id, "keepme", "claude-code", "copy")
        .await
        .unwrap();
    let installed = project_root.join(".claude/skills/keepme");
    assert!(installed.exists());

    remove_project_impl(&pool, &project.id, false)
        .await
        .unwrap();
    assert!(
        installed.exists(),
        "disk must be untouched when uninstall_skills=false"
    );
    assert!(list_projects_impl(&pool).await.unwrap().is_empty());
}

/// 同一中央 skill 装到两个项目：互不影响。
#[tokio::test]
async fn e2e_two_projects_share_skill() {
    let tmp = TempDir::new().unwrap();
    let pool = fresh_db().await;

    let canonical = tmp.path().join(".agents/skills/shared");
    seed_central_skill(&pool, &canonical, "shared", "shared").await;

    let root_a = tmp.path().join("proj-a");
    let root_b = tmp.path().join("proj-b");
    std::fs::create_dir_all(&root_a).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();

    let proj_a = add_project_impl(&pool, root_a.to_str().unwrap())
        .await
        .unwrap();
    let proj_b = add_project_impl(&pool, root_b.to_str().unwrap())
        .await
        .unwrap();
    assert_ne!(proj_a.id, proj_b.id);

    install_skill_to_project_impl(&pool, &proj_a.id, "shared", "claude-code", "copy")
        .await
        .unwrap();
    install_skill_to_project_impl(&pool, &proj_b.id, "shared", "claude-code", "copy")
        .await
        .unwrap();

    let a_path = root_a.join(".claude/skills/shared");
    let b_path = root_b.join(".claude/skills/shared");
    assert!(a_path.exists() && b_path.exists());

    // 卸 A，B 不受影响
    uninstall_skill_from_project_impl(&pool, &proj_a.id, "shared", "claude-code")
        .await
        .unwrap();
    assert!(!a_path.exists());
    assert!(b_path.exists());
    assert_eq!(
        get_project_skills_impl(&pool, &proj_b.id)
            .await
            .unwrap()
            .len(),
        1
    );

    // remove A 不连带清 B
    remove_project_impl(&pool, &proj_a.id, true).await.unwrap();
    assert!(
        b_path.exists(),
        "removing project A must not touch project B"
    );
    assert_eq!(list_projects_impl(&pool).await.unwrap().len(), 1);
}
