//! Collections schema：用户定义的技能集合 + 集合-技能关联。
//!
//! 与 metadata.tags 的差异：tags 是本地分类标签（含 AI 建议），collections 是
//! 用户显式批量管理 / 导入导出的容器。两者解耦，UI 上分开展示。

use sqlx::SqliteConnection;

pub(super) async fn init(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    // collections table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(&mut *connection)
    .await?;

    // collection_skills table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_skills (
            collection_id TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            added_at      TEXT NOT NULL,
            PRIMARY KEY (collection_id, skill_id)
        )",
    )
    .execute(&mut *connection)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collection_skills_skill_id
         ON collection_skills(skill_id)",
    )
    .execute(&mut *connection)
    .await?;

    Ok(())
}
