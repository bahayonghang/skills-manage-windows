//! Grok CLI provider —— `~/.grok/sessions/<urlencoded-cwd>/<uuidv7>/`
//!
//! 双源解析：
//! - `updates.jsonl`：每行带 `timestamp`（秒）；`params.update.sessionUpdate
//!   == "user_message_chunk"` 文本里 `<command-name>X</command-name>` 是 skill。
//! - `chat_history.jsonl`：作为补漏（subagent 派生 / 流不完整）。没有
//!   per-message 时间戳，回退到从 session UUIDv7 反推毫秒。
//!
//! `<background_context>` 标签的消息是子代理 replay 的父会话历史，必须
//! 跳过否则会重复计数。
//!
//! 跨两源用 `seen: HashSet<skill>` 去重，按 session 维度独立。
//!
//! 性能形态（08-15-usage-page-loading-perf）：
//! - Local scope：目录遍历 + 读盘 + 解析在单个 blocking 闭包内逐文件
//!   流式完成；updates.jsonl 增加 `<command-name>` 行级预过滤
//!   （chat_history.jsonl 已有 `command-name` 预过滤）；
//! - per-file 解析产出「原始 calls」，合并阶段按 session 重放同一个
//!   `seen(skill)` 去重（updates 在前、chat 在后）——与旧共享 seen 的
//!   解析语义完全等价；
//! - 指纹未变的文件直接取 `ProviderFileCache` 缓存，零磁盘 IO。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::services::usage::file_cache::{fingerprint_from_metadata, ProviderFileCache};
use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

const SOURCE: &str = "Grok CLI";

const BUILTINS: &[&str] = &[
    "compact",
    "always-approve",
    "context",
    "plugins",
    "reload-plugins",
    "session-info",
    "imagine",
    "imagine-video",
    "feedback",
    "loop",
    "help",
    "memory",
    "clear",
    "exit",
];

fn cmd_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<command-name>([^<]+)</command-name>").unwrap())
}

/// UUIDv7 前 48 位是 Unix 毫秒时间戳。
fn uuidv7_to_ms(uuid: &str) -> i64 {
    let hex: String = uuid.chars().filter(|c| *c != '-').take(12).collect();
    i64::from_str_radix(&hex, 16).unwrap_or(0)
}

pub struct GrokProvider;

struct SessionReadRequest {
    project: String,
    session_dir_name: String,
    updates_path: String,
    chat_path: String,
}

impl GrokProvider {
    fn sessions_dir(scope: &Scope) -> String {
        scope.join_home(&[".grok", "sessions"])
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
            "grok session scan",
            move || Ok(scan_local(&sessions_dir, cache)),
            UsageError::task_join,
        )
        .await
    }
}

#[async_trait]
impl UsageProvider for GrokProvider {
    fn id(&self) -> &'static str {
        "grok"
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

/// Local 扫描主体（blocking 闭包内）：一层项目目录 → 一层会话目录 →
/// updates.jsonl / chat_history.jsonl 逐文件「指纹命中取缓存，未命中读盘
/// 解析」。任何时刻只持有一个文件的内容。
fn scan_local(
    sessions_dir: &str,
    mut cache: ProviderFileCache,
) -> (ProviderFileCache, Vec<SkillCall>) {
    let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
    let re = cmd_re();
    let mut calls = Vec::new();

    let root = PathBuf::from(sessions_dir);
    let Ok(projects) = std::fs::read_dir(&root) else {
        return (cache, calls);
    };

    for proj in projects.flatten() {
        if !proj.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = proj.file_name().to_string_lossy().into_owned();
        if !dir_name.starts_with("%2F") {
            continue;
        }
        let project = urlencoding::decode(&dir_name)
            .map(|s| s.into_owned())
            .unwrap_or(dir_name.clone());

        let Ok(session_dirs) = std::fs::read_dir(proj.path()) else {
            continue;
        };
        for sess in session_dirs.flatten() {
            if !sess.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let session_dir_name = sess.file_name().to_string_lossy().into_owned();
            let sess_path = sess.path();
            let mut seen: HashSet<String> = HashSet::new();

            // updates.jsonl 在前、chat_history.jsonl 在后，与旧合并顺序一致。
            for (path, is_updates) in [
                (sess_path.join("updates.jsonl"), true),
                (sess_path.join("chat_history.jsonl"), false),
            ] {
                let path_string = path.to_string_lossy().into_owned();
                // 元数据失败（含文件已删除）必须跳过且不触碰缓存——
                // lookup 会把 path 计入 seen，让已删除文件逃出 vanished 检测。
                let Ok(meta) = std::fs::metadata(&path) else {
                    continue;
                };
                let (mtime_ms, size) = fingerprint_from_metadata::<std::io::Error>(Ok(meta));
                let raw = match cache.lookup(&path_string, mtime_ms, size) {
                    Some(cached) => cached,
                    None => {
                        let Ok(content) = std::fs::read_to_string(&path) else {
                            continue;
                        };
                        let parsed = if is_updates {
                            parse_updates_calls(
                                &content,
                                &project,
                                &session_dir_name,
                                &builtins,
                                re,
                            )
                        } else {
                            parse_chat_calls(&content, &project, &session_dir_name, &builtins, re)
                        };
                        cache.record(path_string, mtime_ms, size, parsed.clone());
                        parsed
                    }
                };
                merge_session_calls(raw, &mut seen, &mut calls);
            }
        }
    }

    (cache, calls)
}

/// Remote（SSH/WSL）路径：既有 FsBackend 批量读取 + 与 Local 相同的
/// 原始解析 + 按 session 合并去重。
async fn collect_remote(scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
    let backend = scope.fs_backend();
    let sessions_dir = GrokProvider::sessions_dir(scope);
    if !backend.exists(&sessions_dir).await {
        return Ok(vec![]);
    }

    let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
    let re = cmd_re();
    let mut calls = Vec::new();
    let mut session_requests = Vec::new();

    for proj in backend.list_entries(&sessions_dir).await? {
        if !proj.is_dir {
            continue;
        }
        let dir_name = proj.name;
        if !dir_name.starts_with("%2F") {
            continue;
        }
        let project = urlencoding::decode(&dir_name)
            .map(|s| s.into_owned())
            .unwrap_or(dir_name.clone());
        let proj_path = scope.join_path(&sessions_dir, &[&dir_name]);

        let session_dirs = match backend.list_entries(&proj_path).await {
            Ok(d) => d,
            Err(_) => continue,
        };

        for sess in session_dirs {
            if !sess.is_dir {
                continue;
            }
            let session_dir_name = sess.name;
            let sess_path = scope.join_path(&proj_path, &[&session_dir_name]);
            let updates = scope.join_path(&sess_path, &["updates.jsonl"]);
            let chat = scope.join_path(&sess_path, &["chat_history.jsonl"]);
            session_requests.push(SessionReadRequest {
                project: project.clone(),
                session_dir_name,
                updates_path: updates,
                chat_path: chat,
            });
        }
    }

    let mut file_paths = Vec::with_capacity(session_requests.len() * 2);
    for request in &session_requests {
        file_paths.push(request.updates_path.clone());
        file_paths.push(request.chat_path.clone());
    }
    let content_by_path = backend.read_many_to_strings(&file_paths).await?;

    for request in session_requests {
        let mut seen: HashSet<String> = HashSet::new();
        if let Some(content) = content_by_path.get(&request.updates_path) {
            merge_session_calls(
                parse_updates_calls(
                    content,
                    &request.project,
                    &request.session_dir_name,
                    &builtins,
                    re,
                ),
                &mut seen,
                &mut calls,
            );
        }
        if let Some(content) = content_by_path.get(&request.chat_path) {
            merge_session_calls(
                parse_chat_calls(
                    content,
                    &request.project,
                    &request.session_dir_name,
                    &builtins,
                    re,
                ),
                &mut seen,
                &mut calls,
            );
        }
    }

    Ok(calls)
}

/// 按 session 去重合并：key = skill 名，与旧实现在解析期共享 seen 的
/// 行为完全等价（updates 先、chat 后，首个出现者胜出）。
fn merge_session_calls(
    raw: Vec<SkillCall>,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) {
    for call in raw {
        if seen.insert(call.skill.clone()) {
            calls.push(call);
        }
    }
}

/// updates.jsonl 的原始 per-file 解析（不做 session 级去重）。
fn parse_updates_calls(
    content: &str,
    project: &str,
    session_dir_name: &str,
    builtins: &HashSet<&str>,
    re: &Regex,
) -> Vec<SkillCall> {
    let mut calls = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // R1 子串预过滤：产出调用的行必然让文本命中 `<command-name>` 正则，
        // 即原始行必含该字面量（JSON 字符串转义不改动 `<`）。
        if !line.contains("<command-name>") {
            continue;
        }
        let record: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let update = &record["params"]["update"];
        if update["sessionUpdate"].as_str() != Some("user_message_chunk") {
            continue;
        }
        let mc = &update["content"];
        if mc["type"].as_str() != Some("text") {
            continue;
        }
        let text = mc["text"].as_str().unwrap_or("");

        for caps in re.captures_iter(text) {
            let skill = caps[1].to_string();
            if builtins.contains(skill.as_str()) {
                continue;
            }

            // Grok timestamp 是秒（小数），换算到毫秒
            let ts = record["timestamp"]
                .as_f64()
                .map(|f| (f * 1000.0) as i64)
                .or_else(|| record["timestamp"].as_i64().map(|n| n * 1000))
                .unwrap_or(0);

            calls.push(SkillCall {
                skill,
                timestamp_ms: ts,
                project: project.to_string(),
                session_id: record["params"]["sessionId"]
                    .as_str()
                    .unwrap_or(session_dir_name)
                    .to_string(),
                source: SOURCE.into(),
            });
        }
    }
    calls
}

/// chat_history.jsonl 的原始 per-file 解析（不做 session 级去重）。
fn parse_chat_calls(
    content: &str,
    project: &str,
    session_dir_name: &str,
    builtins: &HashSet<&str>,
    re: &Regex,
) -> Vec<SkillCall> {
    let session_ts = uuidv7_to_ms(session_dir_name);
    let mut calls = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() || !line.contains("command-name") {
            continue;
        }
        let record: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if record["type"].as_str() != Some("user") {
            continue;
        }

        let text = match &record["content"] {
            Value::String(s) => s.clone(),
            Value::Array(arr) => arr
                .iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join(""),
            _ => continue,
        };

        // Subagent replay 跳过
        if text.contains("<background_context>") {
            continue;
        }

        for caps in re.captures_iter(&text) {
            let skill = caps[1].to_string();
            if builtins.contains(skill.as_str()) {
                continue;
            }

            calls.push(SkillCall {
                skill,
                timestamp_ms: session_ts,
                project: project.to_string(),
                session_id: session_dir_name.to_string(),
                source: SOURCE.into(),
            });
        }
    }
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SkillCallFileCacheRow;
    use crate::services::usage::ENV_LOCK;
    use tempfile::TempDir;

    fn block<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

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

    fn write_grok_fixture(sess_path: &std::path::Path) {
        let updates = [
            // 合法 skill — review
            r#"{"timestamp":1700000000.5,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>review</command-name>"}}}}"#,
            // BUILTINS — clear, 过滤
            r#"{"timestamp":1700000001.0,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>clear</command-name>"}}}}"#,
            // 与 skill 无关的噪声行（无 <command-name>，预过滤跳过 DOM 解析）
            r#"{"timestamp":1700000002.0,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"thinking"}}}}"#,
            // 同行重复 skill —— 合并时去重
            r#"{"timestamp":1700000003.0,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>review</command-name>"}}}}"#,
        ]
        .join("\n");
        std::fs::write(sess_path.join("updates.jsonl"), updates).unwrap();

        let chat = [
            // 已在 updates 出现过的 review — 应被去重
            r#"{"type":"user","content":"<command-name>review</command-name>"}"#,
            // 新 skill — facts，应保留，使用 session ts 回退
            r#"{"type":"user","content":"<command-name>facts</command-name>"}"#,
            // background_context replay — 必须跳过
            r#"{"type":"user","content":"<background_context><command-name>git-commit</command-name></background_context>"}"#,
            // type 不是 user — 过滤
            r#"{"type":"system","content":"<command-name>x</command-name>"}"#,
        ]
        .join("\n");
        std::fs::write(sess_path.join("chat_history.jsonl"), chat).unwrap();
    }

    fn fixture_home(dir: &TempDir) -> std::path::PathBuf {
        let sess_uuid = "01900000-0000-7000-8000-000000000000";
        let proj_encoded = "%2Fhome%2Fme%2Fapp";
        let sess_path = dir
            .path()
            .join(".grok")
            .join("sessions")
            .join(proj_encoded)
            .join(sess_uuid);
        std::fs::create_dir_all(&sess_path).unwrap();
        write_grok_fixture(&sess_path);
        sess_path
    }

    #[test]
    fn collect_dual_source_dedupes_and_skips_background_context() {
        with_fake_home(|dir| {
            // sess uuidv7 with known prefix 0190 0000 0000 ⇒ first 48 bits = 0x019000000000
            let sess_uuid = "01900000-0000-7000-8000-000000000000";
            fixture_home(dir);

            let calls = block(GrokProvider.collect(&Scope::Local)).unwrap();
            assert_eq!(calls.len(), 2, "got: {calls:#?}");

            let skills: Vec<&str> = calls.iter().map(|c| c.skill.as_str()).collect();
            assert!(skills.contains(&"review"));
            assert!(skills.contains(&"facts"));
            assert!(!skills.contains(&"clear"));
            assert!(!skills.contains(&"git-commit"));

            for c in &calls {
                assert_eq!(c.project, "/home/me/app");
                assert_eq!(c.source, "Grok CLI");
            }

            let review = calls.iter().find(|c| c.skill == "review").unwrap();
            assert_eq!(review.session_id, "sid-real");
            assert_eq!(review.timestamp_ms, 1_700_000_000_500);

            let facts = calls.iter().find(|c| c.skill == "facts").unwrap();
            assert_eq!(facts.session_id, sess_uuid);
            // UUIDv7 前 48 位 0x019000000000 = 1718800000000ms 范围
            assert!(facts.timestamp_ms > 0);
        });
    }

    #[test]
    fn uuidv7_to_ms_extracts_first_48_bits() {
        // 48-bit hex 0x019000000000 = 1718800000000 (一个 2024 年中的时间戳)
        let ms = uuidv7_to_ms("01900000-0000-7000-8000-000000000000");
        assert_eq!(ms, 0x019000000000);
    }

    /// 无过滤参考实现（改动前的逐行 DOM 解析 updates 内容），锁定
    /// `<command-name>` 预过滤的等价性。
    fn parse_updates_unfiltered_reference(
        content: &str,
        project: &str,
        session_dir_name: &str,
    ) -> Vec<SkillCall> {
        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let re = cmd_re();
        let mut seen: HashSet<String> = HashSet::new();
        let mut calls = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let record: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let update = &record["params"]["update"];
            if update["sessionUpdate"].as_str() != Some("user_message_chunk") {
                continue;
            }
            let mc = &update["content"];
            if mc["type"].as_str() != Some("text") {
                continue;
            }
            let text = mc["text"].as_str().unwrap_or("");
            for caps in re.captures_iter(text) {
                let skill = caps[1].to_string();
                if builtins.contains(skill.as_str()) || !seen.insert(skill.clone()) {
                    continue;
                }
                let ts = record["timestamp"]
                    .as_f64()
                    .map(|f| (f * 1000.0) as i64)
                    .or_else(|| record["timestamp"].as_i64().map(|n| n * 1000))
                    .unwrap_or(0);
                calls.push(SkillCall {
                    skill,
                    timestamp_ms: ts,
                    project: project.to_string(),
                    session_id: record["params"]["sessionId"]
                        .as_str()
                        .unwrap_or(session_dir_name)
                        .to_string(),
                    source: SOURCE.into(),
                });
            }
        }
        calls
    }

    #[test]
    fn updates_pre_filter_never_drops_parsing_relevant_lines() {
        let content = [
            r#"{"timestamp":1700000000.5,"params":{"sessionId":"sid","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>review</command-name>"}}}}"#,
            // sessionUpdate 不是 user_message_chunk 但含 needle（保守放行，解析后丢弃）
            r#"{"timestamp":1700000001.0,"params":{"sessionId":"sid","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"<command-name>ghost</command-name>"}}}}"#,
            // user_message_chunk 但无 skill 标签
            r#"{"timestamp":1700000002.0,"params":{"sessionId":"sid","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"plain question"}}}}"#,
            // 损坏行
            r#"{not json"#,
            // content type 非 text
            r#"{"timestamp":1700000003.0,"params":{"sessionId":"sid","update":{"sessionUpdate":"user_message_chunk","content":{"type":"image","text":"<command-name>x</command-name>"}}}}"#,
        ]
        .join("\n");

        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let filtered = parse_updates_calls(&content, "/p", "sess", &builtins, cmd_re());
        let reference = parse_updates_unfiltered_reference(&content, "/p", "sess");
        assert_eq!(
            filtered, reference,
            "pre-filter changed updates parse output"
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].skill, "review");
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
            let sess_path = fixture_home(dir);
            let full = block(GrokProvider.collect(&Scope::Local)).unwrap();

            // 空缓存第一轮 == 全量
            let (cache, first) = block(
                GrokProvider.collect_incremental(&Scope::Local, ProviderFileCache::default()),
            )
            .unwrap();
            assert_eq!(first, full);
            assert_eq!(cache.upserts().len(), 2, "updates + chat recorded");

            // 稳态：零重解析，结果不变
            let (cache, second) =
                block(GrokProvider.collect_incremental(&Scope::Local, reload_cache(&cache)))
                    .unwrap();
            assert_eq!(second, full);
            assert!(cache.upserts().is_empty());
            assert!(cache.vanished_paths().is_empty());

            // 删除 chat_history → facts 消失；updates 仍命中缓存
            std::fs::remove_file(sess_path.join("chat_history.jsonl")).unwrap();
            let (cache, third) =
                block(GrokProvider.collect_incremental(&Scope::Local, reload_cache(&cache)))
                    .unwrap();
            assert_eq!(third.len(), 1);
            assert_eq!(third[0].skill, "review");
            assert_eq!(cache.vanished_paths().len(), 1);
            assert!(cache.upserts().is_empty(), "updates still fingerprint-hit");
        });
    }
}
