//! Droid (Factory) provider —— `~/.factory/sessions/**/*.jsonl`
//!
//! 与 Codex 类似，但 skill 触发是从 `tool_result` 内的文本里
//! 正则匹配 `Skill "X" is now active` 串提取出来。`session_start`
//! 行给 `session_id` 与 `project`。

use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

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
        let backend = scope.fs_backend();
        let sessions_dir = Self::sessions_dir(scope);
        if !backend.exists(&sessions_dir).await {
            return Ok(vec![]);
        }

        let mut calls = Vec::new();
        let re = active_re();
        let paths = backend.walk_jsonl(&sessions_dir).await?;
        let content_by_path = backend.read_many_to_strings(&paths).await?;

        for path in paths {
            if let Some(content) = content_by_path.get(&path) {
                collect_from_session_content(content, re, &mut calls);
            }
        }

        Ok(calls)
    }
}

fn collect_from_session_content(content: &str, re: &Regex, calls: &mut Vec<SkillCall>) {
    let mut session_id = String::new();
    let mut project = String::new();

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
}

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

    #[test]
    fn collect_extracts_skill_active_text_from_tool_results() {
        with_fake_home(|dir| {
            let sessions = dir.path().join(".factory").join("sessions");
            fs::create_dir_all(&sessions).unwrap();

            let session = [
                r#"{"type":"session_start","id":"sess-D","cwd":"/repo","timestamp":"2024-02-01T08:00:00.000Z"}"#,
                // skill 触发文本
                r#"{"type":"message","timestamp":"2024-02-01T08:05:00.000Z","message":{"content":[{"type":"tool_result","content":"Skill \"git-commit\" is now active. Doing things..."}]}}"#,
                // 不含触发串 — 过滤
                r#"{"type":"message","timestamp":"2024-02-01T08:06:00.000Z","message":{"content":[{"type":"tool_result","content":"Some other output"}]}}"#,
                // 多个匹配
                r#"{"type":"message","timestamp":"2024-02-01T08:07:00.000Z","message":{"content":[{"type":"tool_result","content":"Skill \"review\" is now active and Skill \"facts\" is now active too"}]}}"#,
            ]
            .join("\n");
            fs::write(sessions.join("s.jsonl"), session).unwrap();

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
}
