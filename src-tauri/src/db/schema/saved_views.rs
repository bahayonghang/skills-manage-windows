//! `skill_saved_views` schema — Central Skills V2 saved view persistence.
//!
//! 保存视图：把 sidebar 的多维过滤 + 搜索 + 排序状态打包为一条可命名/重排序的
//! 用户预设。`query` 列存的是 `CentralViewState` 的 JSON 序列化结果，由前端
//! `centralViewState.ts` 编解码；后端不负责解析其内部结构，只做行级 CRUD。
//!
//! 不引用其它表（无外键），独立演进，删除技能/标签不会级联清理 saved view 内
//! 的引用 —— 那些「引用」只是 query JSON 里的字符串字面量，回放时若指向已不存在
//! 的 repo/tag，前端会容错降级（多余的 filter 自动失效）。

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_saved_views (
            id               TEXT PRIMARY KEY,
            name             TEXT NOT NULL,
            query            TEXT NOT NULL,
            sort_order       INTEGER NOT NULL DEFAULT 0,
            icon             TEXT,
            pinned           INTEGER NOT NULL DEFAULT 0,
            created_at       TEXT NOT NULL,
            updated_at       TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_saved_views_order
         ON skill_saved_views(sort_order)",
    )
    .execute(pool)
    .await?;

    Ok(())
}
