//! 文件系统访问抽象。
//!
//! 让 5 个真实 provider 在不知道自己是跑在本地还是远程 SSH/WSL target 上
//! 的前提下，统一通过这层 trait 调用底层 IO。
//!
//! - [`LocalFsBackend`] 包 std::fs / walkdir / tokio::fs
//! - [`RemoteFsBackend`] 包 [`crate::targets::exec::ConnectedRemoteTarget`]
//!   提供的 SSH 执行 + 文件读取
//! - 测试中可用的 mock 由测试 mod 自行 stub
//!
//! 远程访问的几个关键差异：
//! - SQLite 文件（OpenCode）必须先 `fetch_to_local` 拉到本地 tempdir，
//!   再用 sqlx 只读打开（远程 sqlite3 不可移植，且需要文件锁）。
//! - `walk_jsonl` 走 `find <root> -name '*.jsonl' -type f`，
//!   一次 SSH round trip 拿全列表，避免每个目录单独 list。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use crate::targets::ConnectedRemoteTarget;

/// 简化目录条目：调用方仅关心是否目录与名称。
#[derive(Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
}

#[async_trait]
pub trait FsBackend: Send + Sync {
    /// 路径存在性探测。失败时返回 false（不向上抛 Err，让 provider 行为
    /// 与 std::fs::Path::exists 一致）。
    async fn exists(&self, path: &str) -> bool;

    /// 整个文件读成 UTF-8 字符串。读取失败或编码错误返回 Err。
    async fn read_to_string(&self, path: &str) -> Result<String, String>;

    /// 递归列出 root 目录下所有 .jsonl 文件的绝对路径。Local 用 walkdir，
    /// Remote 用 `find -name '*.jsonl' -type f`。
    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, String>;

    /// 列出某层目录的直接条目（只一层）。给 Grok 这种按
    /// 目录名前缀过滤的 provider 用。
    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, String>;

    /// 把（可能远程）文件拉到本地真实 PathBuf，便于 SQLite 等需要文件
    /// 句柄的 API。Local backend 直接返回原路径；Remote backend 拉到
    /// `tempfile::TempDir` 并返回临时路径。
    ///
    /// 调用方持有返回的 PathBuf 时必须连带保留 `_keepalive` 引用——
    /// `BackendFetched` 的析构会清理临时目录。
    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, String>;
}

/// `fetch_to_local` 返回值：本地路径 + 一个生命周期句柄。
/// 句柄持有时确保临时目录不被 GC。
pub struct BackendFetched {
    pub local_path: PathBuf,
    /// 仅 RemoteFsBackend 用得到；LocalFsBackend 留 None。
    /// `tempfile::TempDir` 在 drop 时自动清理。
    _keepalive: Option<tempfile::TempDir>,
}

// ─── Local ───────────────────────────────────────────────────────────────────

pub struct LocalFsBackend;

#[async_trait]
impl FsBackend for LocalFsBackend {
    async fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    async fn read_to_string(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("local read {}: {}", path, e))
    }

    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, String> {
        let root_path = PathBuf::from(root);
        if !root_path.is_dir() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&root_path)
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("jsonl") && p.is_file() {
                if let Some(s) = p.to_str() {
                    out.push(s.to_string());
                }
            }
        }
        Ok(out)
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, String> {
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for ent in entries.flatten() {
            let name = ent.file_name().to_string_lossy().into_owned();
            let is_dir = ent.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push(FsEntry { name, is_dir });
        }
        Ok(out)
    }

    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, String> {
        Ok(BackendFetched {
            local_path: PathBuf::from(path),
            _keepalive: None,
        })
    }
}

// ─── Remote (SSH) ────────────────────────────────────────────────────────────

/// SSH/WSL 远程 backend。`Arc<ConnectedRemoteTarget>` 让多 provider 共享同
/// 一个连接对象（避免每个 provider 都开一条 SSH 通道）。
pub struct RemoteFsBackend {
    pub target: Arc<ConnectedRemoteTarget>,
}

impl RemoteFsBackend {
    pub fn new(target: Arc<ConnectedRemoteTarget>) -> Self {
        Self { target }
    }
}

#[async_trait]
impl FsBackend for RemoteFsBackend {
    async fn exists(&self, path: &str) -> bool {
        self.target.exists(path).await.unwrap_or(false)
    }

    async fn read_to_string(&self, path: &str) -> Result<String, String> {
        let bytes = self.target.read_file(path).await?;
        String::from_utf8(bytes).map_err(|e| format!("remote utf8 {}: {}", path, e))
    }

    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, String> {
        // `find` 在大多数 *nix 都有；返回每行一个绝对路径
        let cmd = format!(
            "find {} -type f -name '*.jsonl' 2>/dev/null",
            shell_escape(root)
        );
        let stdout = self.target.run_command(&cmd).await.unwrap_or_default();
        Ok(stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, String> {
        let entries = self.target.list_dir(path).await.unwrap_or_default();
        Ok(entries
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                is_dir: e.file_type == "dir",
            })
            .collect())
    }

    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, String> {
        let bytes = self.target.read_file(path).await?;
        let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {}", e))?;
        let basename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("remote.bin");
        let local_path = dir.path().join(basename);
        std::fs::write(&local_path, &bytes).map_err(|e| format!("write tmp: {}", e))?;
        Ok(BackendFetched {
            local_path,
            _keepalive: Some(dir),
        })
    }
}

/// 简单 shell 转义：用单引号包裹并将内部单引号 ' → '\'' 转义。
/// 与 targets/exec.rs 的 shell_quote 等价但避免跨模块依赖私有项。
fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn local_backend_round_trips_basic_io() {
        let dir = TempDir::new().unwrap();
        let f1 = dir.path().join("a.jsonl");
        let f2 = dir.path().join("nested").join("b.jsonl");
        std::fs::create_dir_all(f2.parent().unwrap()).unwrap();
        std::fs::write(&f1, "hello").unwrap();
        std::fs::write(&f2, "world").unwrap();

        let backend = LocalFsBackend;
        assert!(backend.exists(f1.to_str().unwrap()).await);
        assert!(!backend.exists("/totally/nonexistent").await);

        let body = backend.read_to_string(f1.to_str().unwrap()).await.unwrap();
        assert_eq!(body, "hello");

        let jsonls = backend
            .walk_jsonl(dir.path().to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(jsonls.len(), 2);

        let entries = backend
            .list_entries(dir.path().to_str().unwrap())
            .await
            .unwrap();
        assert!(entries.iter().any(|e| e.name == "a.jsonl" && !e.is_dir));
        assert!(entries.iter().any(|e| e.name == "nested" && e.is_dir));

        let fetched = backend.fetch_to_local(f1.to_str().unwrap()).await.unwrap();
        // Local 直接返回原路径
        assert_eq!(fetched.local_path, f1);
    }

    #[test]
    fn shell_escape_handles_quotes_and_empty() {
        assert_eq!(shell_escape(""), "''");
        assert_eq!(shell_escape("plain"), "'plain'");
        assert_eq!(shell_escape("can't"), "'can'\\''t'");
    }
}
