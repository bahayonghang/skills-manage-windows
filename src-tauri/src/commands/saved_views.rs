//! Saved Views commands — Central Skills V2 / M2.
//!
//! 5 个 IPC：list / create / update / delete / reorder。
//! `query` 字段是前端 `CentralViewState` 的 JSON，后端透传不解析。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{self, DbPool, SavedView};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationSubjectKind, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::targets::ActiveTarget;
use crate::AppState;

use super::serde_helpers::deserialize_optional_optional_string;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedViewInput {
    pub name: String,
    pub query: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSavedViewInput {
    pub name: Option<String>,
    pub query: Option<String>,
    /// `Some(None)` 表示清空 icon；`None` 表示不变。前端发 `null` 字面量时 serde
    /// 会反序列化成 `Some(None)`，而省略字段反序列化为 `None`。
    #[serde(default, deserialize_with = "deserialize_optional_optional_string")]
    pub icon: Option<Option<String>>,
    pub pinned: Option<bool>,
}

// ─── impl layer (pool-driven, used by tests and command wrappers) ────────────

pub async fn list_saved_views_impl(pool: &DbPool) -> Result<Vec<SavedView>, String> {
    db::list_saved_views(pool).await.map_err(|e| e.to_string())
}

pub async fn create_saved_view_impl(
    pool: &DbPool,
    input: CreateSavedViewInput,
) -> Result<SavedView, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Saved view name cannot be empty".to_string());
    }
    if input.query.trim().is_empty() {
        return Err("Saved view query cannot be empty".to_string());
    }
    db::create_saved_view(
        pool,
        db::NewSavedView {
            name,
            query: &input.query,
            icon: input.icon.as_deref(),
            pinned: input.pinned,
        },
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_saved_view_impl(
    pool: &DbPool,
    id: &str,
    input: UpdateSavedViewInput,
) -> Result<SavedView, String> {
    if let Some(ref name) = input.name {
        if name.trim().is_empty() {
            return Err("Saved view name cannot be empty".to_string());
        }
    }
    if let Some(ref query) = input.query {
        if query.trim().is_empty() {
            return Err("Saved view query cannot be empty".to_string());
        }
    }
    let icon_patch: Option<Option<&str>> = input.icon.as_ref().map(|opt| opt.as_deref());
    db::update_saved_view(
        pool,
        id,
        db::SavedViewPatch {
            name: input.name.as_deref(),
            query: input.query.as_deref(),
            icon: icon_patch,
            pinned: input.pinned,
        },
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn delete_saved_view_impl(pool: &DbPool, id: &str) -> Result<(), String> {
    if db::get_saved_view(pool, id)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err(format!("Saved view '{id}' not found"));
    }
    db::delete_saved_view(pool, id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn reorder_saved_views_impl(pool: &DbPool, ids: Vec<String>) -> Result<(), String> {
    db::reorder_saved_views(pool, &ids)
        .await
        .map_err(|e| e.to_string())
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_saved_views(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SavedView>> {
    crate::ipc_boundary!(
        "list_saved_views",
        async move {
            let pool = state.active_db().await?;
            list_saved_views_impl(&pool).await
        }
        .await
    )
}

#[tauri::command]
pub async fn create_saved_view(
    state: State<'_, AppState>,
    input: CreateSavedViewInput,
) -> crate::ipc_error::IpcResult<SavedView> {
    crate::ipc_boundary_async!("create_saved_view", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("create_saved_view")
            .expect("create_saved_view must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("create_saved_view must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            |view: &SavedView| {
                SafeOperationResult::succeeded("Saved view created.")
                    .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&view.id))
            },
            || async move {
                create_saved_view_impl(&pool, input)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn update_saved_view(
    state: State<'_, AppState>,
    id: String,
    input: UpdateSavedViewInput,
) -> crate::ipc_error::IpcResult<SavedView> {
    crate::ipc_boundary_async!("update_saved_view", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("update_saved_view")
            .expect("update_saved_view must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("update_saved_view must be auditable")
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::SavedView, SafeIdentifier::new(&id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Saved view updated."),
            || async move {
                update_saved_view_impl(&pool, &id, input)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn delete_saved_view(
    state: State<'_, AppState>,
    id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("delete_saved_view", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let entry = crate::ipc_registry::command_policy("delete_saved_view")
            .expect("delete_saved_view must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("delete_saved_view must be auditable")
        };
        let context = OperationContext::new(audit_target)
            .subject(OperationSubjectKind::SavedView, SafeIdentifier::new(&id));
        crate::observability::run_operation(
            &state,
            definition,
            context,
            |_| SafeOperationResult::succeeded("Saved view deleted."),
            || async move {
                delete_saved_view_impl(&pool, &id)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

#[tauri::command]
pub async fn reorder_saved_views(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("reorder_saved_views", {
        let request_context = state.resolve_target_context().await?;
        let audit_target = match request_context.target() {
            ActiveTarget::Local => OperationTarget::local(),
            ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
            ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        };
        let pool = request_context.db().clone();
        let requested_count = ids.len() as u64;
        let entry = crate::ipc_registry::command_policy("reorder_saved_views")
            .expect("reorder_saved_views must be registered");
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            unreachable!("reorder_saved_views must be auditable")
        };
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(audit_target),
            move |_| {
                SafeOperationResult::succeeded("Saved views reordered.")
                    .count(SafeDetailKey::RequestedCount, requested_count)
            },
            || async move {
                reorder_saved_views_impl(&pool, ids)
                    .await
                    .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::test_support::mem_pool as setup_test_db;

    fn test_app_state(pool: DbPool) -> AppState {
        AppState {
            db: pool,
            ai_tag_jobs: crate::AiTagJobRegistry::default(),
            central_update_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.central_update_busy",
                "A Central update job is already running.",
            ),
            central_update_snapshots: crate::CentralUpdateSnapshotCache::default(),
            portable_state_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.portability_busy",
                "A portability job is already running.",
            ),
            skills_cli_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.skills_cli_busy",
                "A Skills CLI job is already running.",
            ),
            secrets: std::sync::Arc::new(crate::secrets::MockSecretStore::default()),
            targets: crate::targets::TargetRegistry::default(),
        }
    }

    fn make_input(name: &str, query: &str) -> CreateSavedViewInput {
        CreateSavedViewInput {
            name: name.to_string(),
            query: query.to_string(),
            icon: None,
            pinned: false,
        }
    }

    #[tokio::test]
    async fn create_saved_view_returns_row_with_generated_id_and_timestamps() {
        let pool = setup_test_db().await;
        let view = create_saved_view_impl(&pool, make_input("Recent", "{\"q\":\"\"}"))
            .await
            .unwrap();
        assert_eq!(view.name, "Recent");
        assert_eq!(view.query, "{\"q\":\"\"}");
        assert!(!view.id.is_empty());
        assert!(!view.created_at.is_empty());
        assert_eq!(view.sort_order, 0);
        assert!(!view.pinned);
    }

    #[tokio::test]
    async fn create_saved_view_rejects_empty_name() {
        let pool = setup_test_db().await;
        let err = create_saved_view_impl(&pool, make_input("   ", "{}"))
            .await
            .unwrap_err();
        assert!(err.contains("name"));
    }

    #[tokio::test]
    async fn create_saved_view_rejects_empty_query() {
        let pool = setup_test_db().await;
        let err = create_saved_view_impl(&pool, make_input("View", "   "))
            .await
            .unwrap_err();
        assert!(err.contains("query"));
    }

    #[tokio::test]
    async fn list_saved_views_orders_pinned_first_then_sort_order() {
        let pool = setup_test_db().await;
        let a = create_saved_view_impl(&pool, make_input("A", "{}"))
            .await
            .unwrap();
        let b = create_saved_view_impl(&pool, make_input("B", "{}"))
            .await
            .unwrap();
        let c = create_saved_view_impl(&pool, make_input("C", "{}"))
            .await
            .unwrap();

        // Pin B
        update_saved_view_impl(
            &pool,
            &b.id,
            UpdateSavedViewInput {
                pinned: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let list = list_saved_views_impl(&pool).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, b.id, "pinned first");
        // A 与 C 按 sort_order 升序：A 先创建（order 0），C 后（order 2）
        assert_eq!(list[1].id, a.id);
        assert_eq!(list[2].id, c.id);
    }

    #[tokio::test]
    async fn create_saved_view_assigns_increasing_sort_order() {
        let pool = setup_test_db().await;
        let a = create_saved_view_impl(&pool, make_input("A", "{}"))
            .await
            .unwrap();
        let b = create_saved_view_impl(&pool, make_input("B", "{}"))
            .await
            .unwrap();
        let c = create_saved_view_impl(&pool, make_input("C", "{}"))
            .await
            .unwrap();
        assert_eq!(a.sort_order, 0);
        assert_eq!(b.sort_order, 1);
        assert_eq!(c.sort_order, 2);
    }

    #[tokio::test]
    async fn update_saved_view_changes_name_query_icon_pinned() {
        let pool = setup_test_db().await;
        let v = create_saved_view_impl(&pool, make_input("Old", "{}"))
            .await
            .unwrap();

        let updated = update_saved_view_impl(
            &pool,
            &v.id,
            UpdateSavedViewInput {
                name: Some("New".into()),
                query: Some("{\"q\":\"x\"}".into()),
                icon: Some(Some("star".into())),
                pinned: Some(true),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "New");
        assert_eq!(updated.query, "{\"q\":\"x\"}");
        assert_eq!(updated.icon.as_deref(), Some("star"));
        assert!(updated.pinned);
        // updated_at 必须比 created_at 同一时刻或更晚（时序粒度毫秒）
        assert!(updated.updated_at >= v.created_at);
    }

    #[tokio::test]
    async fn update_saved_view_can_clear_icon_with_some_none() {
        let pool = setup_test_db().await;
        let v = create_saved_view_impl(
            &pool,
            CreateSavedViewInput {
                name: "WithIcon".into(),
                query: "{}".into(),
                icon: Some("star".into()),
                pinned: false,
            },
        )
        .await
        .unwrap();

        let updated = update_saved_view_impl(
            &pool,
            &v.id,
            UpdateSavedViewInput {
                icon: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(updated.icon.is_none(), "icon should be cleared");
    }

    #[tokio::test]
    async fn update_saved_view_rejects_empty_name() {
        let pool = setup_test_db().await;
        let v = create_saved_view_impl(&pool, make_input("V", "{}"))
            .await
            .unwrap();
        let err = update_saved_view_impl(
            &pool,
            &v.id,
            UpdateSavedViewInput {
                name: Some("   ".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("name"));
    }

    #[tokio::test]
    async fn update_nonexistent_saved_view_fails() {
        let pool = setup_test_db().await;
        let err = update_saved_view_impl(
            &pool,
            "no-such-id",
            UpdateSavedViewInput {
                name: Some("X".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn delete_saved_view_removes_row() {
        let pool = setup_test_db().await;
        let v = create_saved_view_impl(&pool, make_input("Doomed", "{}"))
            .await
            .unwrap();
        delete_saved_view_impl(&pool, &v.id).await.unwrap();
        let list = list_saved_views_impl(&pool).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn delete_nonexistent_saved_view_fails() {
        let pool = setup_test_db().await;
        let err = delete_saved_view_impl(&pool, "no-such-id")
            .await
            .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn reorder_saved_views_updates_sort_order_to_index() {
        let pool = setup_test_db().await;
        let a = create_saved_view_impl(&pool, make_input("A", "{}"))
            .await
            .unwrap();
        let b = create_saved_view_impl(&pool, make_input("B", "{}"))
            .await
            .unwrap();
        let c = create_saved_view_impl(&pool, make_input("C", "{}"))
            .await
            .unwrap();

        // 翻转顺序：C, B, A
        reorder_saved_views_impl(&pool, vec![c.id.clone(), b.id.clone(), a.id.clone()])
            .await
            .unwrap();

        let list = list_saved_views_impl(&pool).await.unwrap();
        assert_eq!(list[0].id, c.id);
        assert_eq!(list[1].id, b.id);
        assert_eq!(list[2].id, a.id);
        assert_eq!(list[0].sort_order, 0);
        assert_eq!(list[1].sort_order, 1);
        assert_eq!(list[2].sort_order, 2);
    }

    #[tokio::test]
    async fn reorder_partial_list_only_updates_listed_ids() {
        let pool = setup_test_db().await;
        let a = create_saved_view_impl(&pool, make_input("A", "{}"))
            .await
            .unwrap();
        let _b = create_saved_view_impl(&pool, make_input("B", "{}"))
            .await
            .unwrap();
        let c = create_saved_view_impl(&pool, make_input("C", "{}"))
            .await
            .unwrap();

        // 只把 C 提到最前，A 退到位置 1。B 保持原 sort_order=1。
        reorder_saved_views_impl(&pool, vec![c.id.clone(), a.id.clone()])
            .await
            .unwrap();

        let c_row = db::get_saved_view(&pool, &c.id).await.unwrap().unwrap();
        let a_row = db::get_saved_view(&pool, &a.id).await.unwrap().unwrap();
        assert_eq!(c_row.sort_order, 0);
        assert_eq!(a_row.sort_order, 1);
    }

    #[tokio::test]
    async fn audited_saved_view_success_and_failure_never_persist_name_query_or_icon() {
        let pool = setup_test_db().await;
        let state = test_app_state(pool.clone());
        let entry = crate::ipc_registry::command_policy("create_saved_view").unwrap();
        let CommandLogPolicy::Operation(definition) = entry.policy else {
            panic!("create_saved_view must be auditable")
        };
        let planted_name = "private-view-name";
        let planted_query = r#"{"path":"C:\\Users\\alice\\secret","token":"ghp_private"}"#;
        let planted_icon = "private-icon";
        let success_pool = pool.clone();
        crate::observability::run_operation(
            &state,
            definition,
            OperationTarget::local(),
            |view: &SavedView| {
                SafeOperationResult::succeeded("Saved view created.")
                    .identifier(SafeDetailKey::Identifier, SafeIdentifier::new(&view.id))
            },
            || async move {
                create_saved_view_impl(
                    &success_pool,
                    CreateSavedViewInput {
                        name: planted_name.to_string(),
                        query: planted_query.to_string(),
                        icon: Some(planted_icon.to_string()),
                        pinned: false,
                    },
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
        .unwrap();

        let failure_pool = pool.clone();
        let failure = crate::observability::run_operation(
            &state,
            definition,
            OperationTarget::local(),
            |_| SafeOperationResult::succeeded("Saved view created."),
            || async move {
                create_saved_view_impl(
                    &failure_pool,
                    CreateSavedViewInput {
                        name: " ".to_string(),
                        query: planted_query.to_string(),
                        icon: Some(planted_icon.to_string()),
                        pinned: false,
                    },
                )
                .await
                .map_err(|_| ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition)))
            },
        )
        .await
        .unwrap_err();
        assert_eq!(failure.code, "internal.unexpected");
        assert!(failure.correlation_id.is_some());

        let page = db::list_operation_logs(&pool, db::OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.entries.len(), 2, "one terminal row per attempt");
        let serialized = serde_json::to_string(&page.entries).unwrap();
        for planted in [planted_name, planted_query, planted_icon, "ghp_private"] {
            assert!(!serialized.contains(planted));
        }
    }
}
