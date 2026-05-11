//! Tag Groups commands — Central Skills V2 / M3.
//!
//! 6 IPC：list / create / update / delete / reorder / set_tag_group。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{self, DbPool, TagGroup};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagGroupInput {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagGroupInput {
    pub name: Option<String>,
    /// `Some(None)` 清空 color；`None` 不变。
    #[serde(default, deserialize_with = "deserialize_optional_optional")]
    pub color: Option<Option<String>>,
}

fn deserialize_optional_optional<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    Option::<Option<String>>::deserialize(deserializer).or_else(|_| Ok(None))
}

// ─── impl layer ──────────────────────────────────────────────────────────────

pub async fn list_tag_groups_impl(pool: &DbPool) -> Result<Vec<TagGroup>, String> {
    db::list_tag_groups(pool).await
}

pub async fn create_tag_group_impl(
    pool: &DbPool,
    input: CreateTagGroupInput,
) -> Result<TagGroup, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Tag group name cannot be empty".to_string());
    }
    db::create_tag_group(
        pool,
        db::NewTagGroup {
            name,
            color: input.color.as_deref(),
        },
    )
    .await
}

pub async fn update_tag_group_impl(
    pool: &DbPool,
    id: &str,
    input: UpdateTagGroupInput,
) -> Result<TagGroup, String> {
    if let Some(ref name) = input.name {
        if name.trim().is_empty() {
            return Err("Tag group name cannot be empty".to_string());
        }
    }
    let color_patch: Option<Option<&str>> = input.color.as_ref().map(|opt| opt.as_deref());
    db::update_tag_group(
        pool,
        id,
        db::TagGroupPatch {
            name: input.name.as_deref(),
            color: color_patch,
        },
    )
    .await
}

pub async fn delete_tag_group_impl(pool: &DbPool, id: &str) -> Result<(), String> {
    if db::get_tag_group(pool, id).await?.is_none() {
        return Err(format!("Tag group '{id}' not found"));
    }
    db::delete_tag_group(pool, id).await
}

pub async fn reorder_tag_groups_impl(pool: &DbPool, ids: Vec<String>) -> Result<(), String> {
    db::reorder_tag_groups(pool, &ids).await
}

pub async fn set_tag_group_impl(
    pool: &DbPool,
    tag_id: &str,
    group_id: Option<&str>,
) -> Result<(), String> {
    db::set_tag_group(pool, tag_id, group_id).await
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tag_groups(state: State<'_, AppState>) -> Result<Vec<TagGroup>, String> {
    let pool = state.active_db().await?;
    list_tag_groups_impl(&pool).await
}

#[tauri::command]
pub async fn create_tag_group(
    state: State<'_, AppState>,
    input: CreateTagGroupInput,
) -> Result<TagGroup, String> {
    let pool = state.active_db().await?;
    create_tag_group_impl(&pool, input).await
}

#[tauri::command]
pub async fn update_tag_group(
    state: State<'_, AppState>,
    id: String,
    input: UpdateTagGroupInput,
) -> Result<TagGroup, String> {
    let pool = state.active_db().await?;
    update_tag_group_impl(&pool, &id, input).await
}

#[tauri::command]
pub async fn delete_tag_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.active_db().await?;
    delete_tag_group_impl(&pool, &id).await
}

#[tauri::command]
pub async fn reorder_tag_groups(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    reorder_tag_groups_impl(&pool, ids).await
}

#[tauri::command]
pub async fn set_tag_group(
    state: State<'_, AppState>,
    tag_id: String,
    group_id: Option<String>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    set_tag_group_impl(&pool, &tag_id, group_id.as_deref()).await
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        db::init_database(&pool).await.unwrap();
        pool
    }

    fn make_input(name: &str) -> CreateTagGroupInput {
        CreateTagGroupInput {
            name: name.to_string(),
            color: None,
        }
    }

    #[tokio::test]
    async fn create_tag_group_returns_row_with_generated_id() {
        let pool = setup_test_db().await;
        let group = create_tag_group_impl(&pool, make_input("Status")).await.unwrap();
        assert_eq!(group.name, "Status");
        assert!(!group.id.is_empty());
        assert_eq!(group.sort_order, 0);
        assert!(!group.is_builtin);
        assert!(group.color.is_none());
    }

    #[tokio::test]
    async fn create_tag_group_rejects_empty_name() {
        let pool = setup_test_db().await;
        let err = create_tag_group_impl(&pool, make_input("   ")).await.unwrap_err();
        assert!(err.contains("name"));
    }

    #[tokio::test]
    async fn create_tag_group_increments_sort_order() {
        let pool = setup_test_db().await;
        let a = create_tag_group_impl(&pool, make_input("A")).await.unwrap();
        let b = create_tag_group_impl(&pool, make_input("B")).await.unwrap();
        let c = create_tag_group_impl(&pool, make_input("C")).await.unwrap();
        assert_eq!(a.sort_order, 0);
        assert_eq!(b.sort_order, 1);
        assert_eq!(c.sort_order, 2);
    }

    #[tokio::test]
    async fn list_tag_groups_orders_by_sort_order() {
        let pool = setup_test_db().await;
        create_tag_group_impl(&pool, make_input("A")).await.unwrap();
        let b = create_tag_group_impl(&pool, make_input("B")).await.unwrap();
        create_tag_group_impl(&pool, make_input("C")).await.unwrap();
        let list = list_tag_groups_impl(&pool).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[1].id, b.id);
    }

    #[tokio::test]
    async fn update_tag_group_changes_name_and_color() {
        let pool = setup_test_db().await;
        let g = create_tag_group_impl(&pool, make_input("Old")).await.unwrap();
        let updated = update_tag_group_impl(
            &pool,
            &g.id,
            UpdateTagGroupInput {
                name: Some("New".into()),
                color: Some(Some("#ff0000".into())),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.color.as_deref(), Some("#ff0000"));
    }

    #[tokio::test]
    async fn update_tag_group_can_clear_color() {
        let pool = setup_test_db().await;
        let g = create_tag_group_impl(
            &pool,
            CreateTagGroupInput {
                name: "WithColor".into(),
                color: Some("#fff".into()),
            },
        )
        .await
        .unwrap();
        let updated = update_tag_group_impl(
            &pool,
            &g.id,
            UpdateTagGroupInput {
                color: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(updated.color.is_none());
    }

    #[tokio::test]
    async fn update_nonexistent_tag_group_fails() {
        let pool = setup_test_db().await;
        let err = update_tag_group_impl(
            &pool,
            "no-such",
            UpdateTagGroupInput {
                name: Some("X".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn delete_tag_group_removes_row_and_clears_member_tags_group_id() {
        let pool = setup_test_db().await;
        let group = create_tag_group_impl(&pool, make_input("G")).await.unwrap();
        let tag = db::create_skill_tag(&pool, "team-a", None, None).await.unwrap();
        set_tag_group_impl(&pool, &tag.id, Some(&group.id)).await.unwrap();

        delete_tag_group_impl(&pool, &group.id).await.unwrap();

        // 删除后 tag 仍存在但 group_id 应为 NULL
        let tag_after = db::get_skill_tag_by_id(&pool, &tag.id).await.unwrap().unwrap();
        assert!(tag_after.group_id.is_none(), "tag.group_id must be cleared");
        assert!(list_tag_groups_impl(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_nonexistent_tag_group_fails() {
        let pool = setup_test_db().await;
        let err = delete_tag_group_impl(&pool, "no-such").await.unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn reorder_tag_groups_updates_sort_order() {
        let pool = setup_test_db().await;
        let a = create_tag_group_impl(&pool, make_input("A")).await.unwrap();
        let b = create_tag_group_impl(&pool, make_input("B")).await.unwrap();
        let c = create_tag_group_impl(&pool, make_input("C")).await.unwrap();
        reorder_tag_groups_impl(&pool, vec![c.id.clone(), b.id.clone(), a.id.clone()])
            .await
            .unwrap();
        let list = list_tag_groups_impl(&pool).await.unwrap();
        assert_eq!(list[0].id, c.id);
        assert_eq!(list[1].id, b.id);
        assert_eq!(list[2].id, a.id);
    }

    #[tokio::test]
    async fn set_tag_group_assigns_tag_to_group() {
        let pool = setup_test_db().await;
        let group = create_tag_group_impl(&pool, make_input("G")).await.unwrap();
        let tag = db::create_skill_tag(&pool, "demo", None, None).await.unwrap();
        set_tag_group_impl(&pool, &tag.id, Some(&group.id)).await.unwrap();
        let after = db::get_skill_tag_by_id(&pool, &tag.id).await.unwrap().unwrap();
        assert_eq!(after.group_id.as_deref(), Some(group.id.as_str()));
    }

    #[tokio::test]
    async fn set_tag_group_can_clear_assignment() {
        let pool = setup_test_db().await;
        let group = create_tag_group_impl(&pool, make_input("G")).await.unwrap();
        let tag = db::create_skill_tag(&pool, "demo2", None, None).await.unwrap();
        set_tag_group_impl(&pool, &tag.id, Some(&group.id)).await.unwrap();
        set_tag_group_impl(&pool, &tag.id, None).await.unwrap();
        let after = db::get_skill_tag_by_id(&pool, &tag.id).await.unwrap().unwrap();
        assert!(after.group_id.is_none());
    }

    #[tokio::test]
    async fn set_tag_group_rejects_unknown_tag_or_group() {
        let pool = setup_test_db().await;
        let err1 = set_tag_group_impl(&pool, "missing-tag", None).await.unwrap_err();
        assert!(err1.contains("Tag"));
        let tag = db::create_skill_tag(&pool, "demo3", None, None).await.unwrap();
        let err2 = set_tag_group_impl(&pool, &tag.id, Some("missing-group"))
            .await
            .unwrap_err();
        assert!(err2.contains("group"));
    }
}
