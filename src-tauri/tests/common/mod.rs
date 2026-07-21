use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;

use skillport_lib::db::{self, DbPool, Skill};

pub async fn fresh_db() -> DbPool {
    // 豁免 test_support::mem_pool：integration crate 无法访问 #[cfg(test)] test_support。
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

fn write_skill_md(dir: &Path, name: &str, description: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let body = format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n");
    std::fs::write(dir.join("SKILL.md"), body).unwrap();
}

pub async fn seed_central_skill(
    pool: &DbPool,
    canonical_dir: &Path,
    skill_id: &str,
    name: &str,
) -> Skill {
    write_skill_md(canonical_dir, name, "integration seed");
    let skill = Skill {
        id: skill_id.to_string(),
        uid: format!("{skill_id}-uid"),
        name: name.to_string(),
        description: Some("integration seed".to_string()),
        file_path: canonical_dir
            .join("SKILL.md")
            .to_string_lossy()
            .into_owned(),
        canonical_path: Some(canonical_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: None,
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(pool, &skill).await.unwrap();
    skill
}
