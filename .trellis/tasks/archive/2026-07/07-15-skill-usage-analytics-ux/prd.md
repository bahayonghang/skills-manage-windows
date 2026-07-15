# 优化 SkillPort 技能使用统计

## Goal

把现有“调用次数展示页”升级为可信、可解释、可继续操作的技能使用分析页：用户应能判断哪些技能被频繁使用、在哪些平台和项目中使用、最近是否仍活跃、技能本身的上下文体量如何，并能从统计结果进入对应技能或统计详情。

## Background

- `ref/codexU` 的 Skill 统计以本机 session 事件为事实源，Skill 行同时展示加载次数、线程数、来源、最近加载时间、`SKILL.md` 静态 Token 估算与文件大小；工具调用另有独立 TOP20 和明确标注的估算 Token。证据：`ref/codexU/Sources/CodexUsageWidget/main.swift:194`、`:7457`、`:1524`、`:1864`。
- SkillPort 当前支持 8 个 provider，其中 Claude Code、Codex、Droid、OpenCode、Grok 有真实采集器，Antigravity、Kiro、Zed 仍是 stub；本任务必须保留本地/SSH/WSL target 与平台筛选能力。证据：`src-tauri/src/services/usage/providers/mod.rs:1`、`src-tauri/src/services/usage/mod.rs:35`。
- 当前 `skill_calls` 只保存技能名、时间、项目、会话和平台，足以做频次/项目/会话聚合，但不包含技能路径、稳定 skill id、静态 Token 或工具调用事件。证据：`src-tauri/src/db/schema/usage.rs:19`、`src/types/usage.ts:3`。
- 当前页面由 4 个 KPI、平台筛选、技能频次、16 周热力图、最近调用和 provider 状态组成；技能行仅显示名称、相对条形和次数。证据：`src/pages/SkillUsageView.tsx:111`、`src/components/usage/SkillBarChart.tsx:102`。
- 单技能详情 IPC 和 Zustand state 已存在，但页面没有消费 `detail/loadDetail`；无法映射到中央库的技能点击后没有可见结果。证据：`src-tauri/src/commands/usage.rs:264`、`src/pages/skillUsageBindings.ts:74`、`src/components/usage/SkillBarChart.tsx:58`。
- 每日聚合当前按 UTC 分日，系统时区午夜附近的调用会落到错误日期；codexU 明确以系统/配置时区计算日期并覆盖跨日与 DST。证据：`src-tauri/src/db/repos/usage_repo.rs:281`、`src-tauri/src/services/usage/aggregate.rs:125`、`ref/codexU/Sources/CodexUsageWidget/Domain/StatisticsTimeZone.swift:60`。
- 用户已确认本任务仅完成技能统计闭环；codexU 同页的工具 TOP20、工具事件采集与工具 Token 归因不纳入本任务。

## Requirements

### R1. 统计口径可信且可解释

- 每日趋势按运行 SkillPort 的系统本地时区分日；各区块必须明确标出固定口径：KPI/技能排行为全部已记录历史、热力图为最近 16 周、最近调用为最新 20 条，不能无提示地混成同一时间范围。
- 0、无记录、未检测到、采集失败和估算不可用必须是不同状态；缺失值不得伪装成 0。
- `SKILL.md` 静态 Token 必须显式标注“估算”，并说明它表示技能文件体量而非该技能导致的任务 Token 消耗；说明不得暴露 prompt、回复正文或 tool arguments。
- 平台筛选、active target 切换和强制刷新后，所有受筛选面板必须来自同一 target/source；快速连续切换不得由旧请求覆盖新选择。

### R2. 技能排行提供决策所需上下文

- 每行至少展示技能名、调用/加载次数、会话数、最近使用时间；项目数、来源/匹配状态和 `SKILL.md` 静态 Token 作为空间允许时的次级信息。
- `SKILL.md` 静态 Token 只在技能能唯一映射到当前 target 的技能记录且内容可读取时计算；文件缺失、名称冲突或未映射时显示明确的 unavailable 状态。
- 技能名相同但存在多个候选时不得静默跳到任意中央技能；必须保留“无法唯一映射”的诚实状态。
- 排序至少覆盖调用次数、最近使用和名称；若加入静态 Token 排序，必须与调用次数排序使用明确的 segmented/menu 控件，不能用难以识别的循环按钮。

### R3. 从统计结果继续操作

- 点击任意技能行都打开同一页内的统计详情，复用现有 `SkillUsageDetail` 能力，展示调用数、首次/最近使用、项目分布和技能自身趋势。
- 能唯一映射到中央技能的行和统计详情额外提供明确的“打开技能”动作，进入现有技能详情路由；未映射或名称冲突时不显示错误跳转动作。
- 最近调用列表沿用同一统计详情选择和中央技能动作规则，且项目路径默认只显示末级名称；完整路径不作为默认可见文本。

### R4. 页面结构服务高频扫视

- 保留 SkillPort 的高密度“调度台”风格，不复制 codexU 的紫色渐变、额度环或大面积卡片皮肤。
- 顶部控制区整合 target、平台、固定时间口径说明和刷新状态；本任务不新增时间范围切换器。窄窗口下结构换行但不让筛选项或标题溢出。
- 主区优先级为“技能排行/趋势 -> 最近调用 -> 数据源诊断”；provider 健康属于诊断信息，不应与技能排行争夺同等首屏面积。
- 技能排行使用紧凑列表/表格式行，避免当前只有一条进度条和次数、其余上下文必须靠猜的状态。
- 加载使用稳定尺寸 skeleton；空状态说明缺少哪类数据以及可执行的下一步；刷新保留旧数据时明确标注缓存/过期状态。
- 图表状态除颜色外同时提供数字、文本或 tooltip；键盘、焦点态、中英文和 reduced-motion 均满足现有设计系统约束。

### R5. 保持现有产品边界

- 继续本地优先，不上传 usage、线程、路径或日志数据。
- 前端只通过 `@/lib/ipc` -> Zustand store 访问新命令；组件不得直接 `invoke()`。
- 本地批量文件读取不得阻塞 async runtime；远程 target 的元数据读取必须通过现有 target/FsBackend 路径，不得把远程文件误当本地路径读取。
- 保留中央技能卡片上的近 30 天调用徽标和现有 platform filter 行为，除非新统一口径要求同步调整并有回归测试。

## Acceptance Criteria

- [ ] Asia/Shanghai 跨 UTC 日界的调用归入本地自然日；至少覆盖一个 DST 时区或明确证明当前实现不依赖固定 offset。
- [ ] KPI、排行、热力图和最近调用都可看见其固定时间口径，且平台筛选后共享同一 source/target 口径。
- [ ] 技能排行每行显示次数、会话数和最近使用；能唯一匹配时显示静态 `SKILL.md` Token 估算，缺失/歧义时不显示伪造数值。
- [ ] 唯一匹配技能可进入现有技能详情；未匹配/歧义技能可打开统计详情，点击不会静默失败。
- [ ] 快速切换平台/target 时旧请求不会覆盖新结果，刷新失败保留缓存时有明确状态。
- [ ] 页面在至少 1280x720、1024x768 和窄桌面宽度下无重叠、截断性溢出或不可达控件；中英文均验证。
- [ ] 键盘可操作所有筛选、排序、刷新和技能行；焦点态可见；热力图/排行不是只靠颜色表达。
- [ ] 不展示 prompt、assistant 回复、tool arguments、auth 信息；项目默认展示 basename。
- [ ] 新增/变更 IPC 均进入 `src/lib/ipc/commandMap.ts`，浏览器 fixture 和按命令名测试同步更新。
- [ ] 对应 Rust 聚合/DB/provider 测试、Vitest 组件/store 测试、`pnpm typecheck`、`pnpm lint`、`cargo clippy -- -D warnings` 和最终 `just ci` 通过。

## Out of Scope

- 复制 codexU 的额度环、API 等效价值、今日/7 日/累计 Token 总览或订阅“羊毛进度”。
- 上传遥测、云端账号聚合或跨设备同步。
- 猜测 Antigravity、Kiro、Zed 未公开的日志格式；这些 provider 仍按真实能力呈现 unavailable。
- 在没有可靠事件关联时把 Skill 调用次数冒充为任务 Token 消耗。
- 工具调用 TOP20、工具分类、工具事件事实表及工具 Token/成本估算；如后续需要，另建 Trellis 任务。
- 自定义日期范围、7/30/90 天切换或跨时区偏好设置；本任务只修正系统本地时区并标清现有固定口径。
