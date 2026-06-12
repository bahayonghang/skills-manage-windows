//! OpenCode provider —— `~/.local/share/opencode/opencode.db`
//!
//! 数据存在 SQLite。SQL 直接抓 `part p JOIN session s` 中
//! `data.type='tool' AND data.tool='skill' AND state.status='completed'` 的项，
//! 解出 skill name / start time / project (directory) / session_id。
//!
//! 我们用 sqlx (项目主依赖) 而非 skilled 用的 rusqlite，行为等价。
//! 库可能正被 OpenCode 进程占用，因此用 read-only 模式打开避免锁冲突。

use std::collections::HashSet;
use std::str::FromStr;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::services::usage::{Scope, SkillCall, UsageError, UsageProvider};

const SOURCE: &str = "OpenCode";

const BUILTINS: &[&str] = &[
    "bash", "compact", "help", "model", "config", "exit", "clear", "status", "version", "approve",
    "settings", "list",
];

pub struct OpenCodeProvider;

impl OpenCodeProvider {
    fn db_path(scope: &Scope) -> String {
        scope.join_home(&[".local", "share", "opencode", "opencode.db"])
    }
}

#[async_trait]
impl UsageProvider for OpenCodeProvider {
    fn id(&self) -> &'static str {
        "opencode"
    }
    fn display_name(&self) -> &'static str {
        SOURCE
    }

    async fn available(&self, scope: &Scope) -> bool {
        let backend = scope.fs_backend();
        backend.exists(&Self::db_path(scope)).await
    }

    async fn collect(&self, scope: &Scope) -> Result<Vec<SkillCall>, UsageError> {
        let backend = scope.fs_backend();
        let db_path = Self::db_path(scope);
        if !backend.exists(&db_path).await {
            return Ok(vec![]);
        }
        let fetched = backend.fetch_to_local(&db_path).await?;

        let url = format!(
            "sqlite://{}?mode=ro",
            fetched.local_path.to_string_lossy().replace('\\', "/")
        );
        let opts = SqliteConnectOptions::from_str(&url)?;
        // 关闭 wal 之类的写文件副作用 —— 我们只读。
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts.read_only(true))
            .await
            .map_err(UsageError::OpenCodeDbOpen)?;

        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT p.data, s.directory, p.session_id
             FROM part p
             JOIN session s ON p.session_id = s.id
             WHERE json_extract(p.data, '$.type') = 'tool'
               AND json_extract(p.data, '$.tool') = 'skill'",
        )
        .fetch_all(&pool)
        .await
        .map_err(UsageError::OpenCodeQuery)?;

        pool.close().await;

        let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();
        let mut calls = Vec::new();

        for (data_str, directory, session_id) in rows {
            let data: Value = match serde_json::from_str(&data_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let state = &data["state"];
            if state["status"].as_str() != Some("completed") {
                continue;
            }

            let skill = match state["input"]["name"].as_str() {
                Some(s) if !s.is_empty() && !builtins.contains(s) => s.to_string(),
                _ => continue,
            };

            let ts = match &state["time"]["start"] {
                Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                    .map(|d| d.timestamp_millis())
                    .unwrap_or(0),
                Value::Number(n) => n.as_i64().unwrap_or(0),
                _ => 0,
            };

            calls.push(SkillCall {
                skill,
                timestamp_ms: ts,
                project: directory,
                session_id,
                source: SOURCE.into(),
            });
        }

        Ok(calls)
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

    /// 构造一个最小可读的 opencode.db，并把 home 重定向到这里。
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

    async fn make_fixture_db(db_path: &std::path::Path) {
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let url = format!(
            "sqlite://{}?mode=rwc",
            db_path.to_string_lossy().replace('\\', "/")
        );
        let opts = SqliteConnectOptions::from_str(&url)
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE part (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, data TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO session (id, directory) VALUES ('sess1', '/home/me/proj')")
            .execute(&pool)
            .await
            .unwrap();

        // 三条 part：completed skill (应保留) / failed skill (过滤) / non-skill (过滤)
        for (id, data) in [
            (
                "p1",
                r#"{"type":"tool","tool":"skill","state":{"status":"completed","input":{"name":"review"},"time":{"start":"2024-03-01T12:00:00.000Z"}}}"#,
            ),
            (
                "p2",
                r#"{"type":"tool","tool":"skill","state":{"status":"failed","input":{"name":"x"},"time":{"start":"2024-03-01T12:01:00.000Z"}}}"#,
            ),
            (
                "p3",
                r#"{"type":"tool","tool":"bash","state":{"status":"completed","input":{"command":"ls"}}}"#,
            ),
            // BUILTINS 过滤
            (
                "p4",
                r#"{"type":"tool","tool":"skill","state":{"status":"completed","input":{"name":"clear"},"time":{"start":"2024-03-01T12:02:00.000Z"}}}"#,
            ),
        ] {
            sqlx::query("INSERT INTO part (id, session_id, data) VALUES (?, 'sess1', ?)")
                .bind(id)
                .bind(data)
                .execute(&pool)
                .await
                .unwrap();
        }
        pool.close().await;
    }

    #[test]
    fn collect_returns_only_completed_non_builtin_skills() {
        with_fake_home(|dir| {
            let db = dir
                .path()
                .join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db");
            block(async {
                make_fixture_db(&db).await;
            });
            let calls = block(OpenCodeProvider.collect(&Scope::Local)).unwrap();
            assert_eq!(calls.len(), 1, "got: {calls:#?}");
            assert_eq!(calls[0].skill, "review");
            assert_eq!(calls[0].source, "OpenCode");
            assert_eq!(calls[0].project, "/home/me/proj");
            assert_eq!(calls[0].session_id, "sess1");
            assert!(calls[0].timestamp_ms > 0);
        });
    }
}
