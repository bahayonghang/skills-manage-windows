//! Local scope 的增量收集编排 —— `skill_call_file_cache` 的读取与持久化 diff。
//!
//! 从 `refresh_with_providers` 拆出，保持 `mod.rs` 在建模规模预算内。

use chrono::Utc;

use super::file_cache::ProviderFileCache;
use super::{Scope, SkillCall, UsageError, UsageProvider};
use crate::db::{self, DbPool};

/// Local scope 的增量收集：载入指纹缓存 → provider 增量收集 → 持久化 diff
/// （删除消失文件的缓存行 + upsert 新解析行）。
///
/// 缓存是派生数据：缓存读取/持久化失败只记 warn 并退化为全量扫描，绝不
/// 拖垮本次 refresh。日志只带 provider id 与 sqlx 错误，不含文件路径
/// （`skill_call_file_cache` 是首个含路径的 usage 侧表，遵循 redaction 约定）。
pub(super) async fn collect_local_incremental(
    pool: &DbPool,
    target_id: &str,
    provider: &dyn UsageProvider,
    scope: &Scope,
) -> Result<Vec<SkillCall>, UsageError> {
    let cache = match db::list_file_cache_rows(pool, target_id, provider.id()).await {
        Ok(rows) => ProviderFileCache::from_rows(rows),
        Err(error) => {
            tracing::warn!(
                provider = provider.id(),
                error = %error,
                "usage file cache load failed; falling back to full scan"
            );
            ProviderFileCache::default()
        }
    };

    let (cache, calls) = provider.collect_incremental(scope, cache).await?;

    let vanished = cache.vanished_paths();
    if !vanished.is_empty() {
        if let Err(error) =
            db::delete_file_cache_rows(pool, target_id, provider.id(), &vanished).await
        {
            tracing::warn!(
                provider = provider.id(),
                error = %error,
                "usage file cache prune failed"
            );
        }
    }

    if !cache.upserts().is_empty() {
        let upserts: Vec<db::NewSkillCallFileCache> = cache
            .upserts()
            .iter()
            .map(|item| db::NewSkillCallFileCache {
                file_path: item.file_path.clone(),
                mtime_ms: item.mtime_ms,
                size: item.size,
                calls_json: serde_json::to_string(&item.calls).unwrap_or_else(|_| "[]".to_string()),
            })
            .collect();
        let scanned_at_ms = Utc::now().timestamp_millis();
        if let Err(error) =
            db::upsert_file_cache_rows(pool, target_id, provider.id(), &upserts, scanned_at_ms)
                .await
        {
            tracing::warn!(
                provider = provider.id(),
                error = %error,
                "usage file cache upsert failed"
            );
        }
    }

    Ok(calls)
}
