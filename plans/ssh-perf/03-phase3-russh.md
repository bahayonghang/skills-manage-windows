# 阶段 3 — 换 russh 库

## 目标

把 SSH 实现从"每次 spawn ssh.exe 子进程"改造为"持久 russh client + 跨命令复用 session/channel"。

**用户决策（2026-05-06）：直接换 russh，不走 ControlMaster 中间路线。**

## 选型理由

```text
┌──────────────┬──────────────────────────┬─────────────────────────┐
│ 维度         │ ControlMaster            │ russh                   │
├──────────────┼──────────────────────────┼─────────────────────────┤
│ 改动量       │ 小                       │ 大                      │
│ Windows 支持 │ 受限（命名管道不稳）      │ 一致                    │
│ 复用粒度     │ 进程级（仍要 spawn ssh）  │ session 级（无 spawn）  │
│ 取消控制     │ kill 子进程              │ 直接 close channel      │
│ 错误诊断     │ 解析 stderr 文本         │ 类型化错误              │
│ 测试         │ 难（要真机）              │ 单测可走 mock transport │
│ 长期方向     │ 临时方案                 │ 终态                    │
└──────────────┴──────────────────────────┴─────────────────────────┘
```

详细 ADR 见 `plans/ssh-perf/decisions.md`。

## 任务清单

### 3.1 加 russh 依赖

`src-tauri/Cargo.toml`：

```text
[dependencies]
russh = "0.45"          # 主库（须查最新版）
russh-keys = "0.45"     # 私钥解析
russh-sftp = "2"        # 可选，若想用 SFTP 替代 cat（性能更好）
async-trait = "0.1"     # russh handler 用
```

确认版本时跑 `cargo update --dry-run` 与 `cargo audit`。

### 3.2 重写 `ConnectedSshTarget` 内核

文件：`src-tauri/src/targets/exec.rs`（替换大半内容）

新结构：

```rust
pub struct ConnectedSshTarget {
    target: RemoteTargetConfig,
    handle: Mutex<russh::client::Handle<SkillportClientHandler>>,
    remote_home: OnceCell<String>,
    remote_os: OnceCell<String>,
}
```

`SkillportClientHandler` 实现 `russh::client::Handler`：
- `check_server_key`：实现 known_hosts 持久化（首次 accept-new，之后严格校验）
- 其他默认实现

### 3.3 client 池：同 target 复用

新增 `src-tauri/src/targets/client_pool.rs`：

```rust
pub struct SshClientPool {
    inner: tokio::sync::Mutex<HashMap<String, Arc<ConnectedSshTarget>>>,
}

impl SshClientPool {
    pub async fn get_or_connect(&self, target: &RemoteTargetConfig)
      -> Result<Arc<ConnectedSshTarget>, String>;

    pub async fn invalidate(&self, target_id: &str);
    pub async fn close_all(&self);
}
```

接入：
- `AppState` 持有 `Arc<SshClientPool>`
- `connect_ssh_target` 改为查池或新建
- 应用退出 hook 调 `close_all`

### 3.4 单命令执行 → channel exec

替换 `run_command`、`run_command_bytes`、`run_command_with_stdin`：

```rust
impl ConnectedSshTarget {
    pub async fn run_command(&self, command: &str) -> Result<String, String> {
        let mut h = self.handle.lock().await;
        let mut channel = h.channel_open_session().await?;
        channel.exec(true, command).await?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_status: Option<u32> = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, ext: 1 } => stderr.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status: c } => exit_status = Some(c),
                ChannelMsg::Eof => break,
                _ => {}
            }
        }
        // 同原行为：非零退出 → Err
    }
}
```

并发：阶段 4 加 `tokio::sync::Semaphore` 限制同 target 同时打开 channel ≤ 4。

### 3.5 known_hosts 持久化

russh 的 `check_server_key` 自实现：
- 路径：`<app_data>/known_hosts`（与系统 ssh 隔离）
- 算法：与 OpenSSH `known_hosts` 兼容格式（hashed hostname 可选）
- 首次连接：accept-new + 写入
- 后续：严格匹配，不匹配 Err
- UI 层提供"信任并替换"操作（设置页加按钮）

新增 `src-tauri/src/targets/known_hosts.rs`。

### 3.6 认证

key 模式：

```rust
let key = russh_keys::load_secret_key(&target.key_path, None)?;
handle.authenticate_publickey(&target.username, Arc::new(key)).await?;
```

password 模式：

```rust
let pwd = load_target_password(target)?;
handle.authenticate_password(&target.username, &pwd).await?;
```

key 含 passphrase 时，从 cred store 读 passphrase（或 UI 输入）。

`askpass.rs` 在阶段 3 后**整个删除**。

### 3.7 ssh.exe 路径整体下线

现有 `ssh_program()`、`base_command()`、`maybe_run_ssh_askpass_helper`、askpass 临时文件全部删除。**不保留**。

`Cargo.toml` 中如有 `windows-sys` 等仅为 `CREATE_NO_WINDOW` 而引入的依赖，确认是否还有其他用途，按需保留。

### 3.8 数据流改造

阶段 2 的 `run_bash_script` 现在直接走 channel：

```rust
let mut channel = handle.channel_open_session().await?;
channel.exec(true, "bash --noprofile --norc -s").await?;
channel.data(script_body.as_bytes()).await?;
channel.eof().await?;
// 收 stdout
```

文件读：用 SFTP（russh-sftp）替代脚本 cat，性能更好且能拿到准确的 stat：

```rust
let sftp = handle.sftp().await?;
let mut file = sftp.open(path).await?;
let mut buf = Vec::new();
file.read_to_end(&mut buf).await?;
```

权衡：阶段 2 已经做了 cat 批量；阶段 3 用 SFTP 是替代方案，先用 channel exec + bash 批读保持阶段 2 协议；SFTP 列入阶段 5 评估项。

### 3.9 安装路径同步改造

`linker.rs`、`installation/remote.rs`、`central_skills/files.rs`、`central_skills/delete.rs`、`github_import/remote.rs` 中所有调用 `connection.run_command/list_dir/exists/...` 的地方都通过新的 `ConnectedSshTarget` 接口走，行为不变。**接口签名保持兼容**，只换底层。

### 3.10 cancel 接入

`tokio::sync::CancellationToken` 传入：
- `run_command(cmd, cancel_token)` 检测到 cancel 立即 close channel 返回 Err("cancelled")
- 阶段 4 的 cancel_scan IPC 调到这里

## 关键设计决策

| 决策点                       | 选择                                                |
|------------------------------|----------------------------------------------------|
| 是否保留 ssh.exe fallback   | 否，直接切                                         |
| 多 channel 并发             | 上限 4 / target，阶段 4 视情况调                    |
| known_hosts 来源            | 自管理，与系统 ssh 隔离                             |
| key passphrase              | 从 cred store 读，不弹 UI                           |
| 文件读取                    | 阶段 3 仍用 bash cat 批读；SFTP 留阶段 5            |
| 并发对远端压力              | 给 SshClientPool 加单 target 4 channel 上限         |
| Drop 行为                   | ConnectedSshTarget Drop 异步 disconnect（spawn）    |

## 风险

| 风险                                           | 缓解                                            |
|------------------------------------------------|------------------------------------------------|
| russh 与某些远端服务器（旧 OpenSSH）兼容性       | 选 algorithm preferences 与 OpenSSH 一致         |
| 私钥格式（OpenSSH new format / PKCS#8 / RSA）   | russh-keys 支持主流格式，先用 sample 验证        |
| Windows 下私钥权限（NTFS ACL）                  | russh-keys 不像 ssh.exe 严格校验权限             |
| 长连接突然断（NAT 超时、远端重启）              | 检测到 Disconnect 自动 invalidate pool 重连     |
| 多并发 channel 把远端打挂                      | Semaphore 上限                                  |
| russh API 版本变动                             | 锁版本 0.45.x，CHANGELOG 标注                    |
| `Drop` 异步 disconnect 漏掉                    | 提供显式 close + 退出 hook 兜底                  |

## 测试

| 类型     | 用例                                              |
|----------|--------------------------------------------------|
| 单测     | mock russh handle，验证 run_command 调用 channel  |
| 集成     | docker openssh 容器，跑 100 次 run_command        |
| 集成     | 100 次连续 connect，无 fd 泄漏（lsof）            |
| 集成     | known_hosts 不匹配场景：拒绝连接                   |
| 集成     | 主动 cancel 一个长跑命令，确认 channel close       |
| 端到端   | 真实 dckj，扫描时间 ≤ 3s（家宽）                   |
| 端到端   | 真实 dckj，30 次连续切 target，无残留              |

## 文件改动清单

```text
src-tauri/Cargo.toml                              +依赖
src-tauri/src/targets/exec.rs                     重写大半
src-tauri/src/targets/client_pool.rs              +200 行（新）
src-tauri/src/targets/known_hosts.rs              +150 行（新）
src-tauri/src/targets/askpass.rs                  删除
src-tauri/src/targets/mod.rs                      调整 include!
src-tauri/src/lib.rs                              AppState 增 client_pool
src-tauri/src/main.rs                             退出 hook 接 close_all
src-tauri/src/services/installation/remote.rs     接口适配
src-tauri/src/commands/linker.rs                  接口适配
src-tauri/src/services/central_skills/*.rs        接口适配
src-tauri/src/services/github_import/remote.rs    接口适配
src-tauri/src/targets/tests.rs                    新增 mock 测试
+ 集成测试 docker compose 文件
```

## 估时

3-5 天工作日。重点在测试覆盖与 known_hosts 处理。

## 验收

- 真实 dckj 扫描时间 ≤ 3s
- 100 次连续 run_command，TCP 握手只发生 1 次（用 wireshark/tcpdump 抽检）
- known_hosts 不匹配时 UI 显式提示而非静默失败
- 删除 askpass.rs 后回归全绿
- 内存：100 次扫描后驻留进程内存 ≤ 200MB（无明显泄漏）
