# SkillKit 与 SkillPort 对标研究

## 1. 快照与方法

- SkillKit：`ref/skillkit` commit `9e89e03a0ea1f7a5e3551d7abc7ea98a7433eb77`，2026-07-16，`chore(release): v0.4.0`。
- SkillPort：当前 `dev` 工作树，研究阶段未改产品代码。
- 方法：源码/配置/测试/历史规格交叉检查，SkillKit 两张 light 截图目视审阅，并使用 `impeccable audit` 框架和确定性 detector 辅助定位。
- detector 只用于发现漂移线索，不直接判定缺陷。2026-07-18 扫描结果：SkillKit 220 项（10 warning / 210 advisory），SkillPort 173 项（1 warning / 172 advisory）。
- 上一条的 173 是 Impeccable detector 项数；与下文“173 个数值型 arbitrary 字号”恰好同数，但统计对象不同，不能混为同一基线。

## 2. 技术与产品面

| 维度 | SkillKit | SkillPort | 判断 |
| --- | --- | --- | --- |
| 桌面栈 | Electron 31、React 18、better-sqlite3（`ref/skillkit/apps/desktop/package.json:23`、`:28`、`:44`） | Tauri 2、React 19、Zustand、SQLite（`package.json`、`src-tauri/Cargo.toml`） | 不迁移运行时架构 |
| 安装入口 | Link / ZIP，输入实时识别 share/GitHub（`InstallView.tsx:79`、`:114`、`:333`） | Central 头部直接打开 GitHub wizard（`CentralSkillsShell.tsx:354`） | 适配统一入口，但保留现有 wizard |
| GitHub 预览 | tree API + raw `SKILL.md`，失败回退 tarball（`installer.ts:704`、`:719`） | 本地 preview 先下载完整 archive（`preview.rs:136`），再构建候选与文件清单 | 吸收 acquisition 快路径 |
| GitHub 安装 | 选中子树逐文件下载，超过 300 文件回退整包（`installer.ts:736`、`:753`） | 本地 import 再下载完整 archive（`import.rs:32`），之后按 sourcePath 写入 | 吸收选中子树下载，但用现有预算与原子写入 |
| 预览可信度 | 候选元数据为主 | preview DTO 已含冲突、pluginName、文件清单（`types.rs:38`），缺 `SKILL.md` 会 fail closed（`preview.rs:99`） | SkillPort 明显更强，禁止降级 |
| 远程目标 | 无同等 SSH/WSL preview workspace | command 按 Local / SSH / WSL 分流（`commands/github_import.rs:31`、`:64`） | 保持现状 |
| 技能列表 | 响应式 CSS grid，全量 map（`theme.css:286`、`MySkillsView.tsx:404`） | 列表 >60、网格 >40 时虚拟化（`CentralSkillListContent.tsx:188`、`:203`） | 保持 SkillPort |
| 扫描 | 同步 `readdirSync/statSync`，逐行 upsert/delete（`scan.ts:9`、`:91`） | blocking FS 隔离 + 1..8 有界并发（`scanner/mod.rs:247`、`:277`），单事务持久化（`persistence.rs:354`） | SkillKit 只作反例 |
| 操作历史 | 页面内 Recent installs，仅 React state（`InstallView.tsx:77`、`:110`） | 持久化 Operation Logs，有列表/筛选/导出 | 明确不迁移 Recent |
| 深链 | `skillkit://share` 与 OAuth（`electron/main.ts:84`） | `tauri.conf.json:91` 无 deep-link 配置 | 仅考虑 import-intent 深链 |
| 测试 | package scripts 无测试框架 | Vitest + Rust tests + `just ci` | 不吸收无测试做法 |

## 3. 界面审计

### 3.1 评分

该评分是代码与现有截图审阅，不是完整浏览器/辅助技术实测；未测项不会写成通过。

| 维度 | SkillKit | SkillPort | 关键依据 |
| --- | ---: | ---: | --- |
| Accessibility | 2/4 | 3/4 | SkillKit 有部分 aria，但使用原生 `confirm`（`MySkillsView.tsx:165`）且没有 reduced-motion；SkillPort 有 focus/reduced-motion 基线，但小字号与弱化文字仍需逐区验证 |
| Performance | 2/4 | 3/4 | SkillKit 有 tree 快路径与 10 分钟/4 项 LRU（`installer.ts:46`），但同步扫描、全量卡片和布局属性动画；SkillPort 有虚拟化、并发扫描和事务，GitHub archive 路径仍可优化 |
| Responsive | 2/4 | 3/4 | SkillKit 安装页只有有限断点并在宽屏留下大量空白；SkillPort 以 900x600 桌面最小窗口为基线并有结构断点，仍需字号缩放回归 |
| Theming | 2/4 | 4/4 | SkillKit detector 报 125 个颜色和 32 个圆角漂移；SkillPort 有 6 主题、14 accent 与语义 token，少量 detector advisory 需核验 |
| Anti-patterns | 2/4 | 3/4 | SkillKit 固定卡片海、chip 导航、手绘内联 SVG 与大留白明显；SkillPort 设计身份清晰，但任意字号与局部 pill/uppercase 需要治理 |
| **Total** | **10/20** | **16/20** | SkillKit 可学机制多于视觉；SkillPort 应优化一致性而非重做方向 |

### 3.2 SkillKit 可学点

- **来源识别反馈**：输入时立即告诉用户识别为 GitHub/share/unknown（`InstallView.tsx:79`），降低提交后的错误成本。
- **安装页来源聚合**：Link 与 ZIP 位于同一任务入口，目标选择与来源选择形成清晰顺序（截图 `skill-install-light_en.png`）。
- **tree manifest 快路径**：先获取完整路径清单，只读取候选 `SKILL.md`，失败统一退回 archive（`installer.ts:511`、`:704`）。
- **选中子树下载**：下载前预检文件数量，所有文件准备完成后再安装，避免批量操作装一半（`installer.ts:753`、`:807`）。
- **短生命周期缓存思想**：list 与 install 复用 10 分钟、最多 4 项的解包结果（`installer.ts:46`），值得转化为有界 metadata cache，而不是照搬 tar 目录缓存。
- **contextual toolbar 思想**：视图把上下文控件 portal 到固定顶栏槽位（`ToolbarSlot.tsx:1`）。SkillPort 已有页面 shell/toolbar 分层，本次不另立任务，只保留为组件组织参考。

### 3.3 SkillPort 已经更强的点

- `UnifiedSkillCard` 是唯一技能卡实现；Central 直接复用（`CentralSkillListContent.tsx:9`、`:143`）。
- 大列表/网格按阈值虚拟化，且搜索强制单列扫读（`CentralSkillListContent.tsx:116`、`:188`、`:203`）。
- GitHub 导入已有 Preview → Confirm → Result、文件树、pluginName 分组、扁平 selection、冲突处理和进度反馈。
- 本地 preview 文件清单直接从同一 snapshot 派生，缺失或不含根 `SKILL.md` 时阻断（`preview.rs:99`）。
- archive 有 128 MiB 压缩、20,000 文件、256 MiB 展开、32 MiB 单文件等集中预算（`resource_budget.rs:7`）。
- 扫描使用有界并发并将本次扫描结果一次事务提交（`scanner/mod.rs:247`、`persistence.rs:354`）。
- Operation Logs 已经覆盖持久化历史，SkillKit session-only Recent installs 没有增量价值。

### 3.4 SkillPort 排版 planning 基线

- 2026-07-18 的生产 `src/**/*.ts(x)` 中共有 173 个数值型 arbitrary 字号，分布于 64 个文件：133 个 px（23x10px、107x11px、2x12px、1x13px）与 40 个 rem。
- 133 个 10–13px 命中中有 22 个与同一行的 alpha 前景组合：21 个 `foreground/*`，另有 1 个 `text-primary/85`。因此只按 class 名包含 `foreground` 统计会得到 21，不代表规划中的 22 不可复现。
- typography 子任务启动时必须使用相同口径重跑，以 task-start inventory 作为实施分母，并保留 planning 快照与 delta；前序 unified import 可能改变 `CentralSkillsShell.tsx`，不能把今天的数量永久硬编码为未来事实。

可复现 PowerShell 口径：

```powershell
$fontSizes = @(rg -n -o --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(0?\.[0-9]+|[0-9]+(?:\.[0-9]+)?)(rem|em|px)\]' src)
$smallPx = @(rg -n --glob '*.tsx' --glob '*.ts' --glob '!src/test/**' 'text-\[(10|11|12|13)px\]' src)
$alphaRisk = @($smallPx | Where-Object { $_ -match 'text-(?:muted-)?foreground/(?:[5-8][0-9]|90)|text-primary/(?:[5-8][0-9]|90)' })
```

## 4. 采纳/拒绝矩阵

| 候选 | 判断 | 用户价值 | 成本/风险 | 进入任务 |
| --- | --- | --- | --- | --- |
| 单一“添加技能”入口 | 适配后吸收 | 降低入口发现成本，为未来来源扩展提供稳定 intent router | 不能把 GitHub wizard 再包进嵌套 modal，也不能把平台安装混进导入 | `07-18-unified-skill-import` |
| GitHub URL 识别与预填 | 直接吸收机制 | 粘贴后立即得到明确来源与校验反馈 | 只支持 SkillPort 实际来源，不伪装支持 share link | 同上 |
| 本地 ZIP 导入 | 适配后吸收 | 支持离线/私有交付技能包 | ZIP Slip、zip bomb、重复/大小写冲突、模糊多 skill、原子落盘 | 同上 |
| Git tree 预览快路径 | 适配后吸收 | 避免为候选预览先下载整个仓库 | 必须保持 PAT、镜像、plugin manifest、文件树预算与错误语义 | `07-18-github-import-manifest-fast-path` |
| 只下载选中 skill 子树 | 适配后吸收 | 多 skill 大仓库可显著减少传输与内存 | 根 skill 等于全仓库；逐文件请求可能反而变慢，必须阈值/回退 | 同上 |
| 10 分钟 LRU | 条件吸收 | 预览和导入连续操作可复用 tree metadata | 私有路径元数据、缓存漂移、复杂度；仅在基线证明需要时加入 | 同上 |
| `skillport://` import 深链 | 可选吸收 | 浏览器/文档可一键把 GitHub 来源送入现有确认流程 | 新插件、单实例、URI 注入和 Windows bundle 验证 | `07-18-skillport-import-deep-link` |
| 高密度排版 token | SkillPort 自身优化 | 保持密度同时提高层级、缩放和 AA 可验证性 | detector 有误报，禁止机械全局替换 | `07-18-dense-typography-wcag` |
| 暖白大留白与固定四列卡片 | 明确拒绝 | 无 | 与调度台密度、主题身份、虚拟化和窄窗扫描冲突 | 无 |
| 横向 agent chip 主导航 | 明确拒绝 | 无 | 平台数增长后横向溢出，弱于现有侧栏/筛选 | 无 |
| 按 name 跨工具聚合 | 保持现状 | SkillPort 中央技能本身已是跨平台身份 | 搬运会重复建模并可能掩盖 stable skill id | 无 |
| session-only Recent installs | 明确拒绝 | 无增量价值 | 弱于持久化 Operation Logs | 无 |
| 账号/OAuth/7 天分享服务 | 明确拒绝 | 需要独立产品论证 | 外部后端、隐私、运维和本地优先边界变化 | 无 |
| 同步扫描/逐行 DB 写入 | 明确拒绝 | 无 | 阻塞主进程且事务一致性更差 | 无 |
| React `key` 强制整页重挂载 | 明确拒绝 | 无 | 丢局部 UI 状态并制造不必要渲染（`App.tsx:40`） | 无 |
| 原生 `confirm` / 内联手绘 SVG | 明确拒绝 | 无 | i18n、a11y、视觉一致性与测试能力更差 | 无 |

## 5. 性能假设与度量

| 假设 | 当前证据 | 实施阶段度量 | 停止/回退条件 |
| --- | --- | --- | --- |
| tree preview 比 archive preview 少传输 | SkillPort 当前总是 archive（`preview.rs:143`） | 相同仓库记录 request 数、传输字节、解析峰值、总耗时 | 受支持仓库没有稳定减少字节，或 API 请求放大导致更慢 |
| 选中子树下载比整包导入更快 | 当前 import 再取 archive（`import.rs:34`） | 根 skill、1/10/50 个子 skill 三类 fixture；比较总字节和 wall time | 根 skill 或小仓库走 archive 更优；阈值按数据选择 |
| tree cache 有价值 | SkillKit 的 tar LRU 只能证明思路，不证明 SkillPort 需要 | 比较 preview→confirm→import 的重复 API 成本 | 无明显收益则不加缓存 |
| 排版 token 可提高可读性且不降密度 | planning inventory 为 173 个数值型 arbitrary 字号/64 文件：133 px + 40 rem；22 个 alpha-risk 小字。detector 仅作发现线索 | task-start inventory/delta；6 主题、代表 accent、三档 font scale、900x600 最小窗截图与对比度 | 出现关键控件溢出/遮挡或单位屏信息量显著下降时分区回滚 |

`src/lib/performance.ts:1` 当前只有 mark，没有 measure/预算闭环；子任务需要为自身指标建立可复现记录，但本路线图不额外扩张为全应用遥测项目。

## 6. 结论

最值得学习的是 SkillKit 的“来源意图收敛”和“先清单、后按需下载、失败回退”两类机制。SkillKit 的视觉、同步扫描、刷新方式、分享账号体系和测试成熟度不应迁移。SkillPort 的优化重点是把已有更强的 preview/安全/远程契约放到更高效的 acquisition 层上，并清理自身高密度排版中的语义漂移。
