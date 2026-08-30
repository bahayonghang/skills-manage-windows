//! Droid (Factory) provider —— `~/.factory/sessions/**/*.jsonl`
//!
//! 与 Codex 类似，但 skill 触发是从 `tool_result` 内的文本里
//! 正则匹配 `Skill "X" is now active` 串提取出来。`session_start`
//! 行给 `session_id` 与 `project`。
//!
//! 性能形态（08-15-usage-page-loading-perf）：
//! - Local scope：walk + 读盘 + 解析在单个 blocking 闭包内逐文件流式完成；
//! - 行级子串预过滤：只解析含 `session_start`（提供 session_id/project，
//!   不可丢弃）或 `is now active`（skill 触发的必要子串）的行——本机无
//!   droid 数据，等价性由 fixture 级参考实现测试锁定；
//! - 指纹未变的文件直接取 `ProviderFileCache` 缓存，零磁盘 IO。

use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::services::usage::file_cache::{fingerprint_from_metadata, ProviderFileCache};
use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

const SOURCE: &str = "Droid CLI";

fn active_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"Skill "([^"]+)" is now active"#).unwrap())
}

pub struct DroidProvider;

impl DroidProvider {
    fn sessions_dir(scope: &Scope) -> String {
        scope.join_home(&[".factory", "sessions"])
    }

    async fn collect_with_cache(
        &self,
        scope: &Scope,
        cache: ProviderFileCache,
    ) -> Result<(ProviderFileCache, Vec<SkillCall>), UsageError> {
        if scope.is_remote() {
            // Remote 走既有 FsBackend 批读路径，不进增量缓存。
            let calls = collect_remote(scope).await?;
            return Ok((cache, calls));
        }

        let sessions_dir = Self::sessions_dir(scope);
        crate::fs_util::run_blocking_fs_with(
            "droid session scan",
            move || Ok(scan_local(&sessions_dir, cache)),
            UsageError::task_join,
        )
        .await
    }
}

#[async_trait]
impl UsageProvider for DroidProvider {
    fn id(&self) -> &'static str {
        "droid"
    }
    fn display_name(&self) -> &'static str {
        SOURCE
    }

    async fn available(&self, scope: &Scope) -> bool {
        let backend = scope.fs_backend();
        backend.exists(&Self::sessions_dir(scope)).await
    }

    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        let (_, calls) = self
            .collect_with_cache(scope, ProviderFileCache::default())
            .await?;
        Ok(calls)
    }

    async fn collect_incremental(
        &self,
        scope: &Scope,
        cache: ProviderFileCache,
    ) -> Result<(ProviderFileCache, Vec<SkillCall>), UsageError> {
        self.collect_with_cache(scope, cache).await
    }
}

/// Local 扫描主体（blocking 闭包内）：逐文件流式处理，指纹命中走缓存。
fn scan_local(
    sessions_dir: &str,
    mut cache: ProviderFileCache,
) -> (ProviderFileCache, Vec<SkillCall>) {
    let re = active_re();
    let mut calls = Vec::new();
    let root = PathBuf::from(sessions_dir);
    if !root.is_dir() {
        return (cache, calls);
    }

    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") || !path.is_file() {
            continue;
        }
        let path_string = path.to_string_lossy().into_owned();
        let (mtime_ms, size) = fingerprint_from_metadata(entry.metadata());
        if let Some(cached_calls) = cache.lookup(&path_string, mtime_ms, size) {
            calls.extend(cached_calls);
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let file_calls = parse_session_calls(&content, re);
        cache.record(path_string, mtime_ms, size, file_calls.clone());
        calls.extend(file_calls);
    }

    (cache, calls)
}

/// Remote（SSH/WSL）路径：既有 FsBackend 批量读取 + 共享解析函数。
async fn collect_remote(scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
    let backend = scope.fs_backend();
    let sessions_dir = DroidProvider::sessions_dir(scope);
    if !backend.exists(&sessions_dir).await {
        return Ok(vec![]);
    }

    let re = active_re();
    let mut calls = Vec::new();
    let paths = backend.walk_jsonl(&sessions_dir).await?;
    let content_by_path = backend.read_many_to_strings(&paths).await?;

    for path in paths {
        if let Some(content) = content_by_path.get(&path) {
            calls.extend(parse_session_calls(content, re));
        }
    }

    Ok(calls)
}

/// 单个会话文件的解析。预过滤 needle：`session_start`（行 type 标签）与
/// `is now active`（触发正则 `Skill "X" is now active` 的必要子串；
/// JSON 字符串转义会改动 `"` 为 `\"`，但 `is now active` 片段不受影响）。
fn parse_session_calls(content: &str, re: &Regex) -> Vec<SkillCall> {
    let mut session_id = String::new();
    let mut project = String::new();
    let mut calls = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // R1 子串预过滤：只有 session_start 行与可能含触发串的行才值得
        // JSON DOM 解析。
        if !line.contains("session_start") && !line.contains("is now active") {
            continue;
        }
        let entry: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if entry["type"].as_str() == Some("session_start") {
            session_id = entry["id"].as_str().unwrap_or("").to_string();
            if let Some(cwd) = entry["cwd"].as_str() {
                project = cwd.to_string();
            }
            continue;
        }
        if entry["type"].as_str() != Some("message") {
            continue;
        }

        let parts = match entry["message"]["content"].as_array() {
            Some(a) => a,
            None => continue,
        };

        for part in parts {
            if part["type"].as_str() != Some("tool_result") {
                continue;
            }
            let text = part["content"].as_str().unwrap_or("");
            // 一次 tool_result 可能激活多个 skill，必须 captures_iter 取完
            for caps in re.captures_iter(text) {
                let skill = caps[1].to_string();

                let ts = match &entry["timestamp"] {
                    Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                        .map(|d| d.timestamp_millis())
                        .unwrap_or(0),
                    Value::Number(n) => n.as_i64().unwrap_or(0),
                    _ => 0,
                };

                calls.push(SkillCall {
                    skill,
                    timestamp_ms: ts,
                    project: project.clone(),
                    session_id: session_id.clone(),
                    source: SOURCE.into(),
                });
            }
        }
    }
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SkillCallFileCacheRow;
    use crate::services::usage::ENV_LOCK;
    use std::fs;
    use tempfile::TempDir;

    fn block<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    /// 用 HOME env var 重定向 home_dir 让 droid 看到我们的 fixture。
    fn with_fake_home<F: FnOnce(&TempDir)>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        let prev_home = std::env::var_os("HOME");
        let prev_userprofile = std::env::var_os("USERPROFILE");
        std::env::set_var("HOME", dir.path());
        std::env::set_var("USERPROFILE", dir.path());
        f(&dir);
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_userprofile {
            Some(v) => std::env::set_var("USERPROFILE", v),
            None => std::env::remove_var("USERPROFILE"),
        }
    }

    const SESSION_LINES: &[&str] = &[
        r#"{"type":"session_start","id":"sess-D","cwd":"/repo","timestamp":"2024-02-01T08:00:00.000Z"}"#,
        // skill 触发文本
        r#"{"type":"message","timestamp":"2024-02-01T08:05:00.000Z","message":{"content":[{"type":"tool_result","content":"Skill \"git-commit\" is now active. Doing things..."}]}}"#,
        // 不含触发串 — 过滤
        r#"{"type":"message","timestamp":"2024-02-01T08:06:00.000Z","message":{"content":[{"type":"tool_result","content":"Some other output"}]}}"#,
        // 多个匹配
        r#"{"type":"message","timestamp":"2024-02-01T08:07:00.000Z","message":{"content":[{"type":"tool_result","content":"Skill \"review\" is now active and Skill \"facts\" is now active too"}]}}"#,
        // 其它 type — 解析后丢弃
        r#"{"type":"summary","timestamp":"2024-02-01T08:08:00.000Z"}"#,
        // 损坏行
        r#"{not json"#,
    ];

    #[test]
    fn collect_extracts_skill_active_text_from_tool_results() {
        with_fake_home(|dir| {
            let sessions = dir.path().join(".factory").join("sessions");
            fs::create_dir_all(&sessions).unwrap();
            fs::write(sessions.join("s.jsonl"), SESSION_LINES.join("\n")).unwrap();

            let calls = block(DroidProvider.collect(&Scope::Local)).unwrap();
            assert_eq!(calls.len(), 3);
            let skills: Vec<&str> = calls.iter().map(|c| c.skill.as_str()).collect();
            assert!(skills.contains(&"git-commit"));
            assert!(skills.contains(&"review"));
            assert!(skills.contains(&"facts"));
            for c in &calls {
                assert_eq!(c.session_id, "sess-D");
                assert_eq!(c.project, "/repo");
                assert_eq!(c.source, "Droid CLI");
            }
        });
    }

    /// 无过滤参考实现（改动前的逐行 DOM 解析），锁定预过滤等价性。
    fn parse_unfiltered_reference(content: &str) -> Vec<SkillCall> {
        let re = active_re();
        let mut session_id = String::new();
        let mut project = String::new();
        let mut calls = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if entry["type"].as_str() == Some("session_start") {
                session_id = entry["id"].as_str().unwrap_or("").to_string();
                if let Some(cwd) = entry["cwd"].as_str() {
                    project = cwd.to_string();
                }
                continue;
            }
            if entry["type"].as_str() != Some("message") {
                continue;
            }
            let parts = match entry["message"]["content"].as_array() {
                Some(a) => a,
                None => continue,
            };
            for part in parts {
                if part["type"].as_str() != Some("tool_result") {
                    continue;
                }
                let text = part["content"].as_str().unwrap_or("");
                for caps in re.captures_iter(text) {
                    let ts = match &entry["timestamp"] {
                        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                            .map(|d| d.timestamp_millis())
                            .unwrap_or(0),
                        Value::Number(n) => n.as_i64().unwrap_or(0),
                        _ => 0,
                    };
                    calls.push(SkillCall {
                        skill: caps[1].to_string(),
                        timestamp_ms: ts,
                        project: project.clone(),
                        session_id: session_id.clone(),
                        source: SOURCE.into(),
                    });
                }
            }
        }
        calls
    }

    #[test]
    fn pre_filter_never_drops_parsing_relevant_lines() {
        let content = SESSION_LINES.join("\n");
        let filtered = parse_session_calls(&content, active_re());
        let reference = parse_unfiltered_reference(&content);
        assert_eq!(filtered, reference, "pre-filter changed parse output");
        assert_eq!(filtered.len(), 3);
        // session_start 必须被解析到（提供 session_id/project）
        assert!(filtered.iter().all(|c| c.session_id == "sess-D"));
    }

    fn reload_cache(cache: &ProviderFileCache) -> ProviderFileCache {
        let rows = cache
            .snapshot_upserts()
            .iter()
            .map(|item| SkillCallFileCacheRow {
                file_path: item.file_path.clone(),
                mtime_ms: item.mtime_ms,
                size: item.size,
                calls_json: serde_json::to_string(&item.calls).unwrap(),
                scanned_at_ms: 0,
            })
            .collect();
        ProviderFileCache::from_rows(rows)
    }

    #[test]
    fn incremental_scan_matches_full_scan_and_tracks_changes() {
        with_fake_home(|dir| {
            let sessions = dir.path().join(".factory").join("sessions");
            fs::create_dir_all(&sessions).unwrap();
            let file = sessions.join("s.jsonl");
            fs::write(&file, SESSION_LINES.join("\n")).unwrap();
            let full = block(DroidProvider.collect(&Scope::Local)).unwrap();

            let (cache, first) = block(
                DroidProvider.collect_incremental(&Scope::Local, ProviderFileCache::default()),
            )
            .unwrap();
            assert_eq!(first, full);
            assert_eq!(cache.upserts().len(), 1);

            let (cache, second) =
                block(DroidProvider.collect_incremental(&Scope::Local, reload_cache(&cache)))
                    .unwrap();
            assert_eq!(second, full);
            assert!(cache.upserts().is_empty());

            fs::remove_file(&file).unwrap();
            let (cache, third) =
                block(DroidProvider.collect_incremental(&Scope::Local, reload_cache(&cache)))
                    .unwrap();
            assert!(third.is_empty());
            assert_eq!(cache.vanished_paths().len(), 1);
        });
    }
}
