//! 聚合层 —— 把 `Vec<SkillCallRow>` 派生为各种 UI 形状。
//!
//! 全部纯函数 + 内存计算，不碰 IO；这样切换 sort / 过滤时前端可以
//! 毫秒级重算。完整实现在 Task 4，本文件先放下游需要的类型定义。

use serde::{Deserialize, Serialize};

use crate::db::SkillCallRow;

/// 单个 skill 的频次聚合结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCount {
    pub skill: String,
    pub count: i64,
    pub projects: i64,
    pub sessions: i64,
    pub last_used_ms: i64,
}

/// 一日的调用数（用于 16 周热力图与每日趋势）。`date` 为 ISO `YYYY-MM-DD`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

/// 顶部 KPI 行的 4 个数字。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageKpis {
    pub total_calls: i64,
    pub unique_skills: i64,
    pub unique_projects: i64,
    pub unique_sources: i64,
    pub unique_sessions: i64,
}

/// `usage_get_overview` 的返回包。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageOverview {
    pub kpis: UsageKpis,
    pub top_skills: Vec<SkillCount>,
    pub heatmap: Vec<DayCount>,
    pub last_scan_ms: Option<i64>,
}

/// 计算 KPI —— Task 4 会被聚合查询调用，这里先有定义可调用。
pub fn kpis_from_rows(rows: &[SkillCallRow]) -> UsageKpis {
    use std::collections::HashSet;
    let mut skills = HashSet::new();
    let mut projects = HashSet::new();
    let mut sources = HashSet::new();
    let mut sessions = HashSet::new();
    for r in rows {
        skills.insert(r.skill.as_str());
        projects.insert(r.project.as_str());
        sources.insert(r.source.as_str());
        sessions.insert(r.session_id.as_str());
    }
    UsageKpis {
        total_calls: rows.len() as i64,
        unique_skills: skills.len() as i64,
        unique_projects: projects.len() as i64,
        unique_sources: sources.len() as i64,
        unique_sessions: sessions.len() as i64,
    }
}

/// Top N skill 频次 —— UI 柱状图主数据。
///
/// 排序：count 倒序优先；count 相同按 last_used_ms 倒序；再相同按 skill 名升序。
/// limit=0 返回所有。
pub fn top_skills_from_rows(rows: &[SkillCallRow], limit: usize) -> Vec<SkillCount> {
    use std::collections::{HashMap, HashSet};

    struct Bucket {
        count: i64,
        projects: HashSet<String>,
        sessions: HashSet<String>,
        last_used: i64,
    }
    let mut map: HashMap<&str, Bucket> = HashMap::new();
    for r in rows {
        let b = map.entry(r.skill.as_str()).or_insert_with(|| Bucket {
            count: 0,
            projects: HashSet::new(),
            sessions: HashSet::new(),
            last_used: 0,
        });
        b.count += 1;
        b.projects.insert(r.project.clone());
        b.sessions.insert(r.session_id.clone());
        if r.timestamp_ms > b.last_used {
            b.last_used = r.timestamp_ms;
        }
    }

    let mut result: Vec<SkillCount> = map
        .into_iter()
        .map(|(skill, b)| SkillCount {
            skill: skill.to_string(),
            count: b.count,
            projects: b.projects.len() as i64,
            sessions: b.sessions.len() as i64,
            last_used_ms: b.last_used,
        })
        .collect();

    result.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(b.last_used_ms.cmp(&a.last_used_ms))
            .then(a.skill.cmp(&b.skill))
    });
    if limit > 0 && result.len() > limit {
        result.truncate(limit);
    }
    result
}

/// 每日调用数（升序日期）。给前端组合周/月趋势。
pub fn daily_counts(rows: &[SkillCallRow]) -> Vec<DayCount> {
    use chrono::{DateTime, Utc};
    use std::collections::BTreeMap;

    let mut map: BTreeMap<String, i64> = BTreeMap::new();
    for r in rows {
        let dt = DateTime::<Utc>::from_timestamp_millis(r.timestamp_ms).unwrap_or_default();
        let date = dt.format("%Y-%m-%d").to_string();
        *map.entry(date).or_insert(0) += 1;
    }
    map.into_iter()
        .map(|(date, count)| DayCount { date, count })
        .collect()
}

/// 基于已聚合好的每日计数补齐 16 周热力图网格。
pub fn heatmap_grid_16w_from_daily_counts(days: &[DayCount], now_ms: i64) -> Vec<DayCount> {
    use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
    use std::collections::HashMap;

    let day_count: HashMap<&str, i64> = days.iter().map(|d| (d.date.as_str(), d.count)).collect();

    let now = DateTime::<Utc>::from_timestamp_millis(now_ms).unwrap_or_default();
    // weekday: Monday=0..Sunday=6（与 skilled 的 `(getUTCDay()+6)%7` 一致）
    let today_dow = (now.weekday().num_days_from_sunday() + 6) % 7;
    let total_back_days = (15 * 7 + today_dow) as i64;
    let start_date: NaiveDate = (now - Duration::days(total_back_days)).date_naive();

    let mut grid: Vec<DayCount> = Vec::with_capacity(16 * 7);
    for i in 0..(16 * 7) {
        let d = start_date + Duration::days(i as i64);
        let key = d.format("%Y-%m-%d").to_string();
        let count = day_count.get(key.as_str()).copied().unwrap_or(0);
        grid.push(DayCount { date: key, count });
    }

    grid
}

/// 16 周 × 7 天的热力图密集矩阵：返回 112 个 DayCount，按日期升序。
/// 没有调用的日期 count=0；保证前端永远拿到完整 16w*7d 网格不用补 0。
///
/// 起点：以 `now_ms` 当日所在周的周一为基准，往前推 (16-1)=15 周得到首日。
/// 这与 skilled `buildHeatmapGrid` 完全一致。
pub fn heatmap_grid_16w(rows: &[SkillCallRow], now_ms: i64) -> Vec<DayCount> {
    let days = daily_counts(rows);
    heatmap_grid_16w_from_daily_counts(&days, now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(skill: &str, project: &str, source: &str, ts: i64) -> SkillCallRow {
        SkillCallRow {
            id: 0,
            target_id: "local".to_string(),
            skill: skill.to_string(),
            timestamp_ms: ts,
            project: project.to_string(),
            session_id: "s".to_string(),
            source: source.to_string(),
        }
    }

    #[test]
    fn kpis_count_unique_dimensions() {
        // 两条同会话(s1) + 一条新会话(s2) → unique_sessions=2，区别于 total_calls=3
        let mut r1 = row("a", "/p1", "Claude Code", 1);
        r1.session_id = "s1".into();
        let mut r2 = row("a", "/p1", "Claude Code", 2);
        r2.session_id = "s1".into();
        let mut r3 = row("b", "/p2", "Codex CLI", 3);
        r3.session_id = "s2".into();
        let rows = vec![r1, r2, r3];
        let k = kpis_from_rows(&rows);
        assert_eq!(k.total_calls, 3);
        assert_eq!(k.unique_skills, 2);
        assert_eq!(k.unique_projects, 2);
        assert_eq!(k.unique_sources, 2);
        assert_eq!(k.unique_sessions, 2);
    }

    #[test]
    fn top_skills_sorts_by_count_then_recent_then_name() {
        let rows = vec![
            row("alpha", "/p", "A", 100),
            row("beta", "/p", "A", 50),
            row("beta", "/p", "A", 60),
            row("gamma", "/p", "A", 200),
            row("gamma", "/p", "A", 300),
            row("alpha", "/p", "A", 400), // alpha 总计 2，最近 ts=400
        ];
        // alpha=2(last 400), beta=2(last 60), gamma=2(last 300)
        // 期望排序：alpha (count 2, last 400), gamma (count 2, last 300), beta (count 2, last 60)
        let top = top_skills_from_rows(&rows, 0);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].skill, "alpha");
        assert_eq!(top[1].skill, "gamma");
        assert_eq!(top[2].skill, "beta");

        // limit
        let top2 = top_skills_from_rows(&rows, 2);
        assert_eq!(top2.len(), 2);
    }

    #[test]
    fn top_skills_aggregates_projects_and_sessions_unique() {
        let mut rows = Vec::new();
        let mut r1 = row("x", "/p1", "A", 1);
        r1.session_id = "s1".into();
        rows.push(r1);
        let mut r2 = row("x", "/p1", "A", 2);
        r2.session_id = "s1".into();
        rows.push(r2);
        let mut r3 = row("x", "/p2", "A", 3);
        r3.session_id = "s2".into();
        rows.push(r3);

        let top = top_skills_from_rows(&rows, 0);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].count, 3);
        assert_eq!(top[0].projects, 2);
        assert_eq!(top[0].sessions, 2);
        assert_eq!(top[0].last_used_ms, 3);
    }

    #[test]
    fn daily_counts_groups_by_utc_date() {
        // 2024-01-01 00:00:00 UTC = 1704067200000
        let day1_morning = 1_704_067_200_000;
        let day1_evening = day1_morning + 12 * 3600 * 1000;
        let day2_morning = day1_morning + 24 * 3600 * 1000;
        let rows = vec![
            row("a", "/p", "A", day1_morning),
            row("b", "/p", "A", day1_evening),
            row("c", "/p", "A", day2_morning),
        ];
        let dc = daily_counts(&rows);
        assert_eq!(dc.len(), 2);
        assert_eq!(dc[0].date, "2024-01-01");
        assert_eq!(dc[0].count, 2);
        assert_eq!(dc[1].date, "2024-01-02");
        assert_eq!(dc[1].count, 1);
    }

    #[test]
    fn heatmap_grid_returns_16w_x_7d_dense() {
        let now_ms = 1_704_067_200_000; // 2024-01-01T00:00:00Z (Monday)
                                        // 一个 100 天前的调用应当落到网格内（如果 100 天 < 16w*7d=112d）
        let in_window = now_ms - 100 * 86_400_000;
        let out_window = now_ms - 200 * 86_400_000;
        let rows = vec![
            row("a", "/p", "A", in_window),
            row("a", "/p", "A", in_window),
            row("a", "/p", "A", out_window), // 应当不被计入
        ];
        let grid = heatmap_grid_16w(&rows, now_ms);
        assert_eq!(grid.len(), 16 * 7, "must be exactly 112 cells");

        let total: i64 = grid.iter().map(|d| d.count).sum();
        // out_window 不应该被计入；只有 in_window 的两次
        assert_eq!(total, 2);

        // 网格按日期升序，连续不跳
        for i in 1..grid.len() {
            assert!(grid[i - 1].date <= grid[i].date);
        }
    }
}
