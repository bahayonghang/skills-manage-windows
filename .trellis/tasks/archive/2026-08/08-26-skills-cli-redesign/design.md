# Skills CLI 全局页重设计 — 共享技术设计

本文件只定义跨子任务契约。各 child 的 `design.md` 必须写清自己的数据结构、命令、状态机、测试和回滚，不得以父子树位置代替真实依赖。

## 1. 设计证据与权威顺序

1. 父/子任务 `prd.md` 的 observable requirements 和 AC。
2. 本文件与各 child `design.md` 的技术契约。
3. `research/design-contract.md` 的八区块、placement、update、交互和视觉契约。
4. 当前代码、`.trellis/spec/` 与测试所证明的仓库约束。
5. `research/skills-cli-redesign.dc.html` 只作为非规范静态构图灵感。

缺失的 `design_handoff_skills_cli/README.md`、`support.js` 和未执行的原型事件不参与验收。若恢复的外部交付物与 1–4 冲突，先回到 planning，不直接修改实现。

## 2. 子任务边界与唯一所有者

| 契约/文件族 | 唯一所有者 | 消费者 |
| --- | --- | --- |
| Rust inventory、placement、bounded doc、link/unlink、reveal、export writer、recent-source policy | `backend-contract` | 其余 children |
| 页面 content-width、`activeSurface` overlay controller、install mount seam、mutation-only add action、共享 action toast helper、页头/工具栏/分组、dense card、Dashboard census | `page-shell` | batch/install/detail/update |
| store 的 link/unlink/batch remove/export actions、selection、uninstall dialog 与操作语义 toast 调用 | `batch-actions` | detail |
| install wizard 独立 surface/recent-source store/view model 与 preview flow | `install-wizard` | 无 |
| detail local doc/focus state与 page-shell/batch action 组合 | `detail-drawer` | update 入口 |
| update DB/repository/check/apply/store/update drawer | `update-center` | page-shell/detail entry points |

不得再使用“谁先落地谁拥有”、占位回调完成 AC 或多个 child 同时新增同一 store action。依赖 child 未完成时，消费者 child 保持 planning，不以 placeholder 宣称可独立验收。

## 3. Placement 跨层契约

后端 inventory 返回每个 enabled Local install target 的 placement，而不是只返回已发现的 display-name 列表：

```rust
pub struct SkillsCliPlacement {
    pub agent_id: String,
    pub display_name: String,
    pub target_path: String,
    pub state: SkillsCliPlacementState,
    pub reason_code: Option<String>,
}

pub enum SkillsCliPlacementState {
    ManagedLink,
    DirectCopy,
    Missing,
    Conflict,
    Unavailable,
}
```

- `ManagedLink` 必须解析到当前 lock-owned canonical；Windows junction 与 symlink 均属于该状态。
- `DirectCopy` 是普通目录且未证明为受管 link。它可算平台关联，但不计 linked，link/unlink disabled。
- `Missing` 才允许 link。Windows 实现创建 junction；其他平台按现有受支持 symlink 机制。
- `Conflict` 不覆盖、不递归删；`Unavailable` 不触发本地 FS mutation。
- Unlink 只移除经 reparse/symlink 检查再次证明仍指向 canonical 的 link 本身。
- Inventory、详情、批量 preview 与计数全部消费同一状态，前端不得从路径字符串或 display name 二次猜测。

现有 `agents: Vec<String>` 可兼容保留，但新 UI 以 `placements` 为权威。backend-contract 同步修订 `.trellis/spec/backend/skills-cli-global.md`。

## 4. 安全后端命令面

后端 child 负责给出最终 Rust 类型和 IPC registry；共享最小命令能力为：

| 能力 | 必要输入 | 关键约束 |
| --- | --- | --- |
| Read SKILL.md | `skill_name` | lock-owned canonical、bounded ingestion 1 MiB、UTF-8、growth-safe |
| Link placement | `job_id, skill_name, agent_id` | Local/enabled target、Missing-only、junction/symlink-safe、exclusive job + target lock |
| Unlink placement | `job_id, skill_name, agent_id` | ManagedLink-only、重新验证目标、只删 link |
| Reveal canonical | `skill_name` | backend 从 lock 解析 owned canonical；不接受 renderer 任意路径 |
| Write export | `path, SkillsCliInventoryExportV1` 的稳定序列化结果 | save target path policy、blocking IO、schema/version/count 校验、错误脱敏 |
| Preview remove | `skill_name` | fresh lock/placement；只返回逻辑影响，无 path/argv；conflict fail closed |
| Recent sources（复用既有 generic settings，非上述六条新增 registry command） | exact domain key/command | schema、上限 8、去重、控制字符、generic settings policy |

只有 `skillsCliStore` 的 IPC adapter 可以调用这些命令。所有失败使用稳定 error code，经 `formatBackendError` 显示；动态路径、PAT、命令行和 provider details 不进入 UI、日志或 telemetry。

## 5. Export snapshot

Toolbar 和 batch 共用一个 serializer，scope 由调用方显式传入：

```ts
interface SkillsCliInventoryExportV1 {
  schemaVersion: 1;
  exportedAt: string;
  scope: "all" | "selected";
  skillCount: number;
  skills: Array<{
    name: string;
    source: string | null;
    sourceType: string | null;
    sourceUrl: string | null;
    installKind: string;
    canonicalPath: string;
    folderHash: string | null;
    installedAt: string | null;
    updatedAt: string | null;
    placements: Array<{
      agentId: string;
      displayName: string;
      state: "managed_link" | "direct_copy" | "missing" | "conflict" | "unavailable";
    }>;
  }>;
}
```

上述字段白名单是唯一 v1 schema；不得省略、改名、增加别名或新增未知字段。`skillCount` 必须严格等于 `skills.length`。不导出 PAT、命令环境、错误 details、绝对平台目标路径或 SKILL.md 正文。默认文件名包含日期与 scope。save dialog 取消返回无操作成功；序列化、dialog 和写入失败分别由 owning surface 显示安全错误。

## 6. Overlay、Escape 与焦点

page-shell 提供受控 `activeSurface`/open-close 接口、独立 install mount seam、mutation-only add action、共享 action-toast helper 与内容宽度信号；batch 拥有 uninstall/link menu、selection 与其余 mutation actions，install/detail/update 拥有各自本地内容状态。install 只能通过 mount seam 接入独立 surface，不再修改 page-shell controller 或 canonical `skillsCliStore`。Add mutation resolve/reject 与随后 inventory refresh 必须由两个 catch 边界处理；refresh 失败只能提示 stale data，不能把已成功 mutation 重新解释为失败。

- Dialog/Menu/Drawer 使用 Base UI 的 topmost dismissal 和 `onOpenChange` reason。
- 不增加与 Base UI 竞争的无条件 `window.keydown`。
- 当 `activeSurface` 为空且 link menu 未处理本次事件时，页面才清除 selection。
- 每个 consumer 关闭时清理 local error/focus/selection snapshot 并把焦点还给触发器。
- 多层真实组件测试验证一次 Escape 只关闭一层；Windows WebView2 手工证据仍为独立 gate。

## 7. 更新数据与持久化边界

update-center 必须把三类身份分开：

- `installed baseline`：只由 update-center 的 Verify exact-match 或成功 Apply/Reinstall 建立；普通
  install-wizard 不掌握 pinned upstream identity，不写 baseline。
- `last observed upstream`：最近一次成功网络检查观测到的 identity/hash/message。
- `current content hash`：检查时对 canonical 内容按与 baseline 相同算法计算的值。

`pending update` 从 installed baseline 与 last observed upstream 派生或显式持久化，不能从“本次与上次检查 SHA”派生。新装或 legacy row 缺 baseline 时为 `baseline_required`；只有 Verify exact-match 或一次成功 Apply/Reinstall 能建立可信 baseline，不能静默标成 current。

网络检查按去重 repository/source 工作，复用 command boundary 的 SecretStore auth 与 GitHub client，读取 remaining/reset/retry headers。非 GitHub、rate limit 与局部失败各自形成可缓存状态；已有成功缓存不因单仓库失败被清空。

新增 SQLite schema 必须使用下一连续 migration descriptor、checksum、新库/升级/future-version fixtures。Apply 使用 `.trellis/spec/backend/fs-db-operation-journal.md` 的 recoverable FS+DB journal；rollback 通过后续兼容 migration/feature disable 保持旧二进制可读，不能简单删代码却留下无法识别的 future-version DB。

## 8. UI 数据流

```text
Tauri commands
  -> skillsCli IPC adapter/store (唯一 invoke owner)
  -> page view model (filter/group/count/export snapshot)
  -> page-shell controlled surfaces
  -> batch/install/detail/update presentational components
```

- `runtimeError`、`inventoryError`、`actionError` 保持独立；stale inventory 与 cache 在 refresh/check 失败时保留。
- link/unlink 允许乐观更新，但只对 `Missing ↔ ManagedLink`，失败按 inventory snapshot 回滚。
- 更新、卸载和安装以 refresh 后后端 snapshot 为最终权威，不靠组件本地数组拼接完成。
- 所有长操作使用 `job_id`/correlation；是否暴露 Cancel 由后端是否建立可证明的 cancellation seam 决定。

## 9. 视觉与组件边界

- `UnifiedSkillCard` 增加明确的 Skills CLI dense model/density；不新建第二套卡片实现，不用现有 168px compact 冒充 76px 目标。
- content-width 由页面容器观测，1180/900/720 精确断点分别驱动 grid 与 drawer；不以 viewport 或 `md=768px` 代替。
- `InventoryCensus` 只搬到 Dashboard，不复制组件。
- token、字体、图标、i18n 和焦点热区遵守 `research/design-contract.md` 与现有 frontend specs。

## 10. 兼容、回滚与证据

- 现有 IPC 字段语义不变；新增字段应有 serde/type adapters 与 legacy fixture。变更后运行 `pnpm ipc:codegen`、`pnpm docs:gen`，check 命令保持只读。
- 每个 child 在其明确所有权内形成可独立验证的提交；消费者回滚不得删除前置 child 的共享 contract。
- `--force`、Windows junction、GitHub real-data、installer/WebView2、native focus/rendered fidelity 在取得直接证据前保持 `UNVERIFIED`。
- 父任务不启动实现；六个 child 按 PRD 依赖逐一完成后，父任务只执行 AC1–AC16 集成 gate。
