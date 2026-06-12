use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::db::{self, DbPool};
use crate::services::installation::copy_dir_all;

pub const CENTRAL_STORE_MIGRATION_SETTING_KEY: &str = "central_private_store_migration_v1";

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
) -> Result<CentralStoreMigrationSummary, String> {
    // 1.1 跳过已完成的一次性迁移
    if let Some(raw) = db::get_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY)
        .await
        .map_err(|e| e.to_string())?
    {
        if let Ok(summary) = serde_json::from_str::<CentralStoreMigrationSummary>(&raw) {
            return Ok(summary);
        }
    }

    // 1.2 使用当前平台路径作为迁移源与迁移目标
    let source_dir = crate::paths::universal_skills_dir();
    let target_dir = crate::paths::central_skills_dir();
    let summary = migrate_legacy_central_skills_between(&source_dir, &target_dir).await?;

    // 1.3 记录迁移摘要，供设置页或诊断读取
    let encoded = serde_json::to_string(&summary).map_err(|e| e.to_string())?;
    db::set_setting(pool, CENTRAL_STORE_MIGRATION_SETTING_KEY, &encoded)
        .await
        .map_err(|e| e.to_string())?;
    Ok(summary)
}

async fn migrate_legacy_central_skills_between(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<CentralStoreMigrationSummary, String> {
    let mut summary = CentralStoreMigrationSummary::new(source_dir, target_dir);

    // 1.4 相同路径不迁移，只写入空摘要
    if crate::paths::paths_equivalent(source_dir, target_dir) {
        return Ok(summary);
    }

    // 1.5 确保私有 Central 仓库存在
    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create private Central store: {}", e))?;

    // 1.6 旧目录不存在时结束，不创建任何平台侧副作用
    if !source_dir.exists() {
        return Ok(summary);
    }

    let entries = std::fs::read_dir(source_dir)
        .map_err(|e| format!("Failed to read legacy Central store: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read legacy skill entry: {}", e))?;
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

    #[tokio::test]
    async fn copies_legacy_skills_to_private_store_without_removing_source() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join(".agents").join("skills");
        let target = tmp.path().join(".skillsmanage").join("skills");
        create_skill(&source, "frontend-design");

        let summary = migrate_legacy_central_skills_between(&source, &target)
            .await
            .unwrap();

        assert_eq!(summary.copied, 1);
        assert_eq!(summary.skipped_existing, 0);
        assert!(source.join("frontend-design").join("SKILL.md").exists());
        assert!(target.join("frontend-design").join("SKILL.md").exists());
    }

    #[tokio::test]
    async fn skips_existing_private_skills() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join(".agents").join("skills");
        let target = tmp.path().join(".skillsmanage").join("skills");
        create_skill(&source, "code-review");
        create_skill(&target, "code-review");

        let summary = migrate_legacy_central_skills_between(&source, &target)
            .await
            .unwrap();

        assert_eq!(summary.copied, 0);
        assert_eq!(summary.skipped_existing, 1);
    }
}
