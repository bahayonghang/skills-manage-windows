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

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use super::UsageError;
use crate::targets::{shell_quote, ConnectedRemoteTarget};

const PATH_MARKER: &str = "\u{001e}PATH\t";
const EOF_MARKER: &str = "\u{001f}EOF";
const REMOTE_READ_CHUNK_SIZE: usize = 64;

/// 简化目录条目：调用方仅关心是否目录与名称。
#[derive(Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
}

#[async_trait]
pub trait FsBackend: Send + Sync {
    /// 路径存在性探测。确认 missing 为 `Ok(false)`；transport / protocol /
    /// permission failure 为 `Err`。不得把远程失败伪装成不存在。
    async fn exists(&self, path: &str) -> Result<bool, UsageError>;

    /// 整个文件读成 UTF-8 字符串。读取失败或编码错误返回 Err。
    async fn read_to_string(&self, path: &str) -> Result<String, UsageError>;

    /// 批量读多个 UTF-8 文本文件。默认逐文件读取；Remote backend 会覆盖成
    /// 单次/分批 SSH 脚本，减少 round trips。
    async fn read_many_to_strings(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, String>, UsageError> {
        let mut content_by_path = HashMap::new();
        for path in paths {
            if let Ok(content) = self.read_to_string(path).await {
                content_by_path.insert(path.clone(), content);
            }
        }
        Ok(content_by_path)
    }

    /// 递归列出 root 目录下所有 .jsonl 文件的绝对路径。Local 用 walkdir，
    /// Remote 用 `find -name '*.jsonl' -type f`。
    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, UsageError>;

    /// 列出某层目录的直接条目（只一层）。给 Grok 这种按
    /// 目录名前缀过滤的 provider 用。
    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, UsageError>;

    /// 把（可能远程）文件拉到本地真实 PathBuf，便于 SQLite 等需要文件
    /// 句柄的 API。Local backend 直接返回原路径；Remote backend 拉到
    /// `tempfile::TempDir` 并返回临时路径。
    ///
    /// 调用方持有返回的 PathBuf 时必须连带保留 `_keepalive` 引用——
    /// `BackendFetched` 的析构会清理临时目录。
    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, UsageError>;
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
    async fn exists(&self, path: &str) -> Result<bool, UsageError> {
        match std::fs::metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(UsageError::io("local exists", error)),
        }
    }

    async fn read_to_string(&self, path: &str) -> Result<String, UsageError> {
        let path = path.to_string();
        crate::fs_util::run_blocking_fs_with(
            "usage file read",
            move || {
                std::fs::read_to_string(&path)
                    .map_err(|e| UsageError::io(format!("local read {}", path), e))
            },
            UsageError::task_join,
        )
        .await
    }

    async fn read_many_to_strings(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, String>, UsageError> {
        let paths = paths.to_vec();
        crate::fs_util::run_blocking_fs_with(
            "usage batch file read",
            move || {
                let mut content_by_path = HashMap::new();
                for path in paths {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        content_by_path.insert(path, content);
                    }
                }
                Ok(content_by_path)
            },
            UsageError::task_join,
        )
        .await
    }

    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, UsageError> {
        let root = root.to_string();
        crate::fs_util::run_blocking_fs_with(
            "usage jsonl walk",
            move || {
                let root_path = PathBuf::from(&root);
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
            },
            UsageError::task_join,
        )
        .await
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, UsageError> {
        let path = path.to_string();
        crate::fs_util::run_blocking_fs_with(
            "usage directory listing",
            move || {
                let entries = match std::fs::read_dir(&path) {
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
            },
            UsageError::task_join,
        )
        .await
    }

    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, UsageError> {
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
    async fn exists(&self, path: &str) -> Result<bool, UsageError> {
        self.target
            .exists(path)
            .await
            .map_err(UsageError::from_remote)
    }

    async fn read_to_string(&self, path: &str) -> Result<String, UsageError> {
        let bytes = self
            .target
            .read_file(path)
            .await
            .map_err(UsageError::from_remote)?;
        String::from_utf8(bytes)
            .map_err(|_| UsageError::Parse("Remote log content is not valid UTF-8.".to_string()))
    }

    async fn read_many_to_strings(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, String>, UsageError> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }

        let mut content_by_path = HashMap::new();
        for chunk in paths.chunks(REMOTE_READ_CHUNK_SIZE) {
            let script = build_batch_read_script(chunk);
            let stdout = self
                .target
                .run_command(&script)
                .await
                .map_err(UsageError::from_remote)?;
            content_by_path.extend(parse_batch_read_output(&stdout));
        }
        Ok(content_by_path)
    }

    async fn walk_jsonl(&self, root: &str) -> Result<Vec<String>, UsageError> {
        if !self.exists(root).await? {
            return Ok(Vec::new());
        }
        // Confirmed-present root: do not swallow stderr. Empty stdout is a
        // legal zero-entry success; any transport/protocol/permission failure
        // stays an error.
        let cmd = format!("find {} -type f -name '*.jsonl'", shell_quote(root));
        let stdout = self
            .target
            .run_command(&cmd)
            .await
            .map_err(UsageError::from_remote)?;
        Ok(stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<FsEntry>, UsageError> {
        if !self.exists(path).await? {
            return Ok(Vec::new());
        }
        let entries = self
            .target
            .list_dir(path)
            .await
            .map_err(UsageError::from_remote)?;
        Ok(entries
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                is_dir: e.file_type == "dir",
            })
            .collect())
    }

    async fn fetch_to_local(&self, path: &str) -> Result<BackendFetched, UsageError> {
        let bytes = self
            .target
            .read_file(path)
            .await
            .map_err(UsageError::from_remote)?;
        let dir = tempfile::TempDir::new().map_err(|e| UsageError::io("tempdir", e))?;
        let basename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("remote.bin");
        let local_path = dir.path().join(basename);
        let local_path_for_write = local_path.clone();
        crate::fs_util::run_blocking_fs_with(
            "remote file staging",
            move || {
                std::fs::write(&local_path_for_write, &bytes)
                    .map_err(|e| UsageError::io("write tmp", e))
            },
            UsageError::task_join,
        )
        .await?;
        Ok(BackendFetched {
            local_path,
            _keepalive: Some(dir),
        })
    }
}

fn build_batch_read_script(paths: &[String]) -> String {
    let mut script = String::from("set -eu\n");
    script.push_str("for path in");
    for path in paths {
        script.push(' ');
        script.push_str(&shell_quote(path));
    }
    script.push_str("; do\n");
    script.push_str(&format!("  printf '{}%s\\n' \"$path\"\n", PATH_MARKER));
    script.push_str("  if [ -r \"$path\" ]; then\n");
    script.push_str("    cat -- \"$path\"\n");
    script.push_str("  fi\n");
    script.push_str(&format!("  printf '{}\\n'\n", EOF_MARKER));
    script.push_str("done\n");
    script
}

fn parse_batch_read_output(output: &str) -> HashMap<String, String> {
    let mut content_by_path = HashMap::new();
    let mut remaining = output;
    while let Some(start) = remaining.find(PATH_MARKER) {
        let after_marker = &remaining[start + PATH_MARKER.len()..];
        let Some((path, after_path)) = after_marker.split_once('\n') else {
            break;
        };
        let Some(end) = after_path.find(EOF_MARKER) else {
            break;
        };
        let body = &after_path[..end];
        content_by_path.insert(path.to_string(), body.to_string());
        remaining = &after_path[end + EOF_MARKER.len()..];
    }

    content_by_path
}

#[cfg(test)]
fn encode_batch_read_output(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(path, content)| format!("{PATH_MARKER}{path}\n{content}{EOF_MARKER}\n"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
pub(crate) mod test_fixtures {
    use std::sync::Arc;

    use super::RemoteFsBackend;
    use crate::services::usage::Scope;
    use crate::targets::{
        ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
        SshAuthMethod, WslTargetConfig,
    };
    use crate::test_support::FakeRunner;

    pub(crate) fn ssh_target() -> RemoteTargetConfig {
        RemoteTargetConfig {
            id: "ssh-usage-test".to_string(),
            label: "SSH usage test".to_string(),
            host: "example.invalid".to_string(),
            username: "alice".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        }
    }

    pub(crate) fn wsl_target() -> WslTargetConfig {
        WslTargetConfig {
            id: "wsl-usage-test".to_string(),
            label: "WSL usage test".to_string(),
            distribution: "Ubuntu-24.04".to_string(),
            remote_home: "/home/alice".to_string(),
            remote_os: "Linux".to_string(),
            symlink_enabled: true,
        }
    }

    pub(crate) fn ssh_backend(runner: Arc<FakeRunner>) -> RemoteFsBackend {
        RemoteFsBackend::new(Arc::new(ConnectedRemoteTarget::Ssh(
            ConnectedSshTarget::for_tests_with_runner(ssh_target(), runner),
        )))
    }

    pub(crate) fn wsl_backend(runner: Arc<FakeRunner>) -> RemoteFsBackend {
        RemoteFsBackend::new(Arc::new(ConnectedRemoteTarget::Wsl(
            ConnectedWslTarget::for_tests_with_runner(wsl_target(), runner),
        )))
    }

    pub(crate) fn ssh_scope(runner: Arc<FakeRunner>, target_id: &str) -> Scope {
        let mut target = ssh_target();
        target.id = target_id.to_string();
        Scope::Remote {
            target_id: target_id.to_string(),
            remote_home: target.remote_home.clone(),
            connection: Arc::new(ConnectedRemoteTarget::Ssh(
                ConnectedSshTarget::for_tests_with_runner(target, runner),
            )),
        }
    }

    pub(crate) fn wsl_scope(runner: Arc<FakeRunner>, target_id: &str) -> Scope {
        let mut target = wsl_target();
        target.id = target_id.to_string();
        Scope::Remote {
            target_id: target_id.to_string(),
            remote_home: target.remote_home.clone(),
            connection: Arc::new(ConnectedRemoteTarget::Wsl(
                ConnectedWslTarget::for_tests_with_runner(target, runner),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::RunnerPhase;
    use crate::test_support::FakeRunner;
    use std::sync::Arc;
    use tempfile::TempDir;

    const PATH_SEED: &str = "/home/alice/.codex/sessions";
    const STDERR_SEED: &str = "Permission denied: /home/alice/.ssh/id_ed25519";
    const COMMAND_SEED: &str = "find /home/alice/.codex -name '*.jsonl'";
    const HOST_SEED: &str = "alice@prod.example.invalid";

    fn adversarial_detail() -> String {
        format!("{STDERR_SEED}\n{COMMAND_SEED}\n{HOST_SEED}")
    }

    fn assert_no_remote_diagnostics(error: &UsageError) {
        let public = error.to_string();
        for seed in [PATH_SEED, STDERR_SEED, COMMAND_SEED, HOST_SEED] {
            assert!(!public.contains(seed), "leaked {seed}: {public}");
        }
        assert!(error.is_target_fatal());
    }

    async fn assert_exists_three_state(backend: &RemoteFsBackend, runner: &FakeRunner) {
        runner.push_output(0, "", "");
        assert!(backend.exists(PATH_SEED).await.unwrap());

        runner.push_output(1, "", "");
        assert!(!backend.exists(PATH_SEED).await.unwrap());

        runner.push_timeout();
        let transport = backend.exists(PATH_SEED).await.unwrap_err();
        assert_eq!(transport.stable_code(), "usage.remote_transport");
        assert!(transport.retryable());
        assert_no_remote_diagnostics(&transport);

        runner.push_output(2, "", &adversarial_detail());
        let permission = backend.exists(PATH_SEED).await.unwrap_err();
        assert_eq!(permission.stable_code(), "usage.remote_permission");
        assert!(!permission.retryable());
        assert_no_remote_diagnostics(&permission);
    }

    async fn assert_walk_and_list_three_state(backend: &RemoteFsBackend, runner: &FakeRunner) {
        runner.push_output(1, "", "");
        assert!(backend.walk_jsonl(PATH_SEED).await.unwrap().is_empty());

        runner.push_output(0, "", "");
        runner.push_success("");
        assert!(backend.walk_jsonl(PATH_SEED).await.unwrap().is_empty());

        runner.push_output(0, "", "");
        runner.push_success("/home/alice/.codex/sessions/a.jsonl\n");
        let found = backend.walk_jsonl(PATH_SEED).await.unwrap();
        assert_eq!(found, vec!["/home/alice/.codex/sessions/a.jsonl"]);

        runner.push_error(RunnerPhase::Start, "connection refused");
        let walk_transport = backend.walk_jsonl(PATH_SEED).await.unwrap_err();
        assert_eq!(walk_transport.stable_code(), "usage.remote_transport");
        assert_no_remote_diagnostics(&walk_transport);

        runner.push_output(0, "", "");
        runner.push_output(1, "", &adversarial_detail());
        let walk_permission = backend.walk_jsonl(PATH_SEED).await.unwrap_err();
        assert_eq!(walk_permission.stable_code(), "usage.remote_permission");
        assert_no_remote_diagnostics(&walk_permission);

        runner.push_output(0, "", "");
        runner.push_output_bytes(0, &[0xff, 0xfe], b"");
        let walk_protocol = backend.walk_jsonl(PATH_SEED).await.unwrap_err();
        assert_eq!(walk_protocol.stable_code(), "usage.remote_protocol");
        assert!(!walk_protocol.retryable());
        assert_no_remote_diagnostics(&walk_protocol);

        runner.push_output(1, "", "");
        assert!(backend.list_entries(PATH_SEED).await.unwrap().is_empty());

        runner.push_output(0, "", "");
        runner.push_success("");
        assert!(backend.list_entries(PATH_SEED).await.unwrap().is_empty());

        runner.push_error(RunnerPhase::Start, "broken pipe");
        let list_transport = backend.list_entries(PATH_SEED).await.unwrap_err();
        assert_eq!(list_transport.stable_code(), "usage.remote_transport");
        assert_no_remote_diagnostics(&list_transport);

        runner.push_output(0, "", "");
        runner.push_output(13, "", &adversarial_detail());
        let list_permission = backend.list_entries(PATH_SEED).await.unwrap_err();
        assert_eq!(list_permission.stable_code(), "usage.remote_permission");
        assert_no_remote_diagnostics(&list_permission);
    }

    #[tokio::test]
    async fn fake_ssh_exists_walk_and_list_use_three_state_results() {
        let runner = Arc::new(FakeRunner::new());
        let backend = test_fixtures::ssh_backend(runner.clone());
        assert_exists_three_state(&backend, &runner).await;
        assert_walk_and_list_three_state(&backend, &runner).await;
    }

    #[tokio::test]
    async fn fake_wsl_exists_walk_and_list_use_three_state_results() {
        let runner = Arc::new(FakeRunner::new());
        let backend = test_fixtures::wsl_backend(runner.clone());
        assert_exists_three_state(&backend, &runner).await;
        assert_walk_and_list_three_state(&backend, &runner).await;
    }

    #[tokio::test]
    async fn local_backend_round_trips_basic_io() {
        let dir = TempDir::new().unwrap();
        let f1 = dir.path().join("a.jsonl");
        let f2 = dir.path().join("nested").join("b.jsonl");
        std::fs::create_dir_all(f2.parent().unwrap()).unwrap();
        std::fs::write(&f1, "hello").unwrap();
        std::fs::write(&f2, "world").unwrap();

        let backend = LocalFsBackend;
        assert!(backend.exists(f1.to_str().unwrap()).await.unwrap());
        assert!(!backend.exists("/totally/nonexistent").await.unwrap());

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
    fn build_batch_read_script_quotes_paths() {
        let script = build_batch_read_script(&["/tmp/demo path.txt".to_string()]);
        assert!(script.contains("'/tmp/demo path.txt'"));
        assert!(script.contains(PATH_MARKER));
        assert!(script.contains(EOF_MARKER));
    }

    #[test]
    fn parse_batch_read_output_preserves_special_characters_and_boundaries() {
        let encoded = encode_batch_read_output(&[
            (
                "/tmp/alpha.jsonl".to_string(),
                "{\"text\":\"line\\twith tab\"}\nsecond line\n".to_string(),
            ),
            (
                "/tmp/beta.jsonl".to_string(),
                "<skill><name>review</name></skill>\n".to_string(),
            ),
        ]);
        let parsed = parse_batch_read_output(&encoded);

        assert_eq!(
            parsed.get("/tmp/alpha.jsonl"),
            Some(&"{\"text\":\"line\\twith tab\"}\nsecond line\n".to_string())
        );
        assert_eq!(
            parsed.get("/tmp/beta.jsonl"),
            Some(&"<skill><name>review</name></skill>\n".to_string())
        );
    }
}
