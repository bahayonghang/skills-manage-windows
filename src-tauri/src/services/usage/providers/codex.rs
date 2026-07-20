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

use std::collections::HashSet;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

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
        let backend = scope.fs_backend();
        let sessions_dir = Self::sessions_dir(scope);
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
}

fn parse_session_content(content: &str, builtins: &HashSet<&str>, calls: &mut Vec<SkillCall>) {
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
}
