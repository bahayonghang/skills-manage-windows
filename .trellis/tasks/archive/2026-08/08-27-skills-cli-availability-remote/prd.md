# Skills CLI 全局页可用性与远端支持（父任务）

状态：planning
类型：父任务（拥有源需求、子任务映射、跨子任务验收与最终集成评审）

## Goal

让 `/skills-cli`（Skills CLI global）页面在三个维度上可用：

1. 不因一次 npx probe 失败就把整页读写能力锁死，并去掉误导性告警。
2. 对失效条目提供安全的统一清理，并把多选批量能力补齐到「更新」维度。
3. 从 Local-only 扩展到远端 SSH / WSL 目标。

## Source Requirements（用户原始诉求）

| ID | 用户原话 | 归属子任务 |
| --- | --- | --- |
| U1 | 「当前 skills 这个包是通过 npx skills 来执行的，本地没有通过 npm install 安装，所以你检测不到是正常的，请帮我删除这个警告」 | `08-27-skills-cli-doctor-gate` |
| U2 | 「下面的 unavailable 帮我设计个统一删除的功能和对应的按钮」 | `08-27-skills-cli-bulk-cleanup` |
| U3 | 「还得做一个 skills 多选功能，支持多选更新、删除、unlink 等功能」 | `08-27-skills-cli-bulk-cleanup` |
| U4 | 「优化相关样式和排版」 | `08-27-skills-cli-bulk-cleanup` |
| U5 | 「这个 skills cli 页面还需要远端 ssh 可用，补充这个功能」 | `08-27-skills-cli-remote-target` |

## Confirmed Facts

完整证据见 `research/current-state-evidence.md`。以下为影响任务拆分的结论：

- **F1**：应用从不检测「本地 npm install 的 skills」。argv 固定为
  `<node> <npx-cli.js> --yes --package=skills@1.5.23 -- skills …`
  （`argv.rs:286-290`，spec `skills-cli-global.md:77-80`）。因此 U1 中用户对成因的判断与代码不符，
  但诉求（告警是噪音、且不该锁死页面）成立。
- **F2**：`runtimeBlocked`（`SkillsCliView.tsx:151`）会禁用 uninstall / link / unlink / 批量操作，
  而按 spec 这些路径**根本不 spawn CLI**（`skills-cli-global.md:81-83,99-100,184`）。
  这是既有的过度封锁缺陷，不只是文案问题。
- **F3**：卡片上的 `Unavailable` 徽章在「所有 placement 都是 unavailable」时才出现
  （`SkillCardDenseRow.tsx:30-60,87`）。后端有 4 种 reason：
  `canonical_missing`（真失效）、`platform_unsupported`、`platform_not_detected`、`platform_disabled`
  （`placement.rs:73-110`）。后三种代表技能健康但平台不可用。
  **按徽章字面批量删除会误删健康技能。**
- **F4**：多选框架已由归档任务 `08-26-batch-actions` 交付：选择模式、卡片复选框、组头 Select all、
  批量栏（Link / Unlink / Export selected / Uninstall）、partial-failure 语义均已存在。
  U3 的真实缺口只有「批量更新」与「按平台批量 unlink」。
- **F5**：`skills_cli_apply_updates` 每请求只接受一个 `repositoryKey`
  （`generatedCommandMap.ts:989-993`）。跨仓库批量更新必须分组串行。
- **F6**：Skills CLI 目前是显式 Local-only，有四道闸门：后端 `ensure_local_target`
  （`skills_cli/mod.rs:247-252`）、spec `skills-cli-global.md:12,70-72`、页面 `SkillsCliView.tsx:210-217`、
  侧边栏 `Sidebar.tsx:113-114`。远端化必须先修订 spec 契约。
- **F7**：远端传输已有成熟基座（`ConnectedRemoteTarget`、`TargetContext`、`InstallTransport`、
  `Scope`/`FsBackend`、target-scoped mutation guard），但外壳调用 `ssh.exe`，**无持久会话**，
  每次命令一次握手。

## Decisions

用户于规划阶段裁决，已写入各子任务工件：

- **D1（U1）**：**完全移除 doctor 的 `skills --help` 探测**，只保留 Node 版本检测；
  安装失败在实际 `add` 调用时暴露。同时修掉过度封锁。
  → `08-27-skills-cli-doctor-gate`
- **D2（U2）**：清理入口**覆盖所有 `Unavailable` 技能**，但确认对话框按 `reasonCode` 分两组，
  **默认只勾选 `canonical_missing`**，平台侧原因需手动勾选并显示风险提示。
  → `08-27-skills-cli-bulk-cleanup`
- **D3（U5）**：远端交付**完整读写**（含 install / remove / update）。
  该范围超出单子任务体量，`08-27-skills-cli-remote-target` 降为中层父任务并拆为四个递进子任务。

## Task Map

```
08-27-skills-cli-availability-remote        (父)
├── 08-27-skills-cli-doctor-gate            U1
├── 08-27-skills-cli-bulk-cleanup           U2 U3 U4
└── 08-27-skills-cli-remote-target          U5（中层父）
    ├── 08-27-skills-cli-remote-seam        传输接缝 + 远端路径/doctor + spec 修订
    ├── 08-27-skills-cli-remote-inventory   远端只读列举 + 前端解闸
    ├── 08-27-skills-cli-remote-mutate      远端 link/unlink/安全卸载/leftover
    └── 08-27-skills-cli-remote-install-update  远端安装与更新
```

| 子任务 | 交付物 | 独立可验证性 |
| --- | --- | --- |
| `08-27-skills-cli-doctor-gate` | 移除探测、修掉过度封锁、修正安装失败错误映射、spec + 测试同步 | probe 不再存在；Node 正常时所有非 spawn 操作可用 |
| `08-27-skills-cli-bulk-cleanup` | 分组清理入口、批量更新、按平台批量 unlink、排版优化 | UI 行为与 IPC 调用面 |
| `08-27-skills-cli-remote-target` | 中层父：远端需求集与跨子任务验收 | 四个子任务合入后的集成评审 |

## Ordering Constraints

父子结构不是依赖系统，以下顺序已写入各子任务工件：

1. `doctor-gate` 最先，且必须合入 `dev` 后其余才启动。它定型 doctor 语义与
   `runtimeBlocked` 传播面，`bulk-cleanup` 与 `remote-seam` 都依赖该结果。
2. `bulk-cleanup` 与 `doctor-gate` 共享 `SkillsCliView.tsx` / `SkillsCliBatchBar.tsx`，
   禁止同一工作树并行写。
3. 远端子树内部严格递进：`remote-seam` → `remote-inventory` → `remote-mutate` →
   `remote-install-update`。
4. `bulk-cleanup` 与远端子树之间无产品语义依赖，可并行，但都会碰 `SkillsCliView.tsx`，
   需在不同工作树或错开时间。

## Cross-Child Acceptance Criteria

- PAC1 (F1,F2；机制见 `doctor-gate` R2/R3/R4)：D1 删除探测后，doctor 只检测 Node，
  因此「点击前禁用」与「调用时报错」是两个不同时机，必须分开验收。
  原表述「在 npx probe 失败的机器上……spawn 入口被禁用」在 D1 下不可判定——
  Node 正常而 `npx-cli.js` 缺失时不存在任何点击前信号（TPR-02）。
  - [ ] PAC1a：Node 缺失或低于 22.20 时，Install 与来源预览在**点击前**禁用，
        并显示 `skills_cli.node_missing` 公开句。此时禁用状态由 doctor 结果驱动，可在操作前判定。
  - [ ] PAC1b：Node 正常但 `npx-cli.js` 无法解析或子进程无法 spawn 时，**不存在点击前信号**。
        Install / 来源预览在实际调用时失败，返回稳定且可理解的 `skills_cli.cli_unavailable`
        并经 toast 呈现；不得为了恢复点击前禁用而重新引入已被 D1 删除的探测。
  - [ ] PAC1c：PAC1a 与 PAC1b 两种情形下，列举库存、打开详情、link / unlink / 卸载 / 导出
        全部保持可用——即页面不存在 `runtimeBlocked` 式的全局封锁。
- [ ] PAC2 (F3)：不存在任何「按 placement state 字面批量删除」的入口。
  所有批量清理入口的判据可追溯到 `reasonCode`，且在「平台全未检测」的场景下不选中健康技能。
- [ ] PAC3 (F4)：新增批量能力复用既有 `skillsCliStore` action 与 `SkillsCliBatchBar`，
  没有出现第二套选择状态或第二套 toast helper。
- [ ] PAC4 (F6)：`.trellis/spec/backend/skills-cli-global.md` 与实现一致；
  Local-only 契约要么保留、要么被显式修订，不存在 spec 与代码互相矛盾的中间态。
- [ ] PAC5：`src/i18n/locales/en.json` 与 `zh.json` 新增键成对；
  公共措辞变更时 `README.md` / `README_CN.md` 同步。
- [ ] PAC6：每个子任务各自 `just ci` 通过；父任务在三个子任务合入后做一次集成评审，
  确认 Windows x64 Tauri bundle 下页面行为符合 PAC1–PAC5。

## Out of Scope（父级）

- 迁移到 russh / 持久 SSH 会话池（`plans/ssh-perf` 独立议题）。
- 改变 `SKILLS_CLI_NPM_SPEC` 的 PIN 版本。
- SkillPort Central（`~/.skillsmanage/skills/`）与 `skillport-cli` 二进制的行为。
- 把 `direct_copy` 自动转换为 junction/symlink。
- Skills CLI 快照的导入能力（导出仍只承诺 v1 JSON）。

## Open Questions

无。Q1 / Q2 / Q3 已由用户裁决，记录在上方 Decisions。

Q4（远端 update apply 的快照获取路线）已在规划阶段关闭（TPR-01）：
定为**本机拉取 + 经 SSH 下发**，因为 `remote-install-update` 的 R6
「凭据不得写入远端主机」排除了远端自取。
决策与理由记录在 `08-27-skills-cli-remote-install-update/prd.md` 的 D1 与 `design.md` §2.4。
原先「等远端传输实测数据后再定」的前提不成立——性能只在两条路线都被允许时才是决定因素。
