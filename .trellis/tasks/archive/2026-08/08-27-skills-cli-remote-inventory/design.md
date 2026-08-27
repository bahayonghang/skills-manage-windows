# 技术设计 — Skills CLI 远端只读列举

对应 `prd.md` 的 R1–R9。依赖 `08-27-skills-cli-remote-seam` 已合入。

## 1. 现状结构

### 1.1 列举是纯 FS 读，不 spawn CLI

`list_global`（`mod.rs:360-368`）读 lock v3 + 文件系统投影，全程不调 CLI。
远端必须保持同一性质（R1）——这也是它能先于 install/update 交付的原因。

### 1.2 分类的 syscall 分布（远端化的核心难点）

```40:46:src-tauri/src/services/skills_cli/placement.rs
pub(crate) fn classify_one(
    skill_name: &str,
    canonical: &Path,
    platform: &PlacementPlatform,
) -> SkillsCliPlacement {
```

`classify_placements`（`:27-38`）对**每个技能 × 每个平台**调一次 `classify_one`，
后者对 slot 路径调 `observe_directory_slot`（`installation/directory_link.rs:58-80`）。
`observe_directory_slot` 的 syscall 链：

| 情形 | syscall |
| --- | --- |
| 所有情形 | `std::fs::symlink_metadata(link)` |
| Unix 链接 | 追加 `read_link(link)`，再 `resolved.exists()` |
| Windows 链接/junction | 追加 `CreateFileW` + `DeviceIoControl(FSCTL_GET_REPARSE_POINT)`（`directory_link.rs:334-348`），再 `resolved.exists()` |
| 普通目录 / 冲突文件 | 只用已取到的 metadata |

另有 `canonical_is_owned_directory`（`placement.rs:112-117`）对每个技能的 canonical
调一次 `symlink_metadata`。

本机这些都是廉价 syscall。**远端逐次执行会变成 N×M 次 SSH 握手**，
每次握手连接超时 10s（`exec.rs:7`）。30 技能 × 4 平台 = 120 次握手，不可用。

### 1.3 仓库里已有的批量脚本先例

| 位置 | 手法 | 分块 |
| --- | --- | --- |
| `scanner/ssh_batch.rs:34-71` `build_probe_script` | 多根目录探测合成一个脚本 | 路径走 **argv** |
| `scanner/ssh_batch.rs:106-125` `build_batch_read_script` | 批量读 SKILL.md | argv，块大小 4 |
| `usage/fs_backend.rs:294-309` `build_batch_read_script` | 批量读文件，`PATH_MARKER`/`EOF_MARKER` 协议 | argv，块大小 64 |
| `central_updates/fs.rs:384` `REMOTE_HASH_SCRIPT` | 多根哈希，`$@` | argv |

**它们全部用 argv 传路径，因此都受 `ARG_MAX` 约束而必须分块** ——
即往返次数是 `O(N/K)`，不是常数。

### 1.4 前端闸门

`SkillsCliView.tsx:210-217`（非 Local 渲染 `skillsCli.localOnly` 占位）、
`:111-116`（非 Local 不 `loadAll()`）、`Sidebar.tsx:113-114`（非 Local 隐藏入口）。
切 target 时 `AppShell.tsx:91-127` 已会重置 store 并触发重扫，可直接复用。

## 2. 目标结构

### 2.1 常数级往返：路径清单内嵌进脚本体，不走 argv（R2）

这是本任务与 §1.3 所有先例的关键差异。

`ConnectedRemoteTarget::run_script(script, args)` 把 **script 从 stdin 送入** `sh -s --`
（`exec.rs:682-688`），args 才走命令行。既然脚本体本身不受 `ARG_MAX` 限制，
就把路径清单以 heredoc 形式内嵌进脚本体，让远端 `while read` 遍历：

```sh
while IFS= read -r p; do
  if [ -L "$p" ]; then k=link; t=$(readlink "$p" 2>/dev/null || true)
  elif [ -d "$p" ]; then k=dir; t=
  elif [ -e "$p" ]; then k=file; t=
  else k=absent; t=; fi
  printf '%s\t%s\t%s\n' "$p" "$k" "$t"
done <<'SKILLPORT_PATHS'
…每行一个路径…
SKILLPORT_PATHS
```

于是往返次数**与 N、M 完全无关**。分块常量不存在，也就不需要 `ceil(N/K)` 那套。

**每次列举固定 2 次远端往返**，不能压成 1：

| # | 内容 | 为何不能与另一次合并 |
| --- | --- | --- |
| RT1 | 读远端 lock 文件 | 技能名单来自 lock，未读到之前算不出要探测哪些 slot 路径 |
| RT2 | 一次脚本探测「全部 canonical + 全部 slot」 | 依赖 RT1 的结果 |

不把 lock 解析放进 shell（用 awk 抠 JSON 太脆），所以接受 2 次而不是 1 次。

路径清单的构成：每个技能的 canonical 目录，加上「技能 × 平台」的全部 slot 路径。
清单在 Rust 侧生成，顺序固定，输出按路径回填，缺行视为 `absent`。

### 2.2 原始信号与分类分离（R3、AC4）

脚本**只输出原始信号**（`link` / `dir` / `file` / `absent` + `readlink` 目标），
五态与 `reason_code` 的判定全部留在 Rust 侧的共享分类函数里。

这样做的三个理由：

1. 分类规则只有一份，本机与远端共用，AC3「与本机同输入逐字段一致」才可能成立。
2. `remote_os` 只有 Rust 侧知道，Windows junction 的判定需要它。
3. shell 里写业务判定无法单元测试。

重构 `classify_one`：从「自己做 syscall」改为「接受一个已观测的 slot 状态」：

```rust
// 已有的本机观测结果，与远端观测结果统一成同一个输入类型
pub(crate) enum ObservedSlot {
    Absent,
    ManagedLink { kind: SkillsCliManagedLinkKind, resolves_to_canonical: bool },
    PlainDirectory,
    Conflict,
}

pub(crate) fn classify_one_observed(
    canonical_owned: bool,
    slot: ObservedSlot,
    platform: &PlacementPlatform,
) -> SkillsCliPlacement
```

本机调用方先跑 `observe_directory_slot` 得到 `ObservedSlot`，远端调用方从脚本输出构造。
`classify_absent`（`:73-110`）的四个 `reason_code` 判定顺序**原样保留**，一个字不改。

### 2.3 远端 Windows junction 的诚实边界（AC4）

远端命令一律经 `sh`（SSH 走登录 shell，WSL 走 `sh -lc`，见 `exec.rs:524-526`）。
远端主机若是 Windows，`sh` 来自 Git Bash / MSYS 一类兼容层，
**junction 是否被 `test -L` 识别为链接，取决于该兼容层而非我们的代码**。

因此：

- 脚本输出的是原始信号，Rust 侧按 `remote_os()`（`remote.rs:57`）决定如何解释。
- Unix 远端：`link` + `readlink` 指向 canonical → `ManagedLink { Symlink }`。
- Windows 远端：`link` 同上按 junction 处理；
  若信号是 `dir` 但该路径同时出现在 lock 的 managed link 记录中，
  **不**猜测它是 junction，按 `PlainDirectory` 处理 → 分类为 `direct_copy`。
- 选 `direct_copy` 作为歧义时的落点是 fail-safe：`direct_copy` 永远不会被自动转换
  （父任务 Out of Scope），也永远不会被删除（spec `:99-102`），
  最坏结果是少显示一个 managed link，而不是误删。

自动化测试用 fake 远端观测结果覆盖两种 OS 分支；
**真实远端 Windows 主机的 junction 行为标记 `UNVERIFIED`**，与 RAC7 一致。

### 2.4 远端平台探测（R4）

`enabled` 与 `detected` 是两个不同来源，都必须取远端值：

| 维度 | 来源 | 为何不会误用本机值 |
| --- | --- | --- |
| 平台清单与 `enabled` | `TargetContext::db()` | `resolve_active_context` 已按 target 解析 DB（`targets/registry.rs:420-427`），远端 target 拿到的就是远端 agent 行 |
| `detected`（目录是否存在） | RT2 脚本对每个平台 `global_skills_dir` 的探测结果 | 平台目录路径本身也进 §2.1 的清单 |

所以远端探测不需要新机制，只需要**不写**「用本机 detected 结果兜底」这样的代码。
AC4c 用「本机存在而远端不存在的平台」这一前置来把这条约束变成可判定的断言。

### 2.5 错误分轨与新错误码（R5、R7）

- 远端 lock 缺失或为空 → 空 `skills` 数组 + 携带 `canonicalRoot` / `lockPath`，**不是错误**
  （与本机一致，spec `skills-cli-global.md:81-94`）。
  `read_file_bounded` 返回 `RemotePathMissing` 时在此处吞掉转空。
- 远端 IO 失败 → `internal.unexpected`，**绝不**用 `skills_cli.cli_unavailable`
  （后者已被 `doctor-gate` 收敛为 write-path 环境错误）。
- 连接 / 认证 / 超时需要可区分（R7）。`TargetsError` 到 `SkillsCliError` 的映射：

| `TargetsError` | `SkillsCliError` | IPC 码 |
| --- | --- | --- |
| `ProcessTimedOut` | `Timeout` | `skills_cli.timeout`（已有） |
| `ProcessCancelled` | `Cancelled` | `skills_cli.cancelled`（已有） |
| `RemoteCommandFailed` / `WslCommandFailed`（连接、认证失败） | **新增** `RemoteUnavailable` | **新增** `skills_cli.remote_unavailable` |
| 其余（`RemoteFileTooLarge`、`RemoteInspectFailed` 等） | `Io` | `internal.unexpected` |

新增一个 IPC 码是有代价的（见 §4），但把「SSH 连不上」显示成「内部错误」会让
R7 要求的重试语义无从表达。

库存的 stale-while-revalidate 沿用现有 `inventoryError` 分轨（store 已实现），
不新增状态。

### 2.6 前端解闸（R6）

| 位置 | 现状 | 目标 |
| --- | --- | --- |
| `Sidebar.tsx:113-114` | 非 Local 隐藏入口 | 始终显示 |
| `SkillsCliView.tsx:210-217` | 非 Local 渲染 `localOnly` 占位 | 删除占位分支 |
| `SkillsCliView.tsx:111-116` | 非 Local 不 `loadAll()` | 无条件 `loadAll()` |
| 写操作入口 | 无（此前整页不渲染） | 按能力矩阵禁用，并显示**本地化原因**而非静默 disabled |

「本地化原因」需要前端知道哪些能力在当前 target 未开闸。
不新增 IPC：`ensure_capability` 失败返回的 `skills_cli.local_target_only` 已有公开句，
前端在按钮 tooltip 上复用该句即可。写操作按钮的禁用条件为
「目标非 Local 且该能力尚未开闸」——这份清单在 `remote-mutate` 与
`remote-install-update` 合入时逐步缩短，最终为空。

## 3. 数据流

```
loadAll() → skills_cli_list_global
  → ensure_capability(ListGlobal)                       未开闸 → local_target_only
  → RT1: tx.fs().read_file_bounded(lock_path)           缺失/空 → 空快照（非错误）
  → 解析 lock v3 → 技能名单
  → 生成路径清单（canonical × N + slot × N × M）
  → RT2: tx.fs().probe_paths(清单)                       一次 run_script，内嵌 heredoc
  → 逐条构造 ObservedSlot
  → classify_one_observed（与本机共用）                   → 五态 + reason_code
  → SkillsCliGlobalSnapshot                              形状与本机完全一致
```

前端不区分来源——这是 R1 的验收面。

## 4. 契约与兼容性

- `SkillsCliGlobalSnapshot` 形状不变，命令签名不变。
- **新增一个 IPC 错误码** `skills_cli.remote_unavailable` → 需要
  `ipc_error.rs` 公开句 + en/zh i18n 键 + `pnpm ipc:codegen` 刷新
  `generatedCommandMap.ts` 的 reviewed codes 列表（生成物，不手改）。
  公开句不得含主机名、用户名、路径、stderr。
- `classify_one` 重构为 `classify_one_observed` 属内部契约变更，
  `placement.rs` 的既有测试需同步改造但**不得删除断言**。
- spec `skills-cli-global.md` 的能力矩阵翻 `ListGlobal` / `InstallTargets` /
  `ReadSkillMd` / `ExportInventory` 四行，与代码同批。

## 5. 权衡

- **2 次往返而非 1 次**：把 JSON lock 解析塞进 shell 能省一次握手，
  但 awk 解析 JSON 在字段顺序或转义变化时会静默出错。多一次 10s 上限的握手换正确性。
- **heredoc 内嵌路径 vs argv 分块**：内嵌让往返真正常数化，代价是脚本体随 N×M 增长。
  以 30 技能 × 6 平台 × 平均 80 字节路径估算约 15 KB，远低于 stdin 的实际限制。
  若未来出现超大库存，退路是回到 §1.3 的 argv 分块并把 R2 降级为 `O(N/K)`——
  但那需要改 PRD，不是实现时可自行决定的。
- **歧义落到 `direct_copy`**：牺牲远端 Windows 的显示精度换取「绝不误删」。

## 6. 回滚点

| 单元 | 内容 | 可否单独回滚 |
| --- | --- | --- |
| A | `classify_one` → `classify_one_observed` 重构（纯本机，行为不变） | 可 |
| B | 远端探测脚本 + 远端列举 + 错误映射 + 新错误码 | 依赖 A |
| C | 前端解闸（三处闸门 + 能力驱动的禁用原因） | 依赖 B |

回滚 C 会让页面退回「非 Local 显示占位」，后端能力仍在但无入口——状态自洽，可接受。
回滚 B 必须同时回滚 C，否则页面会对着一个未开闸的能力反复报错。
