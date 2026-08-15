//! `skill_call_file_cache` CRUD —— 增量扫描的按文件指纹缓存（migration 5）。
//!
//! 红线：这张表保存 provider 日志文件路径，属于「派生缓存」而非事实表。
//! 路径不得进入日志、IPC payload、Operation Log 或状态导出；本模块的错误
//! 直接透传 `sqlx::Error`，调用方不得把路径拼进错误文案或 tracing 字段。
//! `skill_calls` 依旧只存日志证明的事实，不放文件路径。

use sqlx::{FromRow, QueryBuilder, Sqlite};

use crate::db::types::DbPool;

/// 缓存表单行：一个 provider 日志文件的指纹 + 解析出的 calls（JSON 序列化）。
#[derive(Debug, Clone, FromRow)]
pub struct SkillCallFileCacheRow {
    pub file_path: String,
    pub mtime_ms: i64,
    pub size: i64,
    pub calls_json: String,
    pub scanned_at_ms: i64,
}

/// 写入侧的新行；`target_id` / `provider` / `scanned_at_ms` 由 repo 填充。
#[derive(Debug, Clone)]
pub struct NewSkillCallFileCache {
    pub file_path: String,
    pub mtime_ms: i64,
    pub size: i64,
    pub calls_json: String,
}

/// SQLite 绑定变量上限保守按 999 计：upsert 每行 7 个变量、delete 每行
/// 3 + N 个变量，分块远低于上限。
const UPSERT_CHUNK_ROWS: usize = 100;
const DELETE_CHUNK_ROWS: usize = 200;

/// 读出一个 (target, provider) 的全部缓存行。增量扫描开场载入指纹用。
pub async fn list_file_cache_rows(
    pool: &DbPool,
    target_id: &str,
    provider: &str,
) -> Result<Vec<SkillCallFileCacheRow>, sqlx::Error> {
    sqlx::query_as::<_, SkillCallFileCacheRow>(
        "SELECT file_path, mtime_ms, size, calls_json, scanned_at_ms
         FROM skill_call_file_cache
         WHERE target_id = ? AND provider = ?",
    )
    .bind(target_id)
    .bind(provider)
    .fetch_all(pool)
    .await
}

/// 批量 upsert 本次扫描新解析/有变化的文件行。
pub async fn upsert_file_cache_rows(
    pool: &DbPool,
    target_id: &str,
    provider: &str,
    rows: &[NewSkillCallFileCache],
    scanned_at_ms: i64,
) -> Result<(), sqlx::Error> {
    for chunk in rows.chunks(UPSERT_CHUNK_ROWS) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "INSERT OR REPLACE INTO skill_call_file_cache
             (target_id, provider, file_path, mtime_ms, size, calls_json, scanned_at_ms) ",
        );
        builder.push_values(chunk, |mut row, item| {
            row.push_bind(target_id)
                .push_bind(provider)
                .push_bind(&item.file_path)
                .push_bind(item.mtime_ms)
                .push_bind(item.size)
                .push_bind(&item.calls_json)
                .push_bind(scanned_at_ms);
        });
        builder.build().execute(pool).await?;
    }
    Ok(())
}

/// 删除盘上已消失文件的缓存行。返回实际删除的行数（测试观测用）。
pub async fn delete_file_cache_rows(
    pool: &DbPool,
    target_id: &str,
    provider: &str,
    file_paths: &[String],
) -> Result<u64, sqlx::Error> {
    let mut deleted = 0u64;
    for chunk in file_paths.chunks(DELETE_CHUNK_ROWS) {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "DELETE FROM skill_call_file_cache
             WHERE target_id = ",
        );
        builder
            .push_bind(target_id)
            .push(" AND provider = ")
            .push_bind(provider)
            .push(" AND file_path IN (");
        {
            let mut separated = builder.separated(", ");
            for path in chunk {
                separated.push_bind(path);
            }
        }
        builder.push(")");
        deleted += builder.build().execute(pool).await?.rows_affected();
    }
    Ok(deleted)
}

/// provider 数据源整体不可用（目录消失）时清空它的全部缓存行。
pub async fn delete_file_cache_for_provider(
    pool: &DbPool,
    target_id: &str,
    provider: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM skill_call_file_cache WHERE target_id = ? AND provider = ?")
        .bind(target_id)
        .bind(provider)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mem_pool;

    fn row(path: &str, mtime_ms: i64) -> NewSkillCallFileCache {
        NewSkillCallFileCache {
            file_path: path.to_string(),
            mtime_ms,
            size: 100,
            calls_json: "[]".to_string(),
        }
    }

    #[tokio::test]
    async fn upsert_list_and_delete_round_trip() {
        let pool = mem_pool().await;
        let rows: Vec<_> = (0..250)
            .map(|i| row(&format!("/sess/{i}.jsonl"), 1_000 + i))
            .collect();
        upsert_file_cache_rows(&pool, "local", "codex", &rows, 10)
            .await
            .unwrap();

        let stored = list_file_cache_rows(&pool, "local", "codex").await.unwrap();
        assert_eq!(stored.len(), 250, "chunked upsert must cover all rows");
        assert!(stored.iter().any(|r| r.file_path == "/sess/249.jsonl"));

        // 同 key 覆盖更新指纹
        upsert_file_cache_rows(&pool, "local", "codex", &[row("/sess/0.jsonl", 9_999)], 20)
            .await
            .unwrap();
        let stored = list_file_cache_rows(&pool, "local", "codex").await.unwrap();
        assert_eq!(stored.len(), 250);
        let updated = stored
            .iter()
            .find(|r| r.file_path == "/sess/0.jsonl")
            .unwrap();
        assert_eq!(updated.mtime_ms, 9_999);

        // target / provider 隔离
        let other = list_file_cache_rows(&pool, "ssh-prod", "codex")
            .await
            .unwrap();
        assert!(other.is_empty());

        let deleted =
            delete_file_cache_rows(&pool, "local", "codex", &["/sess/0.jsonl".to_string()])
                .await
                .unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(
            list_file_cache_rows(&pool, "local", "codex")
                .await
                .unwrap()
                .len(),
            249
        );

        delete_file_cache_for_provider(&pool, "local", "codex")
            .await
            .unwrap();
        assert!(list_file_cache_rows(&pool, "local", "codex")
            .await
            .unwrap()
            .is_empty());
    }
}
