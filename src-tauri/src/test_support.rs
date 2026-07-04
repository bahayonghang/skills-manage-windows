//! Shared test-support harness — the single source of test fixtures.
//!
//! 契约：新增 Rust 测试禁止手抄 `SqlitePool::connect(":memory:")` +
//! `init_database`，一律经本模块的池 fixture；skill 目录 / central skill
//! 行 / agent 目录重定向等常用 fixture 也从这里取。仅两类现场豁免：
//! 故意不建 schema 或手工建 legacy schema 的迁移类测试（现场注释声明），
//! 以及 `tests/` 下的集成测试 crate（`#[cfg(test)]` 模块对其不可见）。
//!
//! 语义基线：`mem_pool` 与历史手抄版本逐字节同款（默认池选项），不做
//! 「顺手改进」；需要单连接语义的测试用 `mem_pool_single_conn`。

use std::path::{Path, PathBuf};

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tempfile::TempDir;

use crate::db::{self, DbPool, Skill};

// ─── 池 fixture ──────────────────────────────────────────────────────────────

/// In-memory pool with the full schema initialised and built-in seeds
/// (agents / scan directories / registries) in place.
pub async fn mem_pool() -> DbPool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("connect in-memory SQLite pool");
    db::init_database(&pool).await.expect("init test database");
    pool
}

/// `max_connections(1)` variant for tests that must observe a single shared
/// connection (e.g. concurrent tasks sharing one in-memory database).
pub async fn mem_pool_single_conn() -> DbPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("connect single-conn in-memory SQLite pool");
    db::init_database(&pool).await.expect("init test database");
    pool
}

/// In-memory pool whose built-in agents point at a POSIX `home`
/// (remote-home semantics, see `db::init_database_for_remote_home`).
pub async fn mem_pool_with_home(home: &str) -> DbPool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("connect in-memory SQLite pool");
    db::init_database_for_remote_home(&pool, home)
        .await
        .expect("init remote-home test database");
    pool
}

/// File-backed pool (WAL mode via `db::create_pool`) inside a fresh
/// [`TempDir`] — for tests that need cross-connection visibility or real
/// files on disk. Keep the returned `TempDir` alive as long as the pool.
pub async fn file_pool() -> (DbPool, TempDir) {
    let dir = TempDir::new().expect("create tempdir for file-backed pool");
    let db_path = dir.path().join("test.sqlite");
    let pool = db::create_pool(&db_path.to_string_lossy())
        .await
        .expect("create file-backed pool");
    db::init_database(&pool).await.expect("init test database");
    (pool, dir)
}

// ─── agent fixture ───────────────────────────────────────────────────────────

/// Point `agents.global_skills_dir` for `agent_id` at `dir`. Works for the
/// central pseudo-agent (`agent_id = "central"`) as well as real platforms.
pub async fn set_agent_dir(pool: &DbPool, agent_id: &str, dir: &Path) {
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = ?")
        .bind(dir.to_string_lossy().into_owned())
        .bind(agent_id)
        .execute(pool)
        .await
        .expect("redirect agent skills dir");
}

// ─── skill fixture ───────────────────────────────────────────────────────────

/// Write a minimal `SKILL.md` into `dir` (creating the directory), keeping the
/// historical fixture frontmatter byte-for-byte. Returns `dir` as owned path.
pub fn write_skill_md(dir: &Path, name: &str, description: Option<&str>) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create skill dir");
    let body = match description {
        Some(d) => format!("---\nname: {name}\ndescription: {d}\n---\n\n# {name}\n"),
        None => format!("---\nname: {name}\n---\n\n# {name}\n"),
    };
    std::fs::write(dir.join("SKILL.md"), body).expect("write SKILL.md");
    dir.to_path_buf()
}

/// Neutral central-skill row skeleton: `is_central: true`, paths derived from
/// `canonical_dir`, everything else left for the caller to fill via
/// struct-update syntax (`Skill { source: Some(..), ..central_skill_row(..) }`).
pub fn central_skill_row(id: &str, canonical_dir: &Path) -> Skill {
    Skill {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        file_path: canonical_dir
            .join("SKILL.md")
            .to_string_lossy()
            .into_owned(),
        canonical_path: Some(canonical_dir.to_string_lossy().into_owned()),
        is_central: true,
        source: None,
        content: None,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

/// Write `SKILL.md` into `canonical_dir` and upsert the matching central
/// skill row (`description` lands in both the frontmatter and the DB row).
pub async fn seed_central_skill(pool: &DbPool, canonical_dir: &Path, id: &str, description: &str) {
    write_skill_md(canonical_dir, id, Some(description));
    let skill = Skill {
        description: Some(description.to_string()),
        ..central_skill_row(id, canonical_dir)
    };
    db::upsert_skill(pool, &skill)
        .await
        .expect("seed central skill row");
}

// ─── FS fixture ──────────────────────────────────────────────────────────────

/// Directory symlink helper for tests, dispatching on platform.
#[cfg(unix)]
pub fn symlink_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create test symlink");
}

/// Directory symlink helper for tests, dispatching on platform.
#[cfg(windows)]
pub fn symlink_dir(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).expect("create test symlink");
}

// ─── harness 自测 ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mem_pool_seeds_builtin_agents() {
        let pool = mem_pool().await;
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM agents WHERE id IN ('central','claude-code') ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            ids,
            vec![("central".to_string(),), ("claude-code".to_string(),)]
        );
    }

    #[tokio::test]
    async fn mem_pool_with_home_rewrites_agent_dirs() {
        let pool = mem_pool_with_home("/home/alice").await;
        let (dir,): (String,) =
            sqlx::query_as("SELECT global_skills_dir FROM agents WHERE id = 'claude-code'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(dir, "/home/alice/.claude/skills");
    }

    #[tokio::test]
    async fn set_agent_dir_roundtrips() {
        let pool = mem_pool().await;
        let tmp = TempDir::new().unwrap();
        set_agent_dir(&pool, "central", tmp.path()).await;
        let (dir,): (String,) =
            sqlx::query_as("SELECT global_skills_dir FROM agents WHERE id = 'central'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(dir, tmp.path().to_string_lossy().into_owned());
    }

    #[tokio::test]
    async fn write_skill_md_is_parseable() {
        let tmp = TempDir::new().unwrap();
        let dir = write_skill_md(&tmp.path().join("demo"), "demo", Some("A test skill"));
        let info = crate::services::scanner::parse_skill_md(&dir.join("SKILL.md"))
            .expect("parse SKILL.md");
        assert_eq!(info.name, "demo");
        assert_eq!(info.description.as_deref(), Some("A test skill"));
    }

    #[tokio::test]
    async fn file_pool_creates_schema_on_disk() {
        let (pool, dir) = file_pool().await;
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(count > 0, "builtin agents should be seeded");
        assert!(dir.path().join("test.sqlite").exists());
    }
}
