//! `operation_logs` table CRUD + filter builder — Phase 2c.

use std::collections::HashMap;

use chrono::{Duration, Local, LocalResult, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row, Sqlite};
use uuid::Uuid;

use crate::db::types::{
    DbPool, NewOperationLogEntry, OperationLogEntry, OperationLogFilter, OperationLogPage,
};

pub(crate) const DEFAULT_OPERATION_LOG_LIMIT: i64 = 100;
pub(crate) const MAX_OPERATION_LOG_LIMIT: i64 = 500;

fn normalize_optional_filter(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_required_log_value(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn operation_log_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(DEFAULT_OPERATION_LOG_LIMIT)
        .clamp(1, MAX_OPERATION_LOG_LIMIT)
}

fn operation_log_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

#[derive(Debug, Clone)]
struct NormalizedOperationLogFilter {
    query: Option<String>,
    target_kind: Option<String>,
    target_id: Option<String>,
    level: Option<String>,
    status: Option<String>,
    category: Option<String>,
    action: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
}

impl From<&OperationLogFilter> for NormalizedOperationLogFilter {
    fn from(filter: &OperationLogFilter) -> Self {
        Self {
            query: normalize_optional_filter(&filter.query).map(|value| format!("%{}%", value)),
            target_kind: normalize_optional_filter(&filter.target_kind),
            target_id: normalize_optional_filter(&filter.target_id),
            level: normalize_optional_filter(&filter.level),
            status: normalize_optional_filter(&filter.status),
            category: normalize_optional_filter(&filter.category),
            action: normalize_optional_filter(&filter.action),
            created_after: normalize_optional_filter(&filter.created_after),
            created_before: normalize_optional_filter(&filter.created_before),
        }
    }
}

fn push_operation_log_filters(
    builder: &mut QueryBuilder<'_, Sqlite>,
    filter: &NormalizedOperationLogFilter,
) {
    builder.push(" WHERE 1 = 1");

    if let Some(value) = &filter.target_kind {
        builder.push(" AND target_kind = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.target_id {
        builder.push(" AND target_id = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.level {
        builder.push(" AND level = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.status {
        builder.push(" AND status = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.category {
        builder.push(" AND category = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.action {
        builder.push(" AND action = ").push_bind(value.clone());
    }
    if let Some(value) = &filter.created_after {
        builder.push(" AND created_at >= ").push_bind(value.clone());
    }
    if let Some(value) = &filter.created_before {
        builder.push(" AND created_at <= ").push_bind(value.clone());
    }
    if let Some(value) = &filter.query {
        builder.push(
            " AND (
                summary LIKE ",
        );
        builder.push_bind(value.clone());
        builder
            .push(" OR error_summary LIKE ")
            .push_bind(value.clone());
        builder
            .push(" OR subject_id LIKE ")
            .push_bind(value.clone());
        builder
            .push(" OR subject_label LIKE ")
            .push_bind(value.clone());
        builder.push(" OR action LIKE ").push_bind(value.clone());
        builder.push(")");
    }
}

pub async fn insert_operation_log(
    pool: &DbPool,
    entry: NewOperationLogEntry,
) -> Result<OperationLogEntry, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let level = normalize_required_log_value(&entry.level, "info");
    let target_kind = normalize_required_log_value(&entry.target_kind, "local");
    let target_id = normalize_required_log_value(&entry.target_id, "local");
    let category = normalize_required_log_value(&entry.category, "general");
    let action = normalize_required_log_value(&entry.action, "operation");
    let status = normalize_required_log_value(&entry.status, "succeeded");
    let summary = normalize_required_log_value(&entry.summary, "Operation completed.");

    sqlx::query(
        "INSERT INTO operation_logs (
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&created_at)
    .bind(level)
    .bind(target_kind)
    .bind(target_id)
    .bind(entry.target_label)
    .bind(category)
    .bind(action)
    .bind(status)
    .bind(entry.subject_type)
    .bind(entry.subject_id)
    .bind(entry.subject_label)
    .bind(summary)
    .bind(entry.error_summary)
    .bind(entry.details_json)
    .bind(entry.duration_ms)
    .bind(entry.batch_id)
    .execute(pool)
    .await?;

    get_operation_log(pool, &id).await?.ok_or_else(|| {
        sqlx::Error::InvalidArgument("Inserted operation log was not found.".to_string())
    })
}

pub async fn get_operation_log(
    pool: &DbPool,
    log_id: &str,
) -> Result<Option<OperationLogEntry>, sqlx::Error> {
    sqlx::query_as::<_, OperationLogEntry>(
        "SELECT
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
         FROM operation_logs
         WHERE id = ?",
    )
    .bind(log_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_operation_logs(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<OperationLogPage, sqlx::Error> {
    let normalized = NormalizedOperationLogFilter::from(&filter);
    let limit = operation_log_limit(filter.limit);
    let offset = operation_log_offset(filter.offset);

    let mut count_builder =
        QueryBuilder::<Sqlite>::new("SELECT COUNT(*) AS cnt FROM operation_logs");
    push_operation_log_filters(&mut count_builder, &normalized);
    let count_row = count_builder.build().fetch_one(pool).await?;
    let total = count_row.try_get::<i64, _>("cnt")?;

    let mut entries_builder = QueryBuilder::<Sqlite>::new(
        "SELECT
            id, created_at, level, target_kind, target_id, target_label,
            category, action, status, subject_type, subject_id, subject_label,
            summary, error_summary, details_json, duration_ms, batch_id
         FROM operation_logs",
    );
    push_operation_log_filters(&mut entries_builder, &normalized);
    entries_builder
        .push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let entries = entries_builder
        .build_query_as::<OperationLogEntry>()
        .fetch_all(pool)
        .await?;

    Ok(OperationLogPage {
        entries,
        total,
        limit,
        offset,
    })
}

pub async fn clear_operation_logs(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<u64, sqlx::Error> {
    let normalized = NormalizedOperationLogFilter::from(&filter);
    let mut builder = QueryBuilder::<Sqlite>::new("DELETE FROM operation_logs");
    push_operation_log_filters(&mut builder, &normalized);

    builder
        .build()
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationLogsExport {
    exported_at: String,
    filter: OperationLogFilter,
    total: i64,
    entries: Vec<OperationLogEntry>,
}

pub async fn export_operation_logs_json(
    pool: &DbPool,
    filter: OperationLogFilter,
) -> Result<String, sqlx::Error> {
    let page = list_operation_logs(
        pool,
        OperationLogFilter {
            limit: Some(MAX_OPERATION_LOG_LIMIT),
            offset: Some(0),
            ..filter.clone()
        },
    )
    .await?;

    serde_json::to_string_pretty(&OperationLogsExport {
        exported_at: Utc::now().to_rfc3339(),
        filter,
        total: page.total,
        entries: page.entries,
    })
    .map_err(|e| sqlx::Error::InvalidArgument(e.to_string()))
}

/// 仪表盘「每日操作数」聚合的一行：本地日历日 + 当天日志数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DailyOperationCount {
    pub date: String,
    pub count: u32,
}

const MAX_DAILY_OPERATION_DAYS: u32 = 60;

/// 把「本地日历日 00:00」换算成 UTC RFC3339，作为 `created_at >= ?` 的字符串
/// 字典序 cut-off（生产 `created_at` 全部由 `Utc::now().to_rfc3339()` 写入，
/// 两端同为 UTC `+00:00` 形式时字典序与时间序一致）。
///
/// DST 语义：分桶按本地日历日，过渡当天 23/25 小时仍计一天。
/// - 折叠（25h 天）：取本地午夜的第一次出现（最早瞬间），窗口起点不前移；
/// - 空洞（23h 天，本地 00:00 不存在）：取当天第一个存在的本地整点；
/// - 极端情形（整天被跳过，如 2011 Samoa 跳过 12-30）：顺延到下一有效日。
fn local_day_start_utc_rfc3339(date: NaiveDate) -> Result<String, sqlx::Error> {
    for day_offset in 0..=1 {
        let Some(day) = date.checked_add_signed(Duration::days(day_offset)) else {
            continue;
        };
        for hour in 0..24 {
            let Some(naive) = day.and_hms_opt(hour, 0, 0) else {
                continue;
            };
            match Local.from_local_datetime(&naive) {
                LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => {
                    return Ok(dt.with_timezone(&Utc).to_rfc3339());
                }
                LocalResult::None => continue,
            }
        }
    }
    Err(sqlx::Error::InvalidArgument(format!(
        "no valid local time on or after {date}"
    )))
}

/// 按本机本地日历日分桶统计 operation_logs，窗口 = `today` 起向前 `days - 1`
/// 天。`today` 由调用方注入（command 层取 `Local::now().date_naive()`），保证
/// 测试确定性。零值填充：恰好返回 `days` 个桶，日期升序（最旧在前）。
///
/// 分组用 SQLite `date(created_at, 'localtime')`，与 cut-off 的本地午夜换算
/// 同一时区来源（OS 本地时区），两端始终一致。
pub async fn list_daily_operation_counts(
    pool: &DbPool,
    today: NaiveDate,
    days: u32,
) -> Result<Vec<DailyOperationCount>, sqlx::Error> {
    let days = days.clamp(1, MAX_DAILY_OPERATION_DAYS);
    let window_start = today - Duration::days(i64::from(days) - 1);
    let cutoff_utc = local_day_start_utc_rfc3339(window_start)?;

    let rows = sqlx::query(
        "SELECT date(created_at, 'localtime') AS day, COUNT(*) AS count
         FROM operation_logs
         WHERE created_at >= ?
         GROUP BY day",
    )
    .bind(&cutoff_utc)
    .fetch_all(pool)
    .await?;

    let mut counts_by_day: HashMap<String, u32> = HashMap::new();
    for row in &rows {
        let day: String = row.try_get("day")?;
        let count: i64 = row.try_get("count")?;
        counts_by_day.insert(day, count.max(0) as u32);
    }

    let mut buckets = Vec::with_capacity(days as usize);
    for offset in 0..days {
        let date = window_start + Duration::days(i64::from(offset));
        let key = date.format("%Y-%m-%d").to_string();
        let count = counts_by_day.get(&key).copied().unwrap_or(0);
        buckets.push(DailyOperationCount { date: key, count });
    }
    Ok(buckets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mem_pool;

    fn fixed_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap()
    }

    /// 测试允许用字面 SQL 写入自定义 created_at（绕过 `insert_operation_log`
    /// 的 `Utc::now()`）。`created_at` 必须是 UTC RFC3339 `+00:00` 形式，与
    /// 生产写入格式一致，保证 `WHERE created_at >= ?` 字典序比较成立。
    async fn insert_log_at(pool: &DbPool, created_at: &str) {
        sqlx::query(
            "INSERT INTO operation_logs (
                id, created_at, level, target_kind, target_id, target_label,
                category, action, status, subject_type, subject_id, subject_label,
                summary, error_summary, details_json, duration_ms, batch_id
            ) VALUES (?, ?, 'info', 'local', 'local', NULL,
                'general', 'test.op', 'succeeded', NULL, NULL, NULL,
                'test', NULL, NULL, NULL, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();
    }

    /// 把「本地日 + 本地时刻」换算成 UTC RFC3339 字面量。用 `chrono::Local`
    /// 动态取本机时区——与 SQLite `localtime` 修饰符同源，因此测试在任意
    /// 时区的机器上都确定。DST 二义时刻取最早瞬间；所选日期（2026-07）避开
    /// 了所有已知时区的过渡日，本地空洞时刻实际不会出现。
    fn utc_rfc3339_at_local(date: NaiveDate, hour: u32, minute: u32) -> String {
        let naive = date.and_hms_opt(hour, minute, 0).unwrap();
        match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => {
                dt.with_timezone(&Utc).to_rfc3339()
            }
            LocalResult::None => panic!("local time {naive} must exist on this machine"),
        }
    }

    fn counts_by_date(buckets: &[DailyOperationCount]) -> HashMap<&str, u32> {
        buckets
            .iter()
            .map(|bucket| (bucket.date.as_str(), bucket.count))
            .collect()
    }

    #[tokio::test]
    async fn empty_table_returns_days_zero_filled_buckets_ascending() {
        let pool = mem_pool().await;
        let today = fixed_today();

        let buckets = list_daily_operation_counts(&pool, today, 7).await.unwrap();

        assert_eq!(buckets.len(), 7);
        assert!(buckets.iter().all(|bucket| bucket.count == 0));
        // 首桶 = today - (days - 1)，末桶 = today，严格升序。
        assert_eq!(buckets.first().unwrap().date, "2026-07-14");
        assert_eq!(buckets.last().unwrap().date, "2026-07-20");
        let dates: Vec<&str> = buckets.iter().map(|bucket| bucket.date.as_str()).collect();
        let mut sorted = dates.clone();
        sorted.sort_unstable();
        assert_eq!(dates, sorted);
    }

    /// UTC 已跨日但本地未跨日 → 计入本地今天。
    /// 本地今天 23:30：UTC- 时区机器上其 UTC 日期已是明天（UTC 已跨日）；
    /// UTC+ 机器上两者同日（UTC 机器上该分歧不存在，退化为普通情形）。
    /// 断言与机器时区无关：分桶键必须是本地日期。
    #[tokio::test]
    async fn utc_crossed_but_local_not_counts_in_local_today() {
        let pool = mem_pool().await;
        let today = fixed_today();
        insert_log_at(&pool, &utc_rfc3339_at_local(today, 23, 30)).await;

        let buckets = list_daily_operation_counts(&pool, today, 3).await.unwrap();

        let counts = counts_by_date(&buckets);
        assert_eq!(counts.get("2026-07-20"), Some(&1));
        assert_eq!(buckets.iter().map(|bucket| bucket.count).sum::<u32>(), 1);
    }

    /// 本地已跨日但 UTC 未跨日 → 计入本地今天。
    /// 本地今天 00:30：UTC+ 时区机器上其 UTC 日期仍是昨天（本地先跨日）。
    #[tokio::test]
    async fn local_crossed_but_utc_not_counts_in_local_today() {
        let pool = mem_pool().await;
        let today = fixed_today();
        insert_log_at(&pool, &utc_rfc3339_at_local(today, 0, 30)).await;

        let buckets = list_daily_operation_counts(&pool, today, 3).await.unwrap();

        let counts = counts_by_date(&buckets);
        assert_eq!(counts.get("2026-07-20"), Some(&1));
        assert_eq!(buckets.iter().map(|bucket| bucket.count).sum::<u32>(), 1);
    }

    /// 本地昨天 23:59：UTC- 机器上其 UTC 日期已是今天（UTC 已跨日但本地
    /// 未跨日），日志仍计入本地昨天。
    #[tokio::test]
    async fn utc_crossed_but_local_not_counts_in_local_yesterday() {
        let pool = mem_pool().await;
        let today = fixed_today();
        let yesterday = today - Duration::days(1);
        insert_log_at(&pool, &utc_rfc3339_at_local(yesterday, 23, 59)).await;

        let buckets = list_daily_operation_counts(&pool, today, 3).await.unwrap();

        let counts = counts_by_date(&buckets);
        assert_eq!(counts.get("2026-07-19"), Some(&1));
        assert_eq!(counts.get("2026-07-20"), Some(&0));
        assert_eq!(buckets.iter().map(|bucket| bucket.count).sum::<u32>(), 1);
    }

    #[tokio::test]
    async fn days_without_logs_are_zero_filled_within_window() {
        let pool = mem_pool().await;
        let today = fixed_today();
        let window_start = today - Duration::days(6);
        insert_log_at(&pool, &utc_rfc3339_at_local(window_start, 10, 0)).await;
        insert_log_at(&pool, &utc_rfc3339_at_local(window_start, 11, 0)).await;
        insert_log_at(&pool, &utc_rfc3339_at_local(today, 12, 0)).await;

        let buckets = list_daily_operation_counts(&pool, today, 7).await.unwrap();

        assert_eq!(buckets.len(), 7);
        let counts = counts_by_date(&buckets);
        assert_eq!(counts.get("2026-07-14"), Some(&2));
        assert_eq!(counts.get("2026-07-20"), Some(&1));
        for date in [
            "2026-07-15",
            "2026-07-16",
            "2026-07-17",
            "2026-07-18",
            "2026-07-19",
        ] {
            assert_eq!(counts.get(date), Some(&0), "{date} should be zero-filled");
        }
    }

    /// cut-off 边界：窗口起点本地 00:00 整点计入（`>=`），前一秒不计入。
    /// 注意：前一秒瞬间的本地日期必为窗口起点前一天，即使漏过 WHERE 也只
    /// 会落在返回窗口之外，因此本用例钉住的是「整点计入」语义；前一秒日志
    /// 在窗口放大到 8 天时必须重新出现在首桶，借此确认它确实只是被
    /// cut-off 排除而非被错误分桶。
    #[tokio::test]
    async fn cutoff_boundary_includes_exact_start_and_excludes_one_second_before() {
        let pool = mem_pool().await;
        let today = fixed_today();
        let window_start = today - Duration::days(6);
        let start_utc = local_day_start_utc_rfc3339(window_start).unwrap();
        let start_dt = chrono::DateTime::parse_from_rfc3339(&start_utc).unwrap();
        let one_second_before = (start_dt - Duration::seconds(1))
            .with_timezone(&Utc)
            .to_rfc3339();
        insert_log_at(&pool, &one_second_before).await;
        insert_log_at(&pool, &start_utc).await;

        let buckets = list_daily_operation_counts(&pool, today, 7).await.unwrap();
        assert_eq!(buckets.iter().map(|bucket| bucket.count).sum::<u32>(), 1);
        assert_eq!(buckets.first().unwrap().date, "2026-07-14");
        assert_eq!(buckets.first().unwrap().count, 1);

        // 窗口放大一天：前一秒日志的本地日进入窗口，计入首桶。
        let wider = list_daily_operation_counts(&pool, today, 8).await.unwrap();
        assert_eq!(wider.first().unwrap().date, "2026-07-13");
        assert_eq!(wider.first().unwrap().count, 1);
        assert_eq!(wider.iter().map(|bucket| bucket.count).sum::<u32>(), 2);
    }

    #[tokio::test]
    async fn days_is_clamped_to_1_and_60() {
        let pool = mem_pool().await;
        let today = fixed_today();

        let one = list_daily_operation_counts(&pool, today, 0).await.unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].date, "2026-07-20");

        let capped = list_daily_operation_counts(&pool, today, 999)
            .await
            .unwrap();
        assert_eq!(capped.len(), 60);
        assert_eq!(capped.first().unwrap().date, "2026-05-22"); // today - 59
        assert_eq!(capped.last().unwrap().date, "2026-07-20");
    }
}
