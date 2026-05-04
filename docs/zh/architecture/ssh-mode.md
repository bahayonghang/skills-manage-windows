# SSH 模式

`src-tauri/src/targets/` 让 SkillPort 用一致的方式驱动远端技能仓库。命令始终走 registry，不在调用点判断 `remote` 标志。

## 目标（Target）

一个目标行编码 SkillPort 如何到达技能仓库：

| 字段 | 说明 |
| --- | --- |
| `id` | UUID，重命名后稳定 |
| `kind` | `local` 或 `ssh` |
| `host` / `port` / `user` | 仅 SSH |
| `private_key_path` | 可选；缺省回退 ssh-agent |
| `password_ref` | 指向加密凭据存储 |
| `remote_root` | 主机上的 `~/.skillsmanage` 默认值 |

未配置 SSH 目标时，`local` 是隐式 fallback。

## 模块

```text
targets/
├── model.rs        持久化目标行
├── registry.rs     活动目标 + 远端 DbPool 缓存
├── exec.rs         exec_local / exec_ssh，统一 stdin / stdout / stderr
├── cred.rs         加密 + 持久化密码
├── askpass.rs      ssh 通过 SSH_ASKPASS 喂密码
├── commands.rs     IPC 命令（重导出为 commands::targets）
└── tests.rs        本地 + mocked SSH 双端测试
```

## 活动目标解析

```text
[任意命令] ──► AppState::active_target()
                          │
                          ├─ Local → 返回本地 DbPool + 本地 exec
                          └─ SSH   → 打开 / 复用 sqlite 池
                                       远端 ~/.skillsmanage/db.sqlite
```

Registry 缓存连接，连续调用不必重复 SSH 握手。`set_active_target` 切换时缓存失效。

## Exec 契约

所有 shell 级动作（`ssh-keyscan` 校验、远端 `mkdir`、安装回退等）都走 `targets::exec::run_command`：

- 本地：`std::process::Command`。
- SSH：`ssh user@host -- 'cmd'`，密码通过 `SSH_ASKPASS`（`askpass.rs`）注入。

服务不直接 `Command::new("ssh")`。这让 SSH 链路可测，以后换传输也不影响业务代码。

## 远端安装

`services::installation::remote.rs` 是 `native.rs` 的 SSH 版：

1. 解析远端 agent 的 `global_skills_dir`。
2. 通过 exec 执行 `mkdir -p` 与 `ln -s`（symlink 失败回退 `cp -r`）。
3. 更新远端 SQLite 池里的 `skill_installations` 行。
4. 在本地 `operation_logs` 镜像一条记录，UI 能看到这次操作。

## 失败处理

- 连接失败以 `Result::Err(String)` 上抛，日志页用 `target_kind = 'ssh'` 标记。
- 长操作通过 registry emit 进度事件，UI 监听并刷新 SSH 横幅。
- 临时失败不会自动切回 local，必须用户显式切换。

## 测试策略

`targets/tests.rs` 走双端：

- 本地 exec 用临时目录直接断言文件系统副作用。
- SSH exec 替换 `PATH` 上的 ssh 二进制为 mock，断言参数形态而非真实网络行为。

Last reviewed: 2026-05-04
