//! Skill Usage schema：3 张支撑「技能调用统计」页面的表。
//!
//! 与 skilled 项目（ref/skilled/index/src/db.rs）保持同形：每行 `skill_calls`
//! 是一次 SkillCall（来自某个 AI 编码工具的会话日志），`skill_call_providers`
//! 记录每个 provider 在指定 target 上的最近一次扫描结果（available 与 call
//! count），`skill_call_scan_state` 记录每个 target 的最近全量扫描时间，给
//! 5 分钟缓存判定使用。
//!
//! `target_id` 在 P1 阶段只取常量 `'local'`；P2 接入远程 target 后会写入
//! 真实 target id。所有写入路径走「事务内 DELETE WHERE target_id=? + INSERT」
//! 的原子替换，避免读端拿到半成品。

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), String> {
    // skill_calls：单次 skill 调用记录。timestamp_ms 是 Unix epoch 毫秒，
    // 与 skilled 一致；这样按时间排序、计算 16 周热力图时不需要解析 TEXT。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_calls (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            target_id    TEXT NOT NULL DEFAULT 'local',
            skill        TEXT NOT NULL,
            timestamp_ms INTEGER NOT NULL,
            project      TEXT NOT NULL,
            session_id   TEXT NOT NULL,
            source       TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill 名是高频聚合 key，索引必须有；source 用于按 provider 过滤；
    // timestamp_ms 用于 recent / heatmap 范围查询；target_id 让多 target
    // 共存时按目标隔离。索引数量与 skilled 一致，没有冗余。
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_calls_skill
         ON skill_calls(skill)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_calls_source
         ON skill_calls(source)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_calls_ts
         ON skill_calls(timestamp_ms)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_calls_target
         ON skill_calls(target_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_call_providers：每个 (target, provider) 的最近健康状态。
    // call_count 冗余存以避免「打开 Skill Usage 页时再 COUNT」抖动。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_call_providers (
            target_id    TEXT NOT NULL DEFAULT 'local',
            provider_id  TEXT NOT NULL,
            display_name TEXT NOT NULL,
            available    INTEGER NOT NULL,
            call_count   INTEGER NOT NULL DEFAULT 0,
            scanned_at   INTEGER NOT NULL,
            PRIMARY KEY (target_id, provider_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_call_scan_state：每个 target 上一次完整扫描的时间戳。
    // 5 分钟缓存判定用：now - last_full_scan_ms < 300_000 ⇒ 跳过扫描。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_call_scan_state (
            target_id          TEXT NOT NULL PRIMARY KEY,
            last_full_scan_ms  INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
