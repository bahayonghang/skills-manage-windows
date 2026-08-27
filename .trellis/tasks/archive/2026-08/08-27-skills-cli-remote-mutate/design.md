# 技术设计 — Skills CLI 远端链接与安全卸载

对应 `prd.md` 的 R1–R10。依赖 `08-27-skills-cli-remote-inventory` 已合入。

## 1. 现状结构

### 1.1 本机链接创建是系统调用，没有远端等价物

```150:177:src-tauri/src/services/installation/directory_link.rs
pub(crate) fn create_skills_cli_directory_link(
    target: &Path,
    link: &Path,
) -> Result<ManagedDirectoryLinkKind, InstallationError> {
```

Windows 分支走 `CreateFileW` + `DeviceIoControl(FSCTL_SET_REPARSE_POINT)`
（`directory_link.rs:271-323`），Unix 分支走 `std::os::unix::fs::symlink`（`:179-201`）。
两者都是**本机进程内的系统调用**，远端无法直接使用。
调用方在 `link.rs:245`、`remove.rs:530`（recovery）。

### 1.2 既有远端删除路径不可复用（必须显式绕开）

```218:237:src-tauri/src/services/installation/transport.rs
    /// Remove an installed entry. Local classifies by recorded link type and
    /// refuses to delete unmanaged real directories; remote has always been
    /// an unconditional `rm -rf` of the install slot.
    pub(crate) async fn remove_install(
        &self,
        pool: &DbPool,
        agent: &db::Agent,
        skill_id: &str,
    ) -> Result<(), InstallationError> {
        match self {
            Self::Local => native::remove_install_local(pool, agent, skill_id).await,
            Self::Remote(connection) => {
                let install_path = crate::targets::remote_join(&agent.global_skills_dir, skill_id);
                connection
                    .remove_tree(&install_path)
                    .await
                    .map_err(transport_error)
            }
        }
    }
```

代码注释自陈：远端一直是**无条件 `rm -rf`**。这与 R3「远端绝不删除普通目录、
绝不对平台路径做递归删除」直接冲突。

**结论：Skills CLI 的远端删除不复用 `InstallTransport::remove_install`，
也不复用 `ConnectedRemoteTarget::remove_tree` 作为平台 slot 的删除手段。**
本任务自建一条经分类闸门的删除路径（§2.3）。这不是"稍加改造"能解决的——
`remove_install` 服务的是 installation 域的语义（DB 记录了 link_type），
Skills CLI 域的判据是 lock + placement 分类，两者不能共用。

### 1.3 卸载阶段模型的精确形态

`remove.rs:43-46` 只定义三个 manifest 阶段：
`prepared` → `staged` → `metadata_committed`。
**没有名为 `cleanup` 的阶段**——提交后的清理（删 canonical 备份目录、删 manifest 文件）
是 `remove.rs:392-399` 的收尾步骤，不写入 manifest。
PRD 中"四阶段"的表述按此校准：远端复用的是三个 manifest 阶段 + 一个收尾步骤。

流程（`execute_remove`，`remove.rs:299-414`）：

1. 建计划，遇 conflict 直接 bail（零写）
2. 读 lock → sha256 指纹 → 写 `prepared` manifest（`:343-344`）
3. canonical 改名为备份（`:346-354`）
4. 删 managed links（`:359-367`）
5. manifest → `staged`（`:368-370`）
6. **`remove_lock_row` 带指纹 CAS**（`:383-386`），不匹配 → `RecoveryRequired`
7. manifest → `metadata_committed`（`:387-389`）
8. 删备份、删 manifest（`:392-399`）

manifest 落盘位置 `{app_data}/skills-cli/remove-recovery/{skill}.json`
（`paths/skills_cli.rs:13-21`）。

### 1.4 锁顺序与 guard

`link.rs:3-4` 的注释即契约：
exclusive job lease（命令层）→ target mutation guard → guard 下重新校验 → FS 变更。

`acquire_target_mutation_guard`（`central_mutation/mod.rs:63-78`）**已是 target-scoped**，
直接传远端 `ActiveTarget` 即可，无需改造。

## 2. 目标结构

### 2.1 远端目录链接的创建（R1）

`SkillsCliFs` 增加 `create_managed_link(target: &str, link: &str) -> Result<ManagedLinkKind, _>`。

| 场景 | 手段 | 备注 |
| --- | --- | --- |
| Local | 现有 `create_skills_cli_directory_link` | 行为不变 |
| 远端 Unix（`remote_os` 非 Windows） | `ln -s <target> <link>` | 与本机 symlink 语义一致 |
| 远端 Windows | `cmd.exe //c mklink /J <link> <target>`，经远端 `sh` 调起 | 见下 |

**关于 `cmd.exe` / `mklink`**：spec `skills-cli-global.md:95-98` 禁止本机使用它们。
那条禁令的理由是「本机有 reparse API，不该退化到字符串拼命令」。
远端**没有** reparse API——SkillPort 只能通过远端 shell 发命令。
因此禁令的前提在远端不成立，spec 需要为远端行补一条明确例外并记录理由（§4）。
禁令中真正跨场景成立的部分保留：**不得退化为 copy**。

三条硬约束：

1. **创建后必须回探验证**。复用 `remote-inventory` 的探测能力，
   确认 slot 被分类为 `managed_link` 且指向正确 canonical。
2. **验证失败必须回滚**：删除刚创建的占位物，返回 `SkillsCliError::PlacementUnavailable`
   → 既有码 `skills_cli.placement_unavailable`，净写为零。不新增错误码。
3. **绝不 fallback 到 copy**。`InstallTransport::resolve_method`（`transport.rs:95-113`）
   在远端不允许 symlink 时退化为 copy——那个策略属 installation 域，
   Skills CLI 域禁止，因为它会把 `managed_link` 意图变成 `direct_copy` 事实。

远端 Windows junction 的真实行为依赖该主机的 sh 兼容层，标记 `UNVERIFIED`（RAC7）。

### 2.2 状态机闸门（R2）

link/unlink 只允许 `Missing ↔ ManagedLink`（spec `:205-206`）。
判据来自 `remote-inventory` 的分类结果，**在 guard 下重新分类**后再决定：

| 当前状态 | link | unlink |
| --- | --- | --- |
| `missing` | 允许 | 无操作（幂等成功） |
| `managed_link` | 无操作（幂等成功） | 允许 |
| `direct_copy` | 拒绝 `direct_copy_not_toggleable` | 拒绝，零写 |
| `conflict` | 拒绝 `placement_conflict` | 拒绝，零写 |
| `unavailable` | 拒绝 `placement_unavailable` | 拒绝，零写 |

三个拒绝码都已存在（`error.rs:178-180`），不新增。

### 2.3 远端安全卸载（R3、R4）

复用 §1.3 的三阶段模型，逐步骤给出远端手段：

| 步骤 | 本机 | 远端 |
| --- | --- | --- |
| 读 lock + 指纹 | `fs::read` + sha256 | `read_file_bounded` + **本机算 sha256**（不在远端算，避免依赖远端有 `sha256sum`） |
| canonical 改名为备份 | `fs::rename` | `mv` 脚本 |
| 删 managed links | `remove_verified_directory_link` | 见下的"验证后删除" |
| lock CAS 写回 | `atomic_write`（temp + persist） | 写 temp + `mv -f` 脚本 |
| 删备份 | `remove_dir_all` | `rm -rf` **仅限我们自己创建的备份路径** |

**"验证后删除"是 R3 的落点**，也是与 §1.2 那条 `remove_tree` 的根本区别：

删除一个平台 slot 前，在同一个远端脚本里先验证再删，避免 TOCTOU：

```sh
# 仅当 $p 是符号链接 / junction 时才删；是普通目录或文件一律跳过并报告
if [ -L "$p" ]; then rm -f "$p"; printf '%s\tremoved\n' "$p"
elif [ -e "$p" ]; then printf '%s\tskipped_not_link\n' "$p"
else printf '%s\tabsent\n' "$p"; fi
```

- **绝不对平台路径调 `rm -rf`**。平台 slot 只用 `rm -f`（Unix 链接）
  或远端 Windows 的 `rmdir`（junction 是空目录形态的重解析点，`rmdir` 只删链接不删目标）。
- `rm -rf` 只允许作用于**我们自己创建的 canonical 备份目录**，路径由我们生成，
  不来自用户数据。
- `skipped_not_link` 计入 partial outcome 的 skipped，不是错误。
- conflict 在建计划阶段就 bail，脚本根本不会被发到远端（零写）。
- independent direct copies 不进入任何删除脚本的路径清单。

### 2.4 recovery manifest 存放位置（R4，对 PRD 措辞的收紧）

PRD R4 原文是"recovery manifest 落在远端可恢复的位置"。设计定为
**manifest 仍写在 SkillPort 本机**，按 target 命名空间隔离：

```
{app_data}/skills-cli/remove-recovery/{target_id}/{skill_name}.json
```

理由：recovery 永远由 SkillPort 驱动。远端中断最常见的成因就是远端不可达——
此时写在远端的 manifest 恰恰读不到，正是最需要它的时刻失效。
本机 manifest 额外的好处是不往用户远端主机写我们的元数据。
manifest 内的路径字段记远端路径字符串，恢复时重新连接即可续做。

`target_id` 分目录是必须的：同一技能名可能同时存在于 Local 与多个远端 target，
不隔离会互相覆盖。Local 保持现有无 target 子目录的路径以免破坏既有 recovery
（迁移不在本任务范围）。

### 2.5 锁顺序与 guard（R5）

顺序与本机完全一致，唯一差别是 guard 传远端 target：

```
skills_cli lease（命令层，已有）
  → acquire_target_mutation_guard(&remote_active_target, OP, TIMEOUT)
  → guard 下重新探测并分类（一次远端往返）
  → 远端 FS 变更
  → drop guard → drop lease
```

`acquire_target_mutation_guard` 以 target id/kind 为键，
所以持有远端 guard **不会**阻塞 Local 写——AC6 的后半句正是验这一点。

### 2.6 批量往返预算（R7）

分块常量 **`K = 32`**，固定开销 **`C = 1`**，即远端命令次数 = `ceil(N / K) + 1`。

- `C = 1` 是 §2.5 的"guard 下重新探测"，一次探测覆盖全部选中项
  （复用 `remote-inventory` 的 heredoc 内嵌清单手法，与 N 无关）。
- 变更本身分块。**这里不用 heredoc 一次做完**，与列举不同，理由是：
  变更脚本一旦超时（`run_script` 是 `ProcessPolicy::standard()`，120s），
  已完成部分的结果就拿不回来了。分块把"结果丢失"的粒度限制在 32 项内，
  partial outcome 仍能报告前面所有块。
- `K = 32` 与仓库既有取值同量级（usage 批量读 64、scanner 批量读 4）。
- 计数口径：fake `CommandRunner` 的 spawn 次数；连接重试不计入。

### 2.7 远端 leftover（R6）

leftover 扫描/应用在 `central_updates/inventory/{scan,leftover_cleanup}.rs`，
它们 import `skills_cli::CliLockOwnership` 等（`lock.rs:269-291`）。

远端化的唯一要求是**注入远端 lock ownership 而不是本机的**：

- 扫描时 `cli_lock_protect=true` 的保护集合来自远端 lock 解析结果。
- 排除项照旧：lock 拥有的 canonical、已解析链接、
  `{mapped_detected_agent.global_skills_dir}/<name>` —— 其中 agent 行取自远端 target 的 DB。
- apply 全程持有该远端 target 的 guard。

反向断言（AC7）：远端扫描过程中不得出现对本机 lock 路径的读取。

## 3. 数据流

```
批量 unlink（N 项，单平台）
  → lease
  → acquire_target_mutation_guard(remote_target)
  → RT×1  guard 下探测全部 slot → 分类
  →       状态机闸门过滤：managed_link 才进变更清单，其余计 skipped
  → RT×ceil(N/32)  每块一个"验证后删除"脚本，逐条回报 removed/skipped_not_link/absent
  → 汇总 PlacementMutationOutcome { succeeded, failed, skipped }
  → drop guard → drop lease
  → 前端刷新远端库存

远端卸载（单技能）
  → lease → guard
  → 建计划（conflict → bail，零写）
  → 读远端 lock → 本机算 sha256
  → 写本机 manifest(prepared, target 命名空间)
  → mv canonical → 备份
  → 验证后删除 managed links
  → manifest(staged)
  → 读远端 lock 再算指纹 → CAS 比对 → 不匹配则 RecoveryRequired
  → 写回 lock（temp + mv -f）
  → manifest(metadata_committed)
  → rm -rf 备份（仅我们生成的路径）→ 删 manifest
```

## 4. 契约与兼容性

- **不新增 IPC 错误码**。拒绝路径复用 `direct_copy_not_toggleable` /
  `placement_conflict` / `placement_unavailable` / `recovery_required`（均在 `error.rs:178-186`）。
  命令签名不变 → 不触发 `pnpm docs:gen`。
- `SkillsCliFs` 新增 `create_managed_link` / `remove_verified_link` / `rename` /
  `atomic_write` 四个方法，Local 实现转调现有函数，行为不变。
- spec `skills-cli-global.md` 需要三处修订：
  1. §"managed link 实现"补远端行，写明远端 Unix 用 `ln -s`、远端 Windows 用
     `cmd.exe //c mklink /J`，并记录"本机禁令因缺少 reparse API 而在远端不适用"的理由；
     同时明确"不得退化为 copy"在远端同样成立。
  2. 删除路径补远端行：平台 slot 只用 `rm -f` / `rmdir`，`rm -rf` 仅限自建备份路径。
  3. 能力矩阵翻 `LinkPlatform` / `UnlinkPlatform` / `PreviewRemove` / `RemoveGlobal` /
     `LeftoverScan` 五行。
- recovery manifest 路径新增 target 子目录，属内部布局变更，Local 路径不变。

## 5. 权衡

- **manifest 放本机**：牺牲"SkillPort 数据丢失后远端可自恢复"，
  换取"远端不可达时仍能恢复"。后者是实际故障模式，前者不是。
- **变更分块而列举不分块**：两者对超时的容忍度不同。列举失败可整体重试且无副作用；
  变更失败必须能报告已完成的部分。
- **远端 Windows 用 `mklink`**：这是本设计里证据最弱的一环。
  真实行为依赖远端 sh 兼容层，只能标 `UNVERIFIED`。
  兜底是 §2.1 的"验证失败即回滚 + `placement_unavailable`"，
  保证最坏情况是功能不可用而非数据损坏。
- **不复用 `InstallTransport::remove_install`**：多一条删除路径是维护成本，
  但共用会把无条件 `rm -rf` 引入 Skills CLI 域，代价不可接受。

## 6. 回滚点

| 单元 | 内容 | 可否单独回滚 |
| --- | --- | --- |
| A | `SkillsCliFs` 四个新方法 + Local 实现（纯重构，行为不变） | 可 |
| B | 远端 link / unlink + 状态机闸门 + 批量分块 | 依赖 A |
| C | 远端卸载 + manifest target 命名空间 + CAS | 依赖 A |
| D | 远端 leftover | 依赖 C |
| E | spec 修订 + 能力矩阵翻闸 | 与 B、C、D 同批 |

B 与 C 可分别回滚：回滚 C 后远端只能 link/unlink 不能卸载，能力矩阵相应收回该行，
状态自洽。E 不能单独保留（spec 与实现矛盾即 PAC4 违规）。
