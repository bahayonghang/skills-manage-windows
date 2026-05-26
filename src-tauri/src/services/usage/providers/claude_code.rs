//! Claude Code provider —— 双源解析。
//!
//! 数据源 1：`~/.claude/history.jsonl` 顶层 slash command 历史。每行是
//! `{ display, project, sessionId, timestamp }` 形态；`display` 中以
//! `/skill-name` 开头的就是一次 skill 调用。BUILTINS 黑名单过滤 `clear`、
//! `model` 等系统命令。
//!
//! 数据源 2：`~/.claude/projects/<encoded-cwd>/<session>.jsonl`，每个文件
//! 是一个会话。其中 `type=="assistant"` 且 `message.content[].type=="tool_use"
//! && name=="Skill"` 的项是 Skill 工具调用。`session_id` 取文件名 stem，
//! `project` 优先从首个含 `cwd` 的行抽取（解码目录名歧义大）。
//!
//! 两源都进同一个 `seen: HashSet<"skill:ts">` 去重。
//!
//! 实现严格对齐 skilled 项目的 `ref/skilled/index/src/providers/claude_code.rs`
//! 与 TypeScript 同名 provider。`available()` 仅判定 `history.jsonl` 是否存在，
//! 与 skilled 一致——projects/ 单独存在但 history 不存在的情况罕见。

use std::collections::HashSet;
use std::sync::OnceLock;

use async_trait::async_trait;
use chrono::DateTime;
use regex::Regex;
use serde_json::Value;

use crate::services::usage::fs_backend::FsBackend;
use crate::services::usage::{Scope, SkillCall, UsageProvider};

const SOURCE: &str = "Claude Code";

/// Claude Code 自带的 slash 命令——这些不是用户技能，要从聚合里剔除。
/// 列表与 skilled 完全一致。
const BUILTINS: &[&str] = &[
    "clear",
    "model",
    "usage",
    "resume",
    "new",
    "quit",
    "exit",
    "login",
    "logout",
    "help",
    "config",
    "compact",
    "doctor",
    "cost",
    "effort",
    "memory",
    "status",
    "skills",
    "permissions",
    "mcp",
    "terminal-setup",
    "remote-env",
    "remote-control",
    "fast",
];

/// `^/<name>` 的提取正则。`name` 须以字母开头，允许字母数字下划线短横线。
fn skill_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^/([a-zA-Z][a-zA-Z0-9_-]*)(?:\s|$)").unwrap())
}

pub struct ClaudeCodeProvider;

impl ClaudeCodeProvider {
    /// 解析 `CLAUDE_CONFIG_DIR` 或回退到 `<home>/.claude`。
    fn claude_home(scope: &Scope) -> String {
        if !scope.is_remote() {
            if let Ok(custom) = std::env::var("CLAUDE_CONFIG_DIR") {
                if !custom.is_empty() {
                    return custom;
                }
            }
        }
        scope.join_home(&[".claude"])
    }
}

#[async_trait]
impl UsageProvider for ClaudeCodeProvider {
    fn id(&self) -> &'static str {
        "claude-code"
    }
    fn display_name(&self) -> &'static str {
        SOURCE
    }

    async fn available(&self, scope: &Scope) -> bool {
        let backend = scope.fs_backend();
        let home = Self::claude_home(scope);
        backend
            .exists(&scope.join_path(&home, &["history.jsonl"]))
            .await
    }

    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, String> {
        let backend = scope.fs_backend();
        let claude_home = Self::claude_home(scope);
        let history_path = scope.join_path(&claude_home, &["history.jsonl"]);
        let projects_dir = scope.join_path(&claude_home, &["projects"]);

        if !backend.exists(&history_path).await {
            return Ok(vec![]);
        }

        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let mut seen: HashSet<String> = HashSet::new();
        let mut calls: Vec<SkillCall> = Vec::new();

        // Source 1: history.jsonl
        let content = backend.read_to_string(&history_path).await?;
        collect_from_history(&content, &builtins, &mut seen, &mut calls);

        // Source 2: projects/**/*.jsonl
        if backend.exists(&projects_dir).await {
            collect_from_projects(
                backend.as_ref(),
                &projects_dir,
                &builtins,
                &mut seen,
                &mut calls,
            )
            .await?;
        }

        Ok(calls)
    }
}

fn collect_from_history(
    content: &str,
    builtins: &HashSet<&str>,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) {
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let display = entry["display"].as_str().unwrap_or("");
        let caps = match skill_re().captures(display) {
            Some(c) => c,
            None => continue,
        };
        let skill = caps[1].to_string();
        if builtins.contains(skill.as_str()) {
            continue;
        }

        // history.jsonl 的 timestamp 是数字毫秒
        let ts = entry["timestamp"].as_i64().unwrap_or(0);
        let key = format!("{skill}:{ts}");
        if !seen.insert(key) {
            continue;
        }

        calls.push(SkillCall {
            skill,
            timestamp_ms: ts,
            project: entry["project"].as_str().unwrap_or("").to_string(),
            session_id: entry["sessionId"].as_str().unwrap_or("").to_string(),
            source: SOURCE.into(),
        });
    }
}

async fn collect_from_projects(
    backend: &dyn FsBackend,
    projects_dir: &str,
    builtins: &HashSet<&str>,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) -> Result<(), String> {
    // 顶层是按 cwd 编码的目录，一次扫一层进各项目，但内部 jsonl 可能嵌
    // 在 subagents/ 子目录里——所以通过 FsBackend 递归列出 jsonl。
    for path in backend.walk_jsonl(projects_dir).await? {
        parse_session_file(backend, &path, builtins, seen, calls).await;
    }
    Ok(())
}

async fn parse_session_file(
    backend: &dyn FsBackend,
    path: &str,
    builtins: &HashSet<&str>,
    seen: &mut HashSet<String>,
    calls: &mut Vec<SkillCall>,
) {
    let content = match backend.read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return,
    };

    let session_id = file_stem_from_path(path);

    // project (cwd) 从首个有 cwd 的行抽出来——比解码目录名靠谱。
    let mut project = String::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Fast path：早期行先 sniff cwd
        if project.is_empty() && line.contains("\"cwd\"") {
            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                if let Some(cwd) = entry["cwd"].as_str() {
                    project = cwd.to_string();
                }
            }
        }

        // 没有 "Skill" 字符串的行直接跳过——比 JSON 解析快很多
        if !line.contains("\"Skill\"") {
            continue;
        }

        let entry: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if project.is_empty() {
            if let Some(cwd) = entry["cwd"].as_str() {
                project = cwd.to_string();
            }
        }

        if entry["type"].as_str() != Some("assistant") {
            continue;
        }

        let content_arr = match entry["message"]["content"].as_array() {
            Some(a) => a,
            None => continue,
        };

        for part in content_arr {
            if part["type"].as_str() != Some("tool_use") {
                continue;
            }
            if part["name"].as_str() != Some("Skill") {
                continue;
            }

            let skill = match part["input"]["skill"].as_str() {
                Some(s) if !s.is_empty() && !builtins.contains(s) => s.to_string(),
                _ => continue,
            };

            // session jsonl 的 timestamp 通常是 ISO 字符串；用 chrono 解析。
            // 兼容罕见的数字 timestamp_ms 形式。
            let ts = parse_timestamp(&entry["timestamp"]);

            let key = format!("{skill}:{ts}");
            if !seen.insert(key) {
                continue;
            }

            let call_project = if project.is_empty() {
                entry["cwd"].as_str().unwrap_or("").to_string()
            } else {
                project.clone()
            };

            calls.push(SkillCall {
                skill,
                timestamp_ms: ts,
                project: call_project,
                session_id: session_id.clone(),
                source: SOURCE.into(),
            });
        }
    }
}

fn file_stem_from_path(path: &str) -> String {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or("");
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
        .to_string()
}

/// `entry["timestamp"]` 的形态可能是：
/// - ISO 8601 字符串：`"2026-05-15T20:43:17.000Z"` ← session JSONL 常见
/// - Unix 毫秒数字：`1700000000000` ← history.jsonl 常见
/// - 缺失或异常：返回 0
fn parse_timestamp(value: &Value) -> i64 {
    if let Some(n) = value.as_i64() {
        return n;
    }
    if let Some(s) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return dt.timestamp_millis();
        }
        // 退路：直接当数字解析
        if let Ok(n) = s.parse::<i64>() {
            return n;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::usage::ENV_LOCK;
    use std::fs;
    use tempfile::TempDir;

    /// 简化版 block_on —— 测试用，避免引入 tokio_test 依赖。
    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    /// 构造一个假的 ~/.claude 目录树，写入 history.jsonl 与若干 session JSONL。
    /// 把 `CLAUDE_CONFIG_DIR` 环境变量指向这里，以隔离真实用户主目录。
    fn make_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        let claude_home = dir.path();

        let history = vec![
            // 合法 skill 调用
            r#"{"display":"/review","project":"/p1","sessionId":"hsess1","timestamp":1700000000000}"#,
            // BUILTINS 命中 — 应当过滤
            r#"{"display":"/clear","project":"/p1","sessionId":"hsess1","timestamp":1700000001000}"#,
            // 不是 slash 开头 — 应当过滤
            r#"{"display":"hello","project":"/p1","sessionId":"hsess1","timestamp":1700000002000}"#,
            // 重复 (skill,ts) — 应当只算一次
            r#"{"display":"/review","project":"/p1","sessionId":"hsess1","timestamp":1700000000000}"#,
            // 不同 skill — 应当保留
            r#"{"display":"/facts-discover","project":"/p2","sessionId":"hsess2","timestamp":1700000010000}"#,
            // 损坏 JSON — 应当跳过不 panic
            r#"{not valid json"#,
            // 空行
            "",
        ]
        .join("\n");
        fs::write(claude_home.join("history.jsonl"), history).unwrap();

        // session 文件结构：projects/<encoded-cwd>/<session>.jsonl
        let proj_dir = claude_home.join("projects").join("-tmp-myproj");
        fs::create_dir_all(&proj_dir).unwrap();

        let session = vec![
            // session 起始：含 cwd 给 project 字段
            r#"{"type":"summary","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:13:20.000Z"}"#,
            // 真正的 Skill tool_use
            r#"{"type":"assistant","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:14:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"git-commit"}}]}}"#,
            // 同 skill 同 ts — 应当被去重（这里 ts 不同，所以保留）
            r#"{"type":"assistant","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:15:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"git-commit"}}]}}"#,
            // BUILTINS — 应当过滤（虽然 Skill tool_use 中很少出现 builtins，但要校验）
            r#"{"type":"assistant","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:16:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"clear"}}]}}"#,
            // type 不是 assistant — 应当过滤
            r#"{"type":"user","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:17:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"git-commit"}}]}}"#,
            // tool name 不是 Skill — 应当过滤
            r#"{"type":"assistant","cwd":"/tmp/myproj","timestamp":"2023-11-14T22:18:00.000Z","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}]}}"#,
        ]
        .join("\n");
        fs::write(proj_dir.join("sessabc.jsonl"), session).unwrap();

        dir
    }

    fn run_collect(dir: &TempDir) -> Vec<SkillCall> {
        std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());
        // 为了避免并发测试相互污染，每次 collect 完都清掉环境变量。
        let result =
            tokio_test_block_on(async { ClaudeCodeProvider.collect(&Scope::Local).await.unwrap() });
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        result
    }

    #[test]
    fn available_returns_true_only_when_history_exists() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());
        // 没有 history.jsonl
        assert!(!tokio_test_block_on(
            ClaudeCodeProvider.available(&Scope::Local)
        ));
        // 创建后应当 true
        std::fs::write(dir.path().join("history.jsonl"), "").unwrap();
        assert!(tokio_test_block_on(
            ClaudeCodeProvider.available(&Scope::Local)
        ));
        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn collect_extracts_history_and_session_calls_with_dedup_and_builtins_filter() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = make_fixture();
        let calls = run_collect(&dir);

        // 期望：
        // - history: review(1700000000000) + facts-discover(1700000010000)，clear/hello/重复/损坏被过滤
        // - session: git-commit(2023-11-14T22:14) + git-commit(2023-11-14T22:15)，clear/user/Bash 被过滤
        // 总共 4 条。
        assert_eq!(calls.len(), 4, "expected 4 calls, got: {calls:#?}");

        // 验证去重：相同 (skill, ts) 不出现两次
        let mut keys: Vec<String> = calls
            .iter()
            .map(|c| format!("{}:{}", c.skill, c.timestamp_ms))
            .collect();
        let unique_count = {
            keys.sort();
            keys.dedup();
            keys.len()
        };
        assert_eq!(unique_count, 4, "calls should be deduped");

        // 验证 source 都是 "Claude Code"
        for c in &calls {
            assert_eq!(c.source, "Claude Code");
        }

        // 验证 BUILTINS 过滤生效
        assert!(
            !calls.iter().any(|c| c.skill == "clear"),
            "BUILTINS skill 'clear' must be filtered"
        );

        // 验证 history.jsonl 来源的两条
        assert!(calls
            .iter()
            .any(|c| c.skill == "review" && c.session_id == "hsess1"));
        assert!(calls
            .iter()
            .any(|c| c.skill == "facts-discover" && c.project == "/p2"));

        // 验证 session JSONL 来源的两条 git-commit
        let git_commit: Vec<_> = calls.iter().filter(|c| c.skill == "git-commit").collect();
        assert_eq!(git_commit.len(), 2);
        for c in git_commit {
            assert_eq!(c.session_id, "sessabc");
            assert_eq!(c.project, "/tmp/myproj");
            assert!(
                c.timestamp_ms > 0,
                "ISO timestamp should be parsed to epoch ms"
            );
        }
    }

    #[test]
    fn collect_returns_empty_when_no_history() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        std::env::set_var("CLAUDE_CONFIG_DIR", dir.path());
        let calls =
            tokio_test_block_on(async { ClaudeCodeProvider.collect(&Scope::Local).await.unwrap() });
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert!(calls.is_empty());
    }

    // Note：原 "collect_returns_empty_for_remote_scope_in_p1" 测试在 P2
    // 之后过时——Scope::Remote 现在需要 `connection: Arc<ConnectedRemoteTarget>`，
    // 单测中无法廉价构造。Remote 行为通过 services/usage 的集成测试 +
    // active target 切换钩子（Task 13）去验证。

    #[test]
    fn parse_timestamp_handles_iso_and_numeric() {
        assert_eq!(parse_timestamp(&Value::Number(1234.into())), 1234);
        assert_eq!(parse_timestamp(&Value::String("1234".into())), 1234);
        let iso = parse_timestamp(&Value::String("2023-11-14T22:13:20.000Z".into()));
        assert!(iso > 0);
        assert_eq!(parse_timestamp(&Value::Null), 0);
    }
}
