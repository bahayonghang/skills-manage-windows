# 技术设计 — Skills CLI doctor 门禁降级与警告去噪

对应 `prd.md` 的 D1 / D2 与 R1–R8。

## 1. 现状结构

### 1.1 告警的实际渲染位置

用户截图中的红字并非独立横幅，而是 header 的运行时状态行：

```116:125:src/components/skillsCli/SkillsCliHeader.tsx
      <p
        className={cn(
          "mt-2 text-xs",
          runtimeError ? "text-destructive-text" : "text-muted-foreground",
        )}
        data-testid="skills-cli-doctor"
        aria-label={t("skillsCli.runtimeStatus")}
      >
        {runtimeLabel}
      </p>
```

`runtimeLabel`（`SkillsCliHeader.tsx:41-50`）在 `runtimeError` 非空时渲染
`formatBackendError(runtimeError, t)`，即公开句「无法执行 Skills CLI 软件包。」。
页面里没有第二个 `role="alert"` 渲染该句（`SkillsCliView.tsx` 的三个 alert 分别属于
update recovery、update cache、inventory）。

**结论**：不需要「删除一个横幅组件」，需要改变 `runtimeError` 的产生条件与消费面。

### 1.2 doctor 的两段职责与一处隐藏耦合

```300:349:src-tauri/src/services/skills_cli/mod.rs
pub(crate) async fn doctor(runner: &dyn SkillsCliRunner) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    doctor_with_launcher(runner, &resolve_launcher()?).await
}
// doctor_with_launcher: ① node --version 检测 ② skills --help 探测
```

隐藏耦合在 `resolve_launcher()`：

```221:248:src-tauri/src/services/skills_cli/argv.rs
pub(crate) fn resolve_node_launcher_from_dirs(search_dirs: &[PathBuf]) -> Result<NodeLauncher, SkillsCliError> {
    let program = find_node_in_paths(search_dirs).ok_or(SkillsCliError::NodeMissing)?...;
    // …
    match candidates.iter().find(|c| c.is_file()).cloned() {
        Some(npx_js) => Ok(NodeLauncher { program, npx_js }),
        None => { tracing::warn!(...); Err(SkillsCliError::CliUnavailable) }
    }
}
```

`NodeLauncher` 把「node 可执行文件」与「npx-cli.js」绑成一个不可分割的值。
只要 `npx-cli.js` 找不到，`doctor()` 在**探测之前**就返回 `cli_unavailable`。
因此仅删除探测无法达成 D1 —— 必须同时拆开这层耦合。

### 1.3 `runtimeBlocked` 的消费面

`SkillsCliView.tsx:151` 的 `runtimeBlocked` 流向五处：卡片 `isLoading`（`:524`）、
批量栏（`:555`）、卸载对话框（`:587,590-592`）、详情抽屉（`:614`）、
以及 header 内部的 `installDisabled`（`SkillsCliHeader.tsx:51`）。

按 spec `skills-cli-global.md:81-83,99-100,184`，前四处对应的操作都不 spawn CLI。

## 2. 目标结构

### 2.1 后端：拆分 launcher 解析

引入两级解析，`NodeLauncher` 的对外形状不变：

| 函数 | 职责 | 失败错误 | 使用方 |
| --- | --- | --- | --- |
| `resolve_node_program(search_dirs)` | 只找 node 可执行文件 | `NodeMissing` | doctor |
| `resolve_node_launcher_from_dirs(search_dirs)` | 在前者基础上再找 `npx-cli.js` | `NodeMissing` / `CliUnavailable` | add、preview |

`resolve_node_launcher_from_dirs` 改为调用 `resolve_node_program` 再补 `npx_js`，
保持既有签名与错误语义，现有 add/preview 调用点无需改动。

### 2.2 后端：doctor 只做 Node 检测

```rust
// 目标形态
pub(crate) async fn doctor(runner: &dyn SkillsCliRunner) -> Result<SkillsCliDoctorReport, SkillsCliError> {
    doctor_with_program(runner, &resolve_node_program_from_env()?).await
}
```

`doctor_with_program` 保留 `node --version` 检测与 `>= 22.20` 判定，
删除 `build_probe_argv` 调用与随后的 `CliUnavailable` 返回。
它还要把 runner 返回的 `CliUnavailable` 重映射为 `NodeMissing`——理由见 §2.4.1。
`SkillsCliDoctorReport { node_version, npm_spec }` 形状不变——`npm_spec` 仍返回 PIN 常量，
它是展示用的声明值，不代表「已验证可执行」。

`build_probe_argv` 本身保留在 `argv.rs`（不再被 doctor 调用），
因为远端子树可能复用同一 argv 构造；若最终无人使用，由 `remote-seam` 任务决定删除。

### 2.3 后端：修正安装/预览失败的错误映射

这是 D1 的必要配套。移除探测后，「包跑不起来」只能在实际 `add` 时暴露，
而 spec `skills-cli-global.md:156` 当前把 add/preview 的非零退出映射到 `internal.unexpected`。

现状的精确形态是**两个不同变体被折叠成同一个通用码**：

```64:66:src-tauri/src/services/skills_cli/error.rs
    /// The CLI ran but exited with a failure status for the request.
    #[error("The Skills CLI command failed")]
    CliFailed,
```

```169:171:src-tauri/src/services/skills_cli/error.rs
            Self::OutputLimitExceeded { .. } | Self::CliFailed | Self::ListUnparsed => {
                "internal.unexpected"
            }
```

`add_global` 在 CLI 非零退出时返回 `CliFailed`（`mod.rs:573`），
与「环境装不起来」的 `CliUnavailable` 语义完全不同：前者是 CLI 跑起来了但这次请求失败
（例如仓库里没有该技能名、来源不可达），后者是包根本无法执行。

因此**不能**把 `CliFailed` 并进 `cli_unavailable`——那会对着一个「仓库里没这个技能」的失败
说「无法执行 Skills CLI 软件包」。正确做法是给它自己的码：

| 失败性质 | 变体 | 错误码 | 现状？ | 理由 |
| --- | --- | --- | --- | --- |
| `npx-cli.js` 无法解析 | `CliUnavailable` | `skills_cli.cli_unavailable` | 已是 | 环境缺失，与 launcher 语义一致 |
| 子进程无法 spawn | `CliUnavailable` | `skills_cli.cli_unavailable` | **否，需改** | 见下方「spawn 分支的缺口」 |
| CLI 非零退出（add） | `CliFailed` | **`skills_cli.cli_failed`（新增）** | 否，需改 | CLI 已执行但请求失败，区别于环境问题 |
| CLI 非零退出（preview） | `PreviewUnparsed` | `skills_cli.preview_unparsed` | 已是 | 保持现状 |
| `ls` 解析失败、输出上限、lock / FS IO | `ListUnparsed` / `OutputLimitExceeded` / `Io` | `internal.unexpected` | 已是 | 保持现状 |

#### spawn 分支的缺口（TPR-03）

上表「子进程无法 spawn」一行**不是**当前行为。`map_runner_error` 把进程启动失败
折叠进了通用 IO 变体：

```56:65:src-tauri/src/services/skills_cli/runner.rs
fn map_runner_error(error: crate::targets::RunnerError) -> SkillsCliError {
    use crate::targets::RunnerError;
    match error {
        RunnerError::Io {
            phase: crate::targets::RunnerPhase::Start,
            source,
        } => SkillsCliError::Io {
            context: "spawn Skills CLI process",
            source,
        },
```

而 `SkillsCliError::Io` 对外是 `internal.unexpected`（`error.rs:172`）。
移除 doctor 探测后，「node 在但 CLI 起不来」的唯一暴露点就是这条 spawn 路径，
若不改，用户看到的是通用内部错误，AC6 的 spawn 分支必然失败。

**改动**：`RunnerPhase::Start` 分支改为返回 `SkillsCliError::CliUnavailable`。
判据是 phase 而非 errno——启动阶段的 IO 失败在语义上就是「这个 CLI 在本机跑不起来」，
与 launcher 解析失败同类。`RunnerPhase` 的其余取值（supervise 阶段）保持
`SkillsCliError::Io` → `internal.unexpected` 不变，因为那是进程已经起来之后的监督失败。

丢弃 `source` 会损失一点诊断力，因此在转换点补一条 `tracing::warn!`，
字段规则见 §2.5。

新增码的公开句需同时落在 `ipc_error.rs::public_message_for_code`、
`src/i18n/locales/en.json` 与 `zh.json` 的 `backendErrors.skills_cli` 下，
以及 `generatedCommandMap.ts` 的 reviewed codes 列表（经 `pnpm ipc:codegen` 生成，不手改）。

`cli_unavailable` 因此收敛为**只表示环境无法执行**，与
`.trellis/spec/backend/domain-error-enums.md` 记录的「cli_unavailable is write-path only」一致。

### 2.4 前端：收敛 `runtimeError` 的消费面

`runtimeError` 保留在 store（分轨不变），语义收窄为「**Node 运行时状态未确认**」——
注意这不等于「一定是 `node_missing`」，见下方 §2.4.1。

| 消费点 | 现状 | 目标 |
| --- | --- | --- |
| `SkillsCliHeader` 状态行 | `runtimeError` 时红字显示公开句 | 保留，但公开句来自多个码而非单一 `node_missing` |
| `SkillsCliHeader` `installDisabled` | `!installAvailable \|\| runtimeError !== null` | 不变，理由改为 fail-closed（见 §2.4.1） |
| 卡片 `isLoading` | `isMutating \|\| runtimeBlocked` | 改为 `isMutating` |
| 批量栏 `runtimeBlocked` | 参与 `mutationsLocked` | 移除该 prop |
| 卸载对话框 `runtimeBlocked` | 拒绝打开 | 移除该 prop |
| 详情抽屉 `runtimeBlocked` | 禁用 link/unlink | 移除该 prop |

`SkillsCliView.tsx:151` 的 `runtimeBlocked` 变量随之删除。
安装动作的失败在 `skillsCliStore` 的 `actionError` + `skillsCliActionToast` 上呈现，
链路已存在，无需新增。

#### 2.4.1 `runtimeError` 是多码集合（TPR-05）

「移除探测后 `runtimeError` 只可能是 `node_missing`」是错误推论。
store 对 **任何** doctor rejection 都写 `runtimeError`：

```272:277:src/stores/skillsCliStore.ts
    if (runtime.status === "fulfilled") {
      patch.doctor = runtime.value;
    } else {
      patch.doctor = null;
      patch.runtimeError = backendErrorStateValue(runtime.reason);
    }
```

而 doctor 仍要 spawn `node --version`，runner 保留了四类非业务失败
（`runner.rs:66-77`）：`Timeout`、`Cancelled`、`OutputLimitExceeded`、`Io`。
因此移除探测后 `runtimeError` 的码集合是：

| 码 | 来源 | 性质 |
| --- | --- | --- |
| `skills_cli.node_missing` | 找不到 node、版本 < 22.20、node 无法启动（见下） | 确定性，可操作 |
| `skills_cli.timeout` | `node --version` 超过 Standard 120s | 瞬时 |
| `skills_cli.cancelled` | 请求被取消 | 瞬时 |
| `internal.unexpected` | supervise 阶段 IO、输出超限 | 未知 |

**与 §2.3 spawn 改动的交互**：§2.3 把 Start-phase spawn 失败映射为 `CliUnavailable`。
doctor 也走同一个 runner，若不处理，「node 文件在但起不来」会让 header 重新显示
「无法执行 Skills CLI 软件包」——正是本任务要消灭的那句话。
且 `domain-error-enums.md` 记录 `cli_unavailable` 是 write-path only。

因此 `doctor_with_program` 必须把 node 版本探测返回的 `CliUnavailable`
**就地重映射为 `NodeMissing`**：node 二进制存在却无法执行，语义上就是 Node 不可用。
这样 `cli_unavailable` 不会出现在 doctor 路径，与 §2.3 的收敛一致。

**Header / Install 策略：fail-closed，不新增机制。**
`installDisabled = !installAvailable || runtimeError !== null` 保持原样，
但依据不再是「一定是 node_missing」，而是「doctor 未成功 ⇒ 无法确认 Node ≥ 22.20 ⇒
不让用户发起一定会失败或行为不明的安装」。
瞬时失败的重试路径复用 header 已有的 Refresh 按钮（`onRefresh` → `loadAll()` → 重跑 doctor），
不新增重试控件。

**这不改变 `SkillsCliHeader.tsx` 的代码**，但它现在是一个被论证过的决定，
而不是「只可能是 node_missing 所以不用管」。对应地需要补瞬时码的用例（见 implement 段 5）。

### 2.5 后端：失败路径的结构化 warn（TPR-04）

R5 要求 add 失败时保留可诊断的 `tracing warn`。当前 add 路径在非零退出时直接返回，
没有任何日志：

```572:574:src-tauri/src/services/skills_cli/mod.rs
    if !output.status_success {
        return Err(SkillsCliError::CliFailed);
    }
```

现有 warn 只有 launcher 找不到 `npx-cli.js` 一处（`argv.rs:241-244`），
以及将被 D1 删除的 doctor probe warn。所以 R5 需要一条新的生产日志，不是「保留」。

**关键约束**：`redaction-policy` 与 spec `skills-cli-global.md:109-111` 禁止 stderr、
路径、URL 进入日志。因此「截断摘要」**不能**是 stderr 的节选——那样无论截多短都是泄露。
摘要改为由我们自己控制的结构化信号构成。

允许字段（白名单，超出者一律不记）：

| 字段 | 类型 | 来源 | 为何安全 |
| --- | --- | --- | --- |
| `operation` | `&'static str` | 编译期常量，如 `"skills_cli.add_global"` | 静态字面量 |
| `exit_code` | `Option<i32>` | `CliOutput.exit_code`（新增字段，见下） | 数值 |
| `stderr_bytes` | `usize` | `output.stderr.len()` | 只记长度，不记内容 |
| `stdout_bytes` | `usize` | `output.stdout.len()` | 同上 |
| `skill_count` | `usize` | `skill_names.len()` | 计数 |
| `agent_count` | `usize` | `cli_agents.len()` | 计数 |
| `source_kind` | `&'static str` | 由 `SkillSource` 变体映射的静态分类 | 不含原始 source 字符串/URL |

明确禁止：stderr / stdout 的任何字节、原始 source 字符串、任何文件系统路径、完整 argv。
由于没有任何字段派生自子进程输出，**不需要截断上限**——这正是能同时满足
「warn 存在」与「哨兵不出现」两个断言的原因。

**`CliOutput` 需要一个新字段**。当前只有 `status_success: bool`，拿不到退出码：

```31:38:src-tauri/src/services/skills_cli/runner.rs
pub(crate) struct CliOutput {
    pub status_success: bool,
    pub stdout: Vec<u8>,
    /// Captured for diagnostics; production paths drop it after parsing.
    /// Parsed only by tests asserting cap behavior.
    #[allow(dead_code)]
    pub stderr: Vec<u8>,
}
```

新增 `pub exit_code: Option<i32>`，在唯一构造点 `CliOutput::from_std`（`runner.rs:42-48`）
由 `output.status.code()` 填充。`FakeCliRunner` 的构造 helper 同步补默认值。

spawn 分支（§2.3）也需要一条 warn，但那里没有 operation 上下文，
字段收窄为 `phase = "start"` 与 `io_kind = source.kind()`。
只记 `ErrorKind` 枚举而不记 `source` 的 Display，因为后者在某些平台会带上程序路径。

## 3. 数据流

```
doctor()  → resolve_node_program（只找 node）→ spawn `node --version` → >= 22.20?
   ├─ 成功 → SkillsCliDoctorReport → header 显示 doctorOk
   └─ 失败 → node_missing | timeout | cancelled | internal.unexpected
             （spawn 失败在此处重映射为 node_missing，见 §2.4.1）
             → runtimeError → header 红字 + Install 禁用（fail-closed）+ Refresh 可重试
                            └─ 不再影响 list / link / unlink / remove / export

add / preview → resolve_node_launcher（含 npx-cli.js）→ spawn → 退出码
   ├─ launcher 解析失败 / spawn 失败 → cli_unavailable ┐
   └─ 非零退出                        → cli_failed     ┴→ actionError → toast
                                        （均不进 header，不禁用其他操作）
```

## 4. 契约与兼容性

- `SkillsCliDoctorReport` 的 IPC 形状不变 → 无需 `pnpm docs:gen`（除非命令签名变动）。
- `skills_cli_doctor` 命令签名不变 → `ipc_registry` 日志策略不变。
- `SkillsCliError` 不新增变体（`CliFailed` 已存在），但**新增一个 IPC 错误码**
  `skills_cli.cli_failed` → 需要 `ipc_error.rs` 公开句 + en/zh i18n 键 + `pnpm ipc:codegen`
  刷新 `generatedCommandMap.ts` 的 reviewed codes 列表。
- `cli_unavailable` 与 `node_missing` 的公开句不变。
- `build_probe_argv` 保留但无生产调用方，与既有的 `build_list_global_argv`
  （`skills ls -g --json`，仅测试使用、生产 list 从不调用）同属一类，有先例。
- `SkillsCliBatchBar` / `SkillsCliUninstallDialog` / `SkillsCliDetailDrawer` 删除一个 prop，
  属于内部组件契约变更，测试需同步。

## 5. 权衡

- **少一次子进程与一次网络往返**：页面首屏不再等待 `skills --help`（Standard 策略 120s 上限，
  网络不通时可能长时间挂起）。这是 D1 的直接收益。
- **代价：安装失败反馈更晚**。用户需点击 Install 并等待实际 `add` 才知道环境不可用。
  §2.3 的错误映射修正把这一代价控制在「错误信息仍可理解」的范围内。
- **不做**：为安装失败提供 stderr 细节。受 `redaction-policy` 约束，且 PRD 已列为 Out of Scope。

## 6. 回滚点

改动分三个回滚单元。**单元 A 内部不可再拆**（TPR-06）。

| 单元 | 内容 | implement 段 | 可否单独回滚 |
| --- | --- | --- | --- |
| **A** | launcher 拆分 + doctor 去探测（`argv.rs`、`mod.rs`） | 段 1 **+** 段 2，原子 | 只能整体回滚 |
| **B** | 错误映射修正 + 失败 warn（`runner.rs`、`error.rs`、`ipc_error.rs`、i18n、spec 矩阵） | 段 3 | 可 |
| **C** | 前端 `runtimeBlocked` 收敛（`SkillsCliView.tsx` + 三个组件 + 测试） | 段 4、5 | 可 |

**为什么 A 不可拆**：§1.2 已经证明 `NodeLauncher` 把 node 与 `npx-cli.js` 绑成一个值，
`resolve_launcher()` 在探测**之前**就会因 `npx-cli.js` 缺失返回 `cli_unavailable`。

- 只回滚段 1（launcher 拆分）而保留段 2：`doctor()` 会调用一个不存在的
  `resolve_node_program()`，编译失败。
- 只回滚段 2（去探测）而保留段 1：探测回来了，`cli_unavailable` 横幅回来了，D1 失效。

两种半回滚都不产生可用状态，所以段 1 与段 2 必须同批提交、同批回滚。

**B、C 的回滚语义**：

- 回滚 B：`cli_failed` 码与两条 warn 消失，add 失败退回 `internal.unexpected`。
  §2.4.1 在 `doctor_with_program` 里的 `CliUnavailable → NodeMissing` 重映射会变成惰性代码
  （spawn 失败重新走 `Io`），无害但应一并还原以免留下误导。
- 回滚 C：`runtimeBlocked` 过度封锁回归，但告警横幅仍不在（A 保留）。
  这是可接受的部分缓解。
- 回滚 A：等于放弃本任务目标，属完整 revert 而非部分缓解。
