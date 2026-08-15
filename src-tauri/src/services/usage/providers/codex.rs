//! Codex CLI provider —— `~/.codex/sessions/**/*.jsonl`
//!
//! 每个 jsonl 文件是一次会话。第一行是 `type=="session_meta"`，其
//! `payload.cwd` 与 `payload.id` 给到 `project` 与 `session_id`。后续
//! `type=="response_item"` 行的 `payload.content[]` 中含 `input_text`，
//! 文本里 `<skill><name>X</name>` 是一次 skill 调用。
//!
//! ⚠ Forked subagent 会话里会出现第二个 session_meta（指向父会话），
//! 必须只用首个 session_meta 否则所有 fork 都会被记到父会话名下。
//!
//! 严格对齐 ref/skilled/index/src/providers/codex.rs。
//!
//! 性能形态（08-15-usage-page-loading-perf）：
//! - Local scope：walk + stat + 读盘 + 解析全部在单个 blocking 闭包内
//!   逐文件完成（流式，峰值内存 ≈ 最大单文件，不再物化全部内容）；
//! - 行级子串预过滤：只解析含 `<skill>` 或 `session_meta` 的行
//!   （needle 经本机全量 2554 文件 / 126 万行验证零漏报，见任务 notes）；
//! - 指纹 (mtime_ms, size) 未变的文件直接取 `ProviderFileCache` 里的
//!   缓存 calls，零磁盘 IO。
//! - Remote scope 保持既有 FsBackend 批读路径，不进增量缓存。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::services::usage::file_cache::{fingerprint_from_metadata, ProviderFileCache};
use crate::services::usage::providers::claude_code; // 复用 parse_timestamp 同形态
use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

const SOURCE: &str = "Codex CLI";

const BUILTINS: &[&str] = &[
    "exit",
    "help",
    "model",
    "clear",
    "compact",
    "undo",
    "diff",
    "history",
    "settings",
    "version",
    "approve",
    "status",
    "imagegen",
    "openai-docs",
    "plugin-creator",
    "skill-creator",
    "skill-installer",
];

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<name>([^<]+)</name>").unwrap())
}

pub struct CodexProvider;

impl CodexProvider {
    fn sessions_dir(scope: &Scope) -> String {
        if !scope.is_remote() {
            if let Ok(custom) = std::env::var("CODEX_HOME") {
                if !custom.is_empty() {
                    return scope.join_path(&custom, &["sessions"]);
                }
            }
        }
        scope.join_home(&[".codex", "sessions"])
    }

    async fn collect_with_cache(
        &self,
        scope: &Scope,
        cache: ProviderFileCache,
    ) -> Result<(ProviderFileCache, Vec<SkillCall>), UsageError> {
        if scope.is_remote() {
            // Remote 走既有 FsBackend 批读路径，不写增量缓存（远端没有廉价
            // 的 mtime/stat 通道，全量语义保持不变）。
            let calls = collect_remote(scope).await?;
            return Ok((cache, calls));
        }

        let sessions_dir = Self::sessions_dir(scope);
        crate::fs_util::run_blocking_fs_with(
            "codex session scan",
            move || Ok(scan_local(&sessions_dir, cache)),
            UsageError::task_join,
        )
        .await
    }
}

#[async_trait]
impl UsageProvider for CodexProvider {
    fn id(&self) -> &'static str {
        "codex"
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

/// Local 扫描主体（blocking 闭包内）：逐文件 stat 指纹 → 缓存命中直接取 →
/// 未命中读盘 + 预过滤 + 解析 → 登记缓存。任何时刻只持有一个文件的内容。
fn scan_local(
    sessions_dir: &str,
    mut cache: ProviderFileCache,
) -> (ProviderFileCache, Vec<SkillCall>) {
    let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
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
        let Some(path_string) = path.to_str().map(str::to_owned) else {
            continue;
        };
        let (mtime_ms, size) = fingerprint_from_metadata(entry.metadata());
        if let Some(cached_calls) = cache.lookup(&path_string, mtime_ms, size) {
            calls.extend(cached_calls);
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut file_calls = Vec::new();
        parse_session_content(&content, &builtins, &mut file_calls);
        cache.record(path_string, mtime_ms, size, file_calls.clone());
        calls.extend(file_calls);
    }

    (cache, calls)
}

/// Remote（SSH/WSL）路径：既有 FsBackend 批量读取 + 共享解析函数。
async fn collect_remote(scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
    let backend = scope.fs_backend();
    let sessions_dir = CodexProvider::sessions_dir(scope);
    if !backend.exists(&sessions_dir).await {
        return Ok(vec![]);
    }

    let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
    let mut calls: Vec<SkillCall> = Vec::new();
    let paths = backend.walk_jsonl(&sessions_dir).await?;
    let content_by_path = backend.read_many_to_strings(&paths).await?;

    for path in paths {
        if let Some(content) = content_by_path.get(&path) {
            parse_session_content(content, &builtins, &mut calls);
        }
    }

    Ok(calls)
}

fn parse_session_content(content: &str, builtins: &HashSet<&str>, calls: &mut Vec<SkillCall>) {
    let mut project = String::new();
    let mut session_id = String::new();
    let re = name_re();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // R1 子串预过滤：只有 session_meta 行和可能含 <skill> 标签的行才值得
        // JSON DOM 解析。会产生解析效果的行必然命中其一：
        // - type=="session_meta" 行必然含字面量 `session_meta`（type 标签本身）；
        // - 产出调用的行要求文本含 `<skill>`（regex 需 `<name>` 位于其中），
        //   JSON 字符串转义不改动 `<`（本机全量数据验证 0 例外，且 codex 侧
        //   serde_json 默认不转义 `<` 为 <>）。
        if !line.contains("session_meta") && !line.contains("<skill>") {
            continue;
        }
        let entry: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match entry["type"].as_str() {
            Some("session_meta") if session_id.is_empty() => {
                project = entry["payload"]["cwd"].as_str().unwrap_or("").to_string();
                session_id = entry["payload"]["id"].as_str().unwrap_or("").to_string();
            }
            Some("response_item") => {
                let content_arr = match entry["payload"]["content"].as_array() {
                    Some(a) => a,
                    None => continue,
                };

                for part in content_arr {
                    if part["type"].as_str() != Some("input_text") {
                        continue;
                    }
                    let text = part["text"].as_str().unwrap_or("");
                    if !text.contains("<skill>") {
                        continue;
                    }

                    for caps in re.captures_iter(text) {
                        let skill = &caps[1];
                        if builtins.contains(skill) {
                            continue;
                        }

                        let ts = claude_code_parse_ts(&entry["timestamp"]);

                        calls.push(SkillCall {
                            skill: skill.to_string(),
                            timestamp_ms: ts,
                            project: project.clone(),
                            session_id: session_id.clone(),
                            source: SOURCE.into(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// 桥接到 claude_code 的 parse_timestamp（私有），通过本地包装重导出。
fn claude_code_parse_ts(value: &Value) -> i64 {
    // claude_code 模块的 parse_timestamp 不是 pub，我们这里复刻同等逻辑。
    if let Some(n) = value.as_i64() {
        return n;
    }
    if let Some(s) = value.as_str() {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return dt.timestamp_millis();
        }
        if let Ok(n) = s.parse::<i64>() {
            return n;
        }
    }
    if let Some(f) = value.as_f64() {
        return f as i64;
    }
    0
}

// 此 import 让 codex 显式依赖 claude_code 模块编译序：让 cargo 知道这俩
// 模块同属 providers，且 claude_code 优先编译以保持 BUILTINS 等静态项
// 已初始化（虽然实际共享内容很少，写在这里防止后续把 codex 列在前面时
// 漏掉 claude_code 自身依赖）。
#[allow(unused_imports)]
use claude_code as _claude_code_dep;

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

    fn make_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        let sessions = dir.path().join("sessions");
        let inner = sessions.join("2024").join("01");
        fs::create_dir_all(&inner).unwrap();

        let session = [
            r#"{"type":"session_meta","timestamp":"2024-01-15T10:00:00.000Z","payload":{"cwd":"/work/app","id":"sess-A"}}"#,
            // 二次 session_meta（fork）—— 应当被忽略
            r#"{"type":"session_meta","timestamp":"2024-01-15T10:05:00.000Z","payload":{"cwd":"/parent","id":"parent-id"}}"#,
            // 合法 skill 调用
            r#"{"type":"response_item","timestamp":"2024-01-15T10:10:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>review</name></skill>"}]}}"#,
            // BUILTINS — 过滤
            r#"{"type":"response_item","timestamp":"2024-01-15T10:11:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>clear</name></skill>"}]}}"#,
            // 没有 <skill> 标签 — 不动
            r#"{"type":"response_item","timestamp":"2024-01-15T10:12:00.000Z","payload":{"content":[{"type":"input_text","text":"normal text"}]}}"#,
            // 多个 skill 在一行
            r#"{"type":"response_item","timestamp":"2024-01-15T10:13:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>a</name></skill> <skill><name>b</name></skill>"}]}}"#,
            // 损坏
            r#"{not json"#,
        ]
        .join("\n");
        fs::write(inner.join("rollout.jsonl"), session).unwrap();

        dir
    }

    #[test]
    fn collect_parses_session_meta_response_items_and_filters_builtins() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = make_fixture();
        std::env::set_var("CODEX_HOME", dir.path());
        let calls = block(CodexProvider.collect(&Scope::Local)).unwrap();
        std::env::remove_var("CODEX_HOME");

        // 期望：review + a + b = 3；clear / "normal text" / 损坏行被过滤
        assert_eq!(calls.len(), 3, "got: {calls:#?}");
        let skills: Vec<&str> = calls.iter().map(|c| c.skill.as_str()).collect();
        assert!(skills.contains(&"review"));
        assert!(skills.contains(&"a"));
        assert!(skills.contains(&"b"));

        // session_id / project 全部来自首个 session_meta（"sess-A" / "/work/app"）
        for c in &calls {
            assert_eq!(
                c.session_id, "sess-A",
                "fork session_meta should not override"
            );
            assert_eq!(c.project, "/work/app");
            assert_eq!(c.source, "Codex CLI");
            assert!(c.timestamp_ms > 0);
        }
    }

    #[test]
    fn available_false_when_dir_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", dir.path());
        // 没创建 sessions 目录
        let avail = block(CodexProvider.available(&Scope::Local));
        std::env::remove_var("CODEX_HOME");
        assert!(!avail);
    }

    /// 无过滤参考实现（改动前的逐行 DOM 解析），用于锁定预过滤等价性。
    fn parse_unfiltered_reference(content: &str) -> Vec<SkillCall> {
        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let mut calls = Vec::new();
        let mut project = String::new();
        let mut session_id = String::new();
        let re = name_re();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match entry["type"].as_str() {
                Some("session_meta") if session_id.is_empty() => {
                    project = entry["payload"]["cwd"].as_str().unwrap_or("").to_string();
                    session_id = entry["payload"]["id"].as_str().unwrap_or("").to_string();
                }
                Some("response_item") => {
                    let content_arr = match entry["payload"]["content"].as_array() {
                        Some(a) => a,
                        None => continue,
                    };
                    for part in content_arr {
                        if part["type"].as_str() != Some("input_text") {
                            continue;
                        }
                        let text = part["text"].as_str().unwrap_or("");
                        if !text.contains("<skill>") {
                            continue;
                        }
                        for caps in re.captures_iter(text) {
                            let skill = &caps[1];
                            if builtins.contains(skill) {
                                continue;
                            }
                            calls.push(SkillCall {
                                skill: skill.to_string(),
                                timestamp_ms: claude_code_parse_ts(&entry["timestamp"]),
                                project: project.clone(),
                                session_id: session_id.clone(),
                                source: SOURCE.into(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        calls
    }

    #[test]
    fn pre_filter_never_drops_parsing_relevant_lines() {
        // 覆盖：session_meta（含 fork 次序）、含/不含 <skill> 的 response_item、
        // 其它 type、损坏行、以及"看似相关但不含 needle"的噪声行。
        let content = [
            r#"{"type":"session_meta","timestamp":"2024-01-15T10:00:00.000Z","payload":{"cwd":"/work/app","id":"sess-A"}}"#,
            r#"{"type":"response_item","timestamp":"2024-01-15T10:10:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>review</name></skill>"}]}}"#,
            // type 拼写相近但不等：不可能成为 session_meta
            r#"{"type":"session_meta2","payload":{"cwd":"/x","id":"y"}}"#,
            // output_text 而非 input_text
            r#"{"type":"response_item","timestamp":"2024-01-15T10:11:00.000Z","payload":{"content":[{"type":"output_text","text":"<skill><name>ghost</name></skill>"}]}}"#,
            // 长噪声行（无 needle）
            r#"{"type":"response_item","timestamp":"2024-01-15T10:12:00.000Z","payload":{"content":[{"type":"input_text","text":"just chatting about skills in general"}]}}"#,
            // turn_context 等其它行型
            r#"{"type":"turn_context","timestamp":"2024-01-15T10:12:30.000Z","payload":{"cwd":"/work/app"}}"#,
            // 损坏行
            r#"{not json"#,
            // 多 skill 同行 + builtins 混合
            r#"{"type":"response_item","timestamp":"2024-01-15T10:13:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>a</name></skill> <skill><name>clear</name></skill> <skill><name>b</name></skill>"}]}}"#,
        ]
        .join("\n");

        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let mut filtered = Vec::new();
        parse_session_content(&content, &builtins, &mut filtered);
        let reference = parse_unfiltered_reference(&content);
        assert_eq!(filtered, reference, "pre-filter changed parse output");
        assert_eq!(filtered.len(), 3, "review + a + b");
    }

    /// 把一次 collect_incremental 返回的 upserts 序列化成 DB 行再载入，
    /// 模拟编排器「落库 → 下次扫描载入」的完整回路。
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
    fn incremental_scan_skips_unchanged_reparses_changed_and_drops_vanished() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = make_fixture();
        std::env::set_var("CODEX_HOME", dir.path());

        // 第一次：空缓存 = 全量扫描，全部文件 record
        let (cache, first) =
            block(CodexProvider.collect_incremental(&Scope::Local, ProviderFileCache::default()))
                .unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(cache.upserts().len(), 1);
        assert!(cache.vanished_paths().is_empty());

        // 第二次：载入缓存后指纹未变 → 结果一致且零重新解析（upserts 为空）
        let (cache, second) =
            block(CodexProvider.collect_incremental(&Scope::Local, reload_cache(&cache))).unwrap();
        assert_eq!(second, first, "incremental result must equal full scan");
        assert!(
            cache.upserts().is_empty(),
            "unchanged files must not re-parse"
        );
        assert!(cache.vanished_paths().is_empty());

        // 改动文件（追加一行新 skill + 改变 size）→ 只重解析该文件
        let rollout = dir
            .path()
            .join("sessions")
            .join("2024")
            .join("01")
            .join("rollout.jsonl");
        let appended = fs::read_to_string(&rollout).unwrap()
            + "\n"
            + r#"{"type":"response_item","timestamp":"2024-01-15T10:20:00.000Z","payload":{"content":[{"type":"input_text","text":"<skill><name>facts</name></skill>"}]}}"#;
        fs::write(&rollout, appended).unwrap();
        let (cache, third) =
            block(CodexProvider.collect_incremental(&Scope::Local, reload_cache(&cache))).unwrap();
        assert_eq!(third.len(), 4);
        assert!(third.iter().any(|c| c.skill == "facts"));
        assert_eq!(cache.upserts().len(), 1, "changed file re-parsed");

        // 删除文件 → 缓存行标 vanished，calls 归零
        fs::remove_file(&rollout).unwrap();
        let (cache, fourth) =
            block(CodexProvider.collect_incremental(&Scope::Local, reload_cache(&cache))).unwrap();
        assert!(fourth.is_empty());
        assert_eq!(cache.vanished_paths().len(), 1);

        std::env::remove_var("CODEX_HOME");
    }
}
