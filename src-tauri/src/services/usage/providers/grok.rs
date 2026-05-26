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

use std::collections::HashSet;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::services::usage::{Scope, SkillCall, UsageProvider};

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

    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, String> {
        let backend = scope.fs_backend();
        let sessions_dir = Self::sessions_dir(scope);
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
                collect_from_updates_content(
                    content,
                    &request.project,
                    &request.session_dir_name,
                    &builtins,
                    re,
                    &mut seen,
                    &mut calls,
                );
            }
            if let Some(content) = content_by_path.get(&request.chat_path) {
                collect_from_chat_content(
                    content,
                    &request.project,
                    &request.session_dir_name,
                    &builtins,
                    re,
                    &mut seen,
                    &mut calls,
                );
            }
        }

        Ok(calls)
    }
}

fn collect_from_updates_content(
    content: &str,
    project: &str,
    session_dir_name: &str,
    builtins: &HashSet<&str>,
    re: &Regex,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) {
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
}

fn collect_from_chat_content(
    content: &str,
    project: &str,
    session_dir_name: &str,
    builtins: &HashSet<&str>,
    re: &Regex,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) {
    let session_ts = uuidv7_to_ms(session_dir_name);

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
            if builtins.contains(skill.as_str()) || seen.contains(&skill) {
                continue;
            }
            seen.insert(skill.clone());

            calls.push(SkillCall {
                skill,
                timestamp_ms: session_ts,
                project: project.to_string(),
                session_id: session_dir_name.to_string(),
                source: SOURCE.into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn collect_dual_source_dedupes_and_skips_background_context() {
        with_fake_home(|dir| {
            // sess uuidv7 with known prefix 0190 0000 0000 ⇒ first 48 bits = 0x019000000000
            let sess_uuid = "01900000-0000-7000-8000-000000000000";
            let proj_encoded = "%2Fhome%2Fme%2Fapp";
            let sess_path = dir
                .path()
                .join(".grok")
                .join("sessions")
                .join(proj_encoded)
                .join(sess_uuid);
            std::fs::create_dir_all(&sess_path).unwrap();

            let updates = vec![
                // 合法 skill — review
                r#"{"timestamp":1700000000.5,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>review</command-name>"}}}}"#,
                // BUILTINS — clear, 过滤
                r#"{"timestamp":1700000001.0,"params":{"sessionId":"sid-real","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"<command-name>clear</command-name>"}}}}"#,
            ]
            .join("\n");
            std::fs::write(sess_path.join("updates.jsonl"), updates).unwrap();

            let chat = vec![
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
}
