# 技术设计 — Skills CLI 远端安装与更新

对应 `prd.md` 的 R1–R11。依赖 `08-27-skills-cli-remote-mutate` 已合入。
**本文件关闭 Q4**（见 §2.4）。

## 1. 现状结构

### 1.1 安装 argv 的固定形状

spec `skills-cli-global.md:77-80` 与 `argv.rs:286-290`：
程序是 `node`，`argv[1]` 是 npm 的 `npx-cli.js`，
前缀 `--yes --package=skills@1.5.23 -- skills`，add 层再加 `-g -y` 与至少一个 `-a`、`-s`。
禁止 `Command::new("npx.cmd")`、禁止 `cmd /c` 字符串拼接、禁止默认 `--all` / `--agent '*'`。

`remote-seam` 只解析了远端 node 程序，**没有**解析远端 `npx-cli.js`——
那是 doctor 不需要的（`doctor-gate` D1 把 npx 解析移到 spawn 路径）。
所以远端 launcher 的完整解析是本任务的作业面。

### 1.2 apply 不 spawn CLI（阶段名是历史遗留）

`updates/apply.rs:45-53` 定义九个 journal 阶段：
`prepared` / `backups_staged` / `cli_started` / `cli_succeeded` / `db_committed` /
`cleanup_pending` / `completed` / `rolled_back` / `recovery_required`。

但 apply 实际**不 spawn CLI**——它从 pinned GitHub 快照复制刷新 canonical
（`apply.rs:377-381` `refresh_canonicals`）。`cli_started` 是命名遗留。
远端沿用同一套阶段常量以保持 DB schema 兼容，不改名。

### 1.3 快照获取的现状

`ProductionSkillsCliGithub`（`updates/github.rs:52-103`）在**本机**用
`reqwest::Client` 调 `github_import::download_repo_snapshot`。
客户端在 `commands/skills_cli.rs:494-496` 构造。apply 在 `:208` 取 pinned 快照。

### 1.4 既有的远端取数先例——以及它为什么不能照抄

`github_import` 域已经实现了「让远端主机自己拉取」：

```206:212:src-tauri/src/services/github_import/remote.rs
pub(super) fn curl_auth_header_config_line(token: &str) -> Result<String, GithubImportError> {
    if token.contains('\n') || token.contains('\r') {
        return Err(GithubImportError::PatTokenHasNewline);
    }
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("header = \"Authorization: Bearer {escaped}\""))
}
```

这行 `Authorization: Bearer <token>` 被写进远端的 `$workspace/curl.conf`
（`remote.rs:117-122`，配 `umask 077`），再由远端 `curl -K` 读取。

它做对了一半：**token 不进 argv**（走配置文件而非命令行）。
但它**把 token 落到了远端磁盘**。

本任务的 R6 比它严格：「GitHub 凭据都不得写入远端主机，也不得进入远端命令行参数」。
所以这个先例只能作为反面参照记录，不能复用。

## 2. 目标结构

### 2.1 远端 launcher 与 argv（R1、R2）

`SkillsCliTransport` 增加远端 launcher 解析：找到远端 node 后，
在其同级目录按与本机 `npx_js_candidates` 相同的候选顺序探测 `npx-cli.js`，
**一次远端往返**完成（与 §2.5 的预检合并）。

远端命令的构造规则：

- 仍是 `<remote_node> <remote_npx_js> --yes --package=skills@<PIN> -- skills -g -y -a … -s …`。
  形状与本机逐字一致，只是路径是远端路径。
- 每个 argv 元素经 `shell_quote`（`exec.rs:703`，POSIX 单引号转义）后拼接，
  **不做**任何手工字符串插值。
- 禁止项在远端同样成立：不出现 `npx.cmd`、不出现 `cmd /c`、
  不出现默认 `--all` 或 `--agent '*'`。
- 来源白名单（拒绝 `&|^%!<>"'`、空格、`-c`，spec `:149`）在**发往远端之前**校验，
  不放宽。这一层在本机执行，远端拿到的已是通过校验的值。

远端 launcher 解析失败 → `SkillsCliError::CliUnavailable`
→ `skills_cli.cli_unavailable`，与本机同码同语义（`doctor-gate` 已把该码收敛为
「环境无法执行」，远端复用正合适）。

### 2.2 进程策略映射（R3）

| 操作 | 本机策略 | 远端等价 |
| --- | --- | --- |
| 来源预览 | Standard 120s | `run_command`（`ProcessPolicy::standard()`，120s） |
| add | BulkTransfer 15min | `run_script_cancellable`（`bulk_transfer()`，15min） |

`ConnectedRemoteTarget` 的策略是按方法固定的（`run_script` → standard、
`run_script_cancellable` → bulk_transfer），所以「选对方法」就是「选对策略」。
add 必须走 cancellable 变体——既为了 15min 上限，也为了 lease 的取消旗标能生效。

stderr 上限由 `ProcessPolicy` 的 `stderr_limit` 承担；
远端 stdout/stderr/URL 不进 `IpcError.message` 与未脱敏日志（R10），
失败时的结构化 warn 沿用 `doctor-gate` §2.5 的白名单做法。

### 2.3 远端无外网（R4）

远端 npx 拉不到 registry 时，CLI 以非零退出结束 → `SkillsCliError::CliFailed`
→ `skills_cli.cli_failed`（`doctor-gate` 新增的码）。

这条路径天然零写：add 失败时 CLI 自己不落盘，我们也没做任何 FS 变更
（lock 由 CLI 写，CLI 没成功就没写）。
需要断言的是**我们**没留半完成状态——即失败后不写 journal、不改 lock、不建链接。

### 2.4 Q4 关闭：本机拉取 + 经 SSH 下发（R6）

**决策：由 SkillPort 本机拉取 GitHub 快照，再经 SSH 流式下发到远端。**

这不是在两个都可行的方案里选一个，而是 R6 只留下了一条路：

| 路线 | 凭据是否离开本机 | 与 R6 |
| --- | --- | --- |
| 远端自取（照抄 §1.4 的 `curl.conf` 手法） | 是——token 落远端磁盘 | **违反** |
| 远端自取但不带凭据 | 否 | 仅公开仓库可用；私有仓库功能缺失，是相对本机 apply 的功能倒退 |
| **本机拉取 + 下发** | 否 | **满足** |

PRD 原本把 Q4 挂在「等 `remote-mutate` 有传输实测数据后再定」。
性能数据只在两条路线都被允许时才是决定因素；R6 已经排除了另一条，
所以不需要等实测就能关闭（TPR-01）。

传输机制：

- 只传**需要刷新的技能子集**，不传整个仓库快照。apply 本就只刷新 owned canonical
  文件（`apply.rs:377-381`），要传的范围本机已知。
- 用 `ConnectedRemoteTarget::run_command_with_stdin_bytes_cancellable`（`remote.rs:125`）
  把 tar 流管进远端 `tar -x`，避免先落远端磁盘再解压。
- **实施时必须确认该方法的 `ProcessPolicy`**：若它不是 `bulk_transfer()`，
  需改用带 bulk 策略的路径，否则大快照会撞 120s（implement 段 4 的显式检查项）。
- 落地目录是远端的 staging 区，成功后再按 journal 阶段换入，失败即丢弃。

凭据边界的可测形式：token 只存在于本机 `reqwest` 的请求头中。
AC6 断言它不出现在任何远端命令 argv、远端环境变量、远端落盘内容里。

### 2.5 锁顺序与 journal（R7）

顺序：`skills_cli` lease → **网络准备** → 远端 target guard → guard 下 recheck → journal。

网络准备（本机拉快照）**排在 guard 之前**，与本机 apply 一致
（`apply.rs` 在 `:239-245` 才取 guard，`:208` 已取快照）。
理由：网络耗时不确定，不该占着互斥锁。

journal 阶段沿用 §1.2 的九个常量，落 SQLite。
远端场景下 `backups_staged` 对应"远端 canonical 已备份"，
`cleanup_pending` 对应"远端 staging 待清理"。
中断后 recovery 由 `updates/apply/recover.rs` 驱动，重连远端续做。

### 2.6 更新检测与 fail-closed（R5）

检测路径（GitHub SHA / snapshot pinning）**完全在本机执行**，与 target 无关——
它只关心「上游有没有新版本」。因此远端检测复用现有实现，不需要远端往返。

需要远端的是「本地是否被修改」「拓扑是否冲突」这两个判定，
它们基于远端 canonical 的摘要与 placement 分类：

- `update_local_modified`：比对远端 canonical 摘要与 baseline。
  摘要计算在远端执行（一次脚本，参照 `central_updates/fs.rs:384` 的 `REMOTE_HASH_SCRIPT`），
  比对在本机。
- `update_topology_conflict`：direct_copy 与 conflict 拓扑 fail-closed，
  判据来自 `remote-inventory` 的分类结果。
- `verified_unsupported` / `unverified`：pinned 全 SHA add/update 与 direct-copy 刷新
  保持 fail-closed，远端不放宽。

### 2.7 `install_origin` 在远端 fail-closed（R8）

`classify_local_path_origin`（spec `:134-135`）依赖本机路径语义，只在 Local 生效。
远端**不实现等价语义**，在能力矩阵中显式记为「远端不支持」，
返回的 placement 中 `install_origin` 为 `None`。

选 fail-closed 而非猜测：`install_origin` 用于区分「这个副本从哪来」，
猜错会误导用户对删除后果的判断。缺失是安全的，猜错不是。

### 2.8 进度事件（R9）

`UPDATE_PROGRESS_EVENT`（`updates/mod.rs:32`）由
`commands/skills_cli.rs:483-486` 的 `AppUpdateProgress` 发出，
服务侧在 `apply.rs:217-228`（prepare）与 `:477-488`（completed）调用。

远端**不新增任何进度机制**——同一个 emitter、同一个事件名、同一批 phase 字符串。
前端 `skills-cli://update-progress` 的消费路径零改动。

## 3. 数据流

```
远端 add
  → ensure_capability(AddGlobal)
  → 来源白名单校验（本机）
  → lease → 远端 launcher 解析（1 次往返，与预检合并）
  → target guard
  → run_script_cancellable(远端 node + npx-cli.js + PIN + -g -y -a -s)
  → 非零退出 → cli_failed（零写）
  → 成功 → 刷新远端库存

远端 apply
  → ensure_capability(ApplyUpdates)
  → lease
  → 【网络准备，guard 之前】本机 reqwest 拉 pinned 快照 → 裁出需刷新子集 → 打 tar
  → target guard
  → guard 下 recheck（分类 + 远端摘要比对）
  → journal(prepared) → 远端备份 canonical → journal(backups_staged)
  → tar 流经 stdin 送入远端 `tar -x` 到 staging  → journal(cli_started)
  → 远端换入 → journal(db_committed) → 清 staging → journal(completed)
  → 任一步失败 → journal(recovery_required)，由 recover.rs 重连续做
  → 全程发 skills-cli://update-progress（沿用既有 emitter）
```

## 4. 契约与兼容性

- 命令签名不变，`skills_cli_apply_updates` 仍是**每请求一个 `repositoryKey`**
  （`generatedCommandMap.ts:989-993`）。跨仓库分组串行由前端负责，本任务不改。
- 不新增 IPC 错误码：`cli_unavailable`（launcher/spawn）、`cli_failed`（非零退出）、
  `timeout`、`update_local_modified`、`update_topology_conflict`、`recovery_required`
  全部已存在。
- `install_origin` 在远端返回 `None`——**这是可观察的行为差异**，
  spec 能力矩阵需明确记一行，避免被当成 bug。
- journal 阶段常量与 DB schema 不变。
- 若实施中发现必须改命令签名（例如 apply 需要携带 target 信息），
  则触发 `pnpm docs:gen` 与 `ipc_registry` 日志策略同步（R11）。当前设计不需要。

## 5. 权衡

- **本机拉取 + 下发的代价是传输**：私有仓库快照要走两跳（GitHub→本机→远端）。
  换来的是凭据边界干净且不依赖远端有外网访问 GitHub 的能力。
  对「远端能连 npm 但连不上 GitHub」的环境反而更稳。
- **add 仍要求远端能连 npm registry**：这条无法回避——
  `skills add` 的语义就是让 CLI 在目标机上装。R4 只保证失败时干净。
- **`install_origin` 缺失**：远端用户看不到来源标注。
  相比猜错，这是可接受的信息缺失。
- **检测在本机、摘要在远端**：多一次远端往返，但避免把 GitHub 访问搬到远端。

## 6. 回滚点

| 单元 | 内容 | 可否单独回滚 |
| --- | --- | --- |
| A | 远端 launcher 解析 + 远端 argv 构造 | 可 |
| B | 远端 add（预览 + 安装） | 依赖 A |
| C | 远端更新检测（远端摘要 + fail-closed 判定） | 可 |
| D | 远端 apply（快照下发 + journal） | 依赖 C |
| E | 能力矩阵翻闸 + spec 修订 | 与 B、D 同批 |

B 与 D 相互独立：可以只交付远端安装而暂不开放远端更新，
此时能力矩阵只翻 `PreviewSource` / `AddGlobal` 两行，状态自洽。
