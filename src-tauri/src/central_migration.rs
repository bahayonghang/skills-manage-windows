use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::Duration;

use crate::db::{self, DbPool};
use crate::fs_util::run_blocking_fs_with;
use crate::services::central_mutation::{
    acquire_central_mutation_guard, DEFAULT_CENTRAL_MUTATION_TIMEOUT,
};
use crate::services::installation::copy_dir_all;

pub const CENTRAL_STORE_MIGRATION_SETTING_KEY: &str = "central_private_store_migration_v1";

/// Failure categories for the one-shot legacy Central store migration.
/// Display texts preserve the historical string-error wording verbatim.
#[derive(Debug, thiserror::Error)]
pub enum CentralMigrationError {
    /// Migration marker read/write via db settings.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Migration summary JSON encoding.
    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error("Failed to create private Central store: {0}")]
    CreateStore(#[source] std::io::Error),

    #[error("Failed to read legacy Central store: {0}")]
    ReadStore(#[source] std::io::Error),

    #[error("Failed to read legacy skill entry: {0}")]
    ReadEntry(#[source] std::io::Error),

    #[error(transparent)]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    #[error("Filesystem task '{operation}' failed to join: {reason}")]
    TaskJoin {
        operation: &'static str,
        reason: String,
    },
}

impl CentralMigrationError {
    fn task_join(operation: &'static str, reason: String) -> Self {
        Self::TaskJoin { operation, reason }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CentralStoreMigrationSummary {
    pub source_path: String,
    pub target_path: String,
    pub copied: usize,
    pub skipped_existing: usize,
    pub failed: usize,
    pub failures: Vec<String>,
    pub completed_at: String,
}

impl CentralStoreMigrationSummary {
    fn new(source_dir: &Path, target_dir: &Path) -> Self {
        Self {
            source_path: source_dir.to_string_lossy().into_owned(),
            target_path: target_dir.to_string_lossy().into_owned(),
            copied: 0,
            skipped_existing: 0,
            failed: 0,
            failures: Vec::new(),
            completed_at: Utc::now().to_rfc3339(),
        }
    }
}

/*
 * ========================================================================
 * 步骤1：迁移旧中央仓库
 * ========================================================================
 * 目标：
 * 1) 将旧 `~/.agents/skills` 中的技能复制到私有 Central 仓库
 * 2) 保留旧目录不删除，避免影响 Codex / Copilot 等现有环境
 */
pub async fn migrate_legacy_central_skills_to_private_store(
    pool: &DbPool,
) -> Result<CentralStoreMigrationSummary, CentralMigrationError> {
    // 1.1 跳过已完成的一次性迁移
    if let Some(raw) = db::get_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY).await? {
        if let Ok(summary) = serde_json::from_str::<CentralStoreMigrationSummary>(&raw) {
            return Ok(summary);
        }
    }

    let _guard = acquire_central_mutation_guard(
        "legacy Central store migration",
        DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;

    migrate_legacy_central_skills_under_guard(
        pool,
        crate::paths::universal_skills_dir(),
        crate::paths::central_skills_dir(),
    )
    .await
}

async fn migrate_legacy_central_skills_under_guard(
    pool: &DbPool,
    source_dir: PathBuf,
    target_dir: PathBuf,
) -> Result<CentralStoreMigrationSummary, CentralMigrationError> {
    // Another process may have completed migration while this process waited.
    if let Some(raw) = db::get_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY).await? {
        if let Ok(summary) = serde_json::from_str::<CentralStoreMigrationSummary>(&raw) {
            return Ok(summary);
        }
    }

    let summary = run_blocking_fs_with(
        "migrate legacy Central store",
        move || migrate_legacy_central_skills_between_blocking(&source_dir, &target_dir),
        CentralMigrationError::task_join,
    )
    .await?;

    // 1.3 记录迁移摘要，供设置页或诊断读取
    let encoded = serde_json::to_string(&summary)?;
    db::set_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY, &encoded).await?;
    Ok(summary)
}

#[cfg(test)]
async fn migrate_legacy_central_skills_with_paths(
    pool: &DbPool,
    source_dir: PathBuf,
    target_dir: PathBuf,
    lock_path: PathBuf,
    timeout: Duration,
) -> Result<CentralStoreMigrationSummary, CentralMigrationError> {
    if let Some(raw) = db::get_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY).await? {
        if let Ok(summary) = serde_json::from_str::<CentralStoreMigrationSummary>(&raw) {
            return Ok(summary);
        }
    }
    let _guard = crate::services::central_mutation::acquire_central_mutation_guard_at(
        lock_path,
        "legacy Central store migration test",
        timeout,
    )
    .await?;
    migrate_legacy_central_skills_under_guard(pool, source_dir, target_dir).await
}

fn migrate_legacy_central_skills_between_blocking(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<CentralStoreMigrationSummary, CentralMigrationError> {
    let mut summary = CentralStoreMigrationSummary::new(source_dir, target_dir);

    // 1.4 相同路径不迁移，只写入空摘要
    if crate::paths::paths_equivalent(source_dir, target_dir) {
        return Ok(summary);
    }

    // 1.5 确保私有 Central 仓库存在
    std::fs::create_dir_all(target_dir).map_err(CentralMigrationError::CreateStore)?;

    // 1.6 旧目录不存在时结束，不创建任何平台侧副作用
    if !source_dir.exists() {
        return Ok(summary);
    }

    let entries = std::fs::read_dir(source_dir).map_err(CentralMigrationError::ReadStore)?;

    for entry in entries {
        let entry = entry.map_err(CentralMigrationError::ReadEntry)?;
        let source_skill_dir = entry.path();
        if !source_skill_dir.join("SKILL.md").exists() {
            continue;
        }

        let target_skill_dir = target_dir.join(entry.file_name());
        if target_skill_dir.exists() {
            summary.skipped_existing += 1;
            continue;
        }

        match copy_dir_all(&source_skill_dir, &target_skill_dir) {
            Ok(()) => summary.copied += 1,
            Err(error) => {
                summary.failed += 1;
                summary.failures.push(format!(
                    "{} -> {}: {}",
                    source_skill_dir.display(),
                    target_skill_dir.display(),
                    error
                ));
                let _ = std::fs::remove_dir_all(&target_skill_dir);
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill(dir: &Path, id: &str) {
        let skill_dir = dir.join(id);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {id}\ndescription: test\n---\n\n# {id}\n"),
        )
        .unwrap();
    }

    #[test]
    fn copies_legacy_skills_to_private_store_without_removing_source() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join(".agents").join("skills");
        let target = tmp.path().join(".skillsmanage").join("skills");
        create_skill(&source, "frontend-design");

        let summary = migrate_legacy_central_skills_between_blocking(&source, &target).unwrap();

        assert_eq!(summary.copied, 1);
        assert_eq!(summary.skipped_existing, 0);
        assert!(source.join("frontend-design").join("SKILL.md").exists());
        assert!(target.join("frontend-design").join("SKILL.md").exists());
    }

    #[test]
    fn skips_existing_private_skills() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join(".agents").join("skills");
        let target = tmp.path().join(".skillsmanage").join("skills");
        create_skill(&source, "code-review");
        create_skill(&target, "code-review");

        let summary = migrate_legacy_central_skills_between_blocking(&source, &target).unwrap();

        assert_eq!(summary.copied, 0);
        assert_eq!(summary.skipped_existing, 1);
    }

    #[tokio::test]
    async fn migration_contends_on_local_mutation_lock_and_retries_after_release() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        let target = tmp.path().join("target");
        let lock_path = tmp.path().join("central-mutation.lock");
        create_skill(&source, "concurrent-skill");
        let (pool, _db_dir) = crate::test_support::file_pool().await;

        let held = crate::services::central_mutation::acquire_central_mutation_guard_at(
            lock_path.clone(),
            "held by contender",
            Duration::from_secs(1),
        )
        .await
        .unwrap();
        let error = migrate_legacy_central_skills_with_paths(
            &pool,
            source.clone(),
            target.clone(),
            lock_path.clone(),
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            CentralMigrationError::CentralMutation(
                crate::services::central_mutation::CentralMutationError::Timeout { .. }
            )
        ));
        assert!(db::get_setting(&pool, CENTRAL_STORE_MIGRATION_SETTING_KEY)
            .await
            .unwrap()
            .is_none());

        drop(held);
        let summary = migrate_legacy_central_skills_with_paths(
            &pool,
            source,
            target.clone(),
            lock_path,
            Duration::from_secs(1),
        )
        .await
        .unwrap();
        assert_eq!(summary.copied, 1);
        assert!(target.join("concurrent-skill").join("SKILL.md").exists());
    }
}
