//! `skill_tag_groups` CRUD — Central Skills V2 / M3.
//!
//! 标签分组：一级，不允许嵌套（D4）。后端只做行级 CRUD；group 与 tag 的关系
//! 通过 `skill_tags.group_id` 列维系，由本模块的 `set_tag_group` 维护。
//! 删除 group 时把所有成员 `skill_tags.group_id` 置 NULL（在事务中处理），
//! 避免悬空引用。

use sqlx::Row;
use uuid::Uuid;

use crate::db::types::{DbPool, TagGroup};
use crate::db::util::now_rfc3339;

pub struct NewTagGroup<'a> {
    pub name: &'a str,
    pub color: Option<&'a str>,
}

pub struct TagGroupPatch<'a> {
    pub name: Option<&'a str>,
    /// `Some(None)` 清空 color；`None` 不变。
    pub color: Option<Option<&'a str>>,
}

pub async fn list_tag_groups(pool: &DbPool) -> Result<Vec<TagGroup>, String> {
    sqlx::query_as::<_, TagGroup>(
        "SELECT id, name, color, sort_order, is_builtin, created_at, updated_at
         FROM skill_tag_groups
         ORDER BY sort_order ASC, created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_tag_group(pool: &DbPool, id: &str) -> Result<Option<TagGroup>, String> {
    sqlx::query_as::<_, TagGroup>(
        "SELECT id, name, color, sort_order, is_builtin, created_at, updated_at
         FROM skill_tag_groups WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn create_tag_group(pool: &DbPool, input: NewTagGroup<'_>) -> Result<TagGroup, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let next_order = next_sort_order(pool).await?;

    sqlx::query(
        "INSERT INTO skill_tag_groups
            (id, name, color, sort_order, is_builtin, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(input.name)
    .bind(input.color)
    .bind(next_order)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_tag_group(pool, &id)
        .await?
        .ok_or_else(|| "Failed to retrieve newly created tag group".to_string())
}

pub async fn update_tag_group(
    pool: &DbPool,
    id: &str,
    patch: TagGroupPatch<'_>,
) -> Result<TagGroup, String> {
    if get_tag_group(pool, id).await?.is_none() {
        return Err(format!("Tag group '{id}' not found"));
    }
    let now = now_rfc3339();

    if let Some(name) = patch.name {
        sqlx::query("UPDATE skill_tag_groups SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(color) = patch.color {
        sqlx::query("UPDATE skill_tag_groups SET color = ?, updated_at = ? WHERE id = ?")
            .bind(color)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    get_tag_group(pool, id)
        .await?
        .ok_or_else(|| "Failed to retrieve updated tag group".to_string())
}

/// 删除 group，同时把所有标签的 group_id 置 NULL（事务原子）。
pub async fn delete_tag_group(pool: &DbPool, id: &str) -> Result<(), String> {
    let now = now_rfc3339();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("UPDATE skill_tags SET group_id = NULL, updated_at = ? WHERE group_id = ?")
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_tag_groups WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn reorder_tag_groups(pool: &DbPool, ids: &[String]) -> Result<(), String> {
    let now = now_rfc3339();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE skill_tag_groups SET sort_order = ?, updated_at = ? WHERE id = ?")
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

/// 把 `tag_id` 的 `group_id` 设为 `group_id`（`None` 表示移出分组）。
/// 若 tag 或 group 不存在则返回错误。
pub async fn set_tag_group(
    pool: &DbPool,
    tag_id: &str,
    group_id: Option<&str>,
) -> Result<(), String> {
    // 校验 tag 存在
    let tag_exists = sqlx::query("SELECT 1 FROM skill_tags WHERE id = ?")
        .bind(tag_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !tag_exists {
        return Err(format!("Tag '{tag_id}' not found"));
    }

    if let Some(gid) = group_id {
        if get_tag_group(pool, gid).await?.is_none() {
            return Err(format!("Tag group '{gid}' not found"));
        }
    }

    let now = now_rfc3339();
    sqlx::query("UPDATE skill_tags SET group_id = ?, updated_at = ? WHERE id = ?")
        .bind(group_id)
        .bind(&now)
        .bind(tag_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn next_sort_order(pool: &DbPool) -> Result<i64, String> {
    let row = sqlx::query("SELECT COALESCE(MAX(sort_order), -1) + 1 AS next FROM skill_tag_groups")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let next: i64 = row.try_get("next").map_err(|e| e.to_string())?;
    Ok(next)
}
