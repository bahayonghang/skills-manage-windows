//! `skill_saved_views` CRUD — Central Skills V2 / M2.
//!
//! 后端不解析 `query` JSON，只做行级 CRUD。`order` 在 DB 里是 `sort_order` 列
//! （`order` 是 SQL 关键字，避免引号风险）。

use sqlx::Row;
use uuid::Uuid;

use crate::db::types::{DbPool, SavedView};
use crate::db::util::now_rfc3339;

/// 新建 saved view 时的输入字段。`order` 留空时自动追加到末尾。
pub struct NewSavedView<'a> {
    pub name: &'a str,
    pub query: &'a str,
    pub icon: Option<&'a str>,
    pub pinned: bool,
}

/// 更新 saved view 时允许变更的字段（不能改 id / created_at）。
pub struct SavedViewPatch<'a> {
    pub name: Option<&'a str>,
    pub query: Option<&'a str>,
    pub icon: Option<Option<&'a str>>,
    pub pinned: Option<bool>,
}

pub async fn list_saved_views(pool: &DbPool) -> Result<Vec<SavedView>, String> {
    sqlx::query_as::<_, SavedView>(
        "SELECT id, name, query, sort_order, icon, pinned, created_at, updated_at
         FROM skill_saved_views
         ORDER BY pinned DESC, sort_order ASC, created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_saved_view(pool: &DbPool, id: &str) -> Result<Option<SavedView>, String> {
    sqlx::query_as::<_, SavedView>(
        "SELECT id, name, query, sort_order, icon, pinned, created_at, updated_at
         FROM skill_saved_views WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn create_saved_view(
    pool: &DbPool,
    input: NewSavedView<'_>,
) -> Result<SavedView, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_rfc3339();

    // 自动 order = max(sort_order) + 1
    let next_order = next_sort_order(pool).await?;

    sqlx::query(
        "INSERT INTO skill_saved_views
            (id, name, query, sort_order, icon, pinned, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.name)
    .bind(input.query)
    .bind(next_order)
    .bind(input.icon)
    .bind(input.pinned)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_saved_view(pool, &id)
        .await?
        .ok_or_else(|| "Failed to retrieve newly created saved view".to_string())
}

pub async fn update_saved_view(
    pool: &DbPool,
    id: &str,
    patch: SavedViewPatch<'_>,
) -> Result<SavedView, String> {
    // 先确认存在
    if get_saved_view(pool, id).await?.is_none() {
        return Err(format!("Saved view '{id}' not found"));
    }

    let now = now_rfc3339();

    if let Some(name) = patch.name {
        sqlx::query("UPDATE skill_saved_views SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(query) = patch.query {
        sqlx::query("UPDATE skill_saved_views SET query = ?, updated_at = ? WHERE id = ?")
            .bind(query)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(icon) = patch.icon {
        sqlx::query("UPDATE skill_saved_views SET icon = ?, updated_at = ? WHERE id = ?")
            .bind(icon)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(pinned) = patch.pinned {
        sqlx::query("UPDATE skill_saved_views SET pinned = ?, updated_at = ? WHERE id = ?")
            .bind(pinned)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    get_saved_view(pool, id)
        .await?
        .ok_or_else(|| "Failed to retrieve updated saved view".to_string())
}

pub async fn delete_saved_view(pool: &DbPool, id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_saved_views WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// 按 `ids` 数组的顺序把每条记录的 `sort_order` 改为其下标。未列出的记录保持原状。
pub async fn reorder_saved_views(pool: &DbPool, ids: &[String]) -> Result<(), String> {
    let now = now_rfc3339();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE skill_saved_views SET sort_order = ?, updated_at = ? WHERE id = ?")
            .bind(index as i64)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn next_sort_order(pool: &DbPool) -> Result<i64, String> {
    let row = sqlx::query("SELECT COALESCE(MAX(sort_order), -1) + 1 AS next FROM skill_saved_views")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let next: i64 = row.try_get("next").map_err(|e| e.to_string())?;
    Ok(next)
}
