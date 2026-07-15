# 技能使用统计闭环设计

## 1. 设计目标

本任务解决的不是“再做一个更漂亮的 dashboard”，而是让 SkillPort 的技能使用统计成为可信的操作入口：统计日期正确、技能身份不乱跳、静态体量有明确口径、筛选结果不会串台，并且每条排行都能进入统计详情或已确认的中央技能。

任务是一个紧耦合的跨层交付：事实采集、派生元数据、IPC 契约、store 一致性和页面交互必须一起验收，因此不拆父子任务。工具调用统计已确认不在范围内。

## 2. 核心决策

### D1. `skill_calls` 继续只保存事实

现有 `skill_calls` 保持为 provider 归一后的调用事实，不增加 `skill_id`、Token 或文件路径列。日志能稳定证明的是技能名、时间、项目、会话和 provider；把后续解析结果写回每条事实会产生重复数据，也会把“当时调用了什么”与“当前中央库如何解析这个名字”混为一谈。

### D2. 新增 target 级派生元数据缓存

新增 `skill_usage_metadata` 表，保存一次成功 usage scan 后对技能名的保守解析结果：

| 列 | 类型 | 语义 |
| --- | --- | --- |
| `target_id` | TEXT | 与 `skill_calls.target_id` 一致 |
| `skill` | TEXT | provider 日志中的原始技能名 |
| `match_status` | TEXT | `matched` / `ambiguous` / `unmatched` |
| `resolved_skill_id` | TEXT NULL | 仅 `matched` 时存在的中央技能 id |
| `static_token_estimate` | INTEGER NULL | 可读取且未超预算时的 Skill.md 静态估算 |
| `static_byte_count` | INTEGER NULL | 同一文件的 UTF-8 字节数 |
| `scanned_at_ms` | INTEGER | 派生时间 |

主键为 `(target_id, skill)`；`resolved_skill_id` 不设外键。原因是 `skills` 表是当前 active target 的库存视图，而 usage 缓存需要在远程 target 暂时不可达时继续展示上次成功结果。

`replace_calls_for_target` 扩展为同一事务内替换 calls、provider health、metadata 和 scan state。任何持久化失败都回滚到上一份完整缓存；单个 Skill.md 读取失败只让该行静态指标为 `NULL`，不让整次 usage scan 失败。

### D3. 保守解析中央技能身份

解析只针对 `skills.is_central = 1`，不把任意平台观察行当成中央技能：

1. 对日志名执行 `trim + lowercase`，先匹配中央技能 `id`；id 唯一时直接 `matched`。
2. 没有 id 命中时，再按规范化后的 `name` 匹配。
3. name 恰好一个候选时 `matched`；多个候选时 `ambiguous`；没有候选时 `unmatched`。
4. `ambiguous` / `unmatched` 不生成跳转 id，也不读取任意候选文件。

这取代 `usage_resolve_skill_id` 当前“按 name 排序后取第一条”的不稳定规则。兼容命令可以保留，但只能从 metadata/cache 或同一 resolver 返回唯一 id，不能再静默猜测。

### D4. 静态 Token 是文件体量，不是消耗归因

成功匹配的中央技能通过当前 `Scope::fs_backend()` 批量读取 `file_path`：本地走现有 `run_blocking_fs_with`，SSH/WSL 走 `RemoteFsBackend::read_many_to_strings`，不新增第二套传输实现。读取结果先经过 `ResourceBudget::default_skill().reject_file_read_size`；超预算、缺失、不可读或非 UTF-8 都记录为静态指标不可用。

估算沿用 codexU 的轻量语义：非空白、非 CJK 字符约每 3.8 个计一个 token，CJK 字符按 1:1，结果向上取整；空文件为 0。UI 固定使用“Skill.md 估算”文案和解释 tooltip，禁止称为“技能消耗 Token”。

### D5. 本地自然日按每条事件动态换算

删除 SQL `strftime(..., 'unixepoch')` 的 UTC 分组。repo 层只按 target/source/cutoff 读取 `timestamp_ms`，usage 聚合层逐条执行 `DateTime<Utc>.with_timezone(&Local)` 后取 `date_naive()`，再生成 16 周网格。

实现拆成可注入的 day resolver：生产 resolver 每条事件查询系统 Local offset；测试 resolver 可在指定时间点切换 offset，证明实现不捕获一个固定 offset。网格锚点使用系统本地今天，cutoff 多取一天以覆盖 UTC 与本地日界偏移，最终仍只输出连续 112 天。

### D6. 不新增全局时间范围筛选

本任务保持现有固定范围并把口径写清楚：

- KPI 与技能排行：全部已记录历史。
- 活动热力图：最近 16 周。
- 最近调用：最新 20 条。
- Central/Platform 技能卡徽标：继续使用现有近 30 天口径。

这样能修复“看不出范围”的问题，又不把任务扩张为新查询矩阵。自定义日期范围另行规划。

## 3. 数据流

```text
provider logs
  -> Vec<SkillCall> facts
  -> distinct skill names
  -> central candidate query
  -> conservative resolver
  -> Scope/FsBackend batch Skill.md reads
  -> static metrics + match status
  -> one transaction: calls + providers + metadata + scan state
  -> overview/recent/detail queries join metadata
  -> typed IPC
  -> usageStore guarded state
  -> ranking / heatmap / inline detail / source diagnostics
```

远程连接失败时不运行替换事务，继续使用该 target 上一次完整缓存；metadata 与 calls 因同事务写入，不会出现“新调用 + 旧身份”混合状态。

## 4. 后端契约

### 4.1 业务类型

新增或重命名为语义明确的类型：

```text
UsageSkillMatchStatus = matched | ambiguous | unmatched

SkillUsageSummary
  skill, count, projects, sessions, lastUsedMs
  matchStatus, resolvedSkillId?
  staticTokenEstimate?, staticByteCount?

RecentSkillCall
  skill, timestampMs, project, sessionId, source
  matchStatus, resolvedSkillId?

SkillProjectCount
  project, count, sessions, lastUsedMs

SkillUsageDetail
  skill, count, sessions, firstUsedMs, lastUsedMs
  byProject: SkillProjectCount[]
  weekly: DayCount[]
  matchStatus, resolvedSkillId?
  staticTokenEstimate?, staticByteCount?
```

`UsageOverview.topSkills` 改为 `SkillUsageSummary[]`。默认 overview 返回全部已记录技能，使“次数 / 最近 / 名称”排序覆盖完整集合，不再只是把数据库 TOP50 重新排字母序。

### 4.2 查询边界

- `usage_repo.rs` 增加中央候选批量查询、metadata CRUD/join、时间戳查询。
- `aggregate.rs` 只保留纯聚合：KPI、排行映射、local-day bucket、16 周网格。
- 新建小型 `services/usage/enrichment.rs`，集中身份解析、静态指标估算和 metadata 构建；provider 不知道数据库或中央技能。
- `usage_get_skill_detail` 增加可选 `source`，项目分布正确统计 `COUNT(DISTINCT session_id)`；详情与当前平台筛选一致。
- `usage_resolve_skill_id` 保留兼容面，但复用保守 resolver/metadata；前端排行不再逐行点击时额外 invoke。

## 5. 前端状态设计

`usageStore` 增加两类状态：

- 选择状态：`selectedSource`、`selectedSkill`。
- 数据质量状态：`usedCachedData`、`refreshError`、`loading`、`refreshing`、`lastRefreshMs`。

平台筛选使用独立 request sequence：一次选择同时请求 overview 与 recent，只有 target id、source 和 sequence 都仍匹配时才原子提交。快速点击平台、target 切换或 refresh 都会使旧 sequence 失效。

refresh 在未筛选状态可直接提交返回的 overview/recent；已有平台筛选时先更新 provider/scope/freshness，再重拉该 source 的 overview/recent，期间保留旧筛选数据，避免短暂闪回“全部平台”。切换 source 时清空当前 detail，详情请求同样带 source 并受 sequence 保护。

`showingCachedAfterError` 增加到正确的 `skillUsage` i18n namespace；远程不可达文案根据是否有缓存区分“显示上次数据”和“暂无可用数据”。

## 6. 页面与交互

### 6.1 信息架构

```text
[Skill Usage] [Local/Remote]                    [全部已记录] [刷新时间] [刷新]
[全部平台] [Claude Code] [Codex CLI] ...

调用次数 | 技能数 | 项目数 | 数据源/会话数       <- 单一紧凑指标带，不做四张渐变卡

┌ 技能排行（主列） ─────────────────┐  ┌ 最近 16 周活动 ───────────┐
│ 次数 / 最近 / 名称 segmented       │  │ 月份、Mon/Wed/Fri、图例    │
│ 名称  匹配  会话  最近  估算体量   │  ├ 统计详情或最近调用 ──────┤
│ ...                                │  │ 选择技能后原位显示详情     │
└────────────────────────────────────┘  └───────────────────────────┘

[数据源诊断 5/8 available · 最近扫描]  <- disclosure，展开后是 provider health
```

视口达到 `xl` 才使用主从两列；1024px 与更窄桌面使用单列，DOM 顺序为排行、所选详情、活动、最近调用、数据源诊断。固定面板使用 `minmax`、稳定最小高度和滚动区，动态文案不改变工具栏或图表尺寸。

### 6.2 技能排行

- 用新的 `SkillUsageTable` 替代名不副实的 `SkillBarChart`。
- 排序用三个显式 segmented buttons：最多使用、最近使用、名称；名称可配方向图标，其余使用确定的降序。
- 每行主信息是技能名；稳定数值列是调用数、会话数、最近使用；项目数和 Skill.md 估算作为次级信息。
- 相对条可保留为低对比背景刻度，但不能替代数字。
- `matched` 显示可识别的中央技能动作；`ambiguous` / `unmatched` 使用中性文字状态，不用警告色制造错误感。
- 行本身选择统计详情并设置 `aria-selected`；“打开技能”是独立 icon button，使用 lucide 图标和 tooltip，避免嵌套 button。

### 6.3 统计详情

新建 `SkillUsageDetailPanel`，不是 modal：

- 标题、匹配状态、调用数、会话数、首次/最近使用。
- 项目分布紧凑列表，默认只显示 basename，完整路径不进入默认 tooltip。
- 单技能 16 周活动图复用 heatmap 的可访问核心。
- matched 时显示“打开技能”命令；关闭/返回按钮恢复最近调用面板，并把焦点还给触发行。

### 6.4 热力图与诊断

- 热力图色阶按非零日 P25/P50/P75/P90；非零日少于 5 天时线性降级。
- 增加月份标签、低/高图例、调用数 tooltip、`aria-label` 与 roving focus；颜色之外始终有数值文本。
- 使用主题 `chart-*` / semantic tokens，不硬编码绿色；尊重 reduced motion。
- provider health 降为底部 disclosure，但保留平台选择联动、available/0/not detected 的现有语义。

## 7. 加载、空态与错误

- 首次加载：按最终布局显示稳定 skeleton，不以全页 spinner 替代内容。
- 手动刷新：保留旧内容，刷新按钮显示进行中；成功后更新扫描时间。
- 有缓存的失败：展示非阻塞 freshness 提示和上次扫描时间。
- 无缓存的失败：展示页面级错误和可重试动作。
- 无调用：说明未在已支持 provider 日志中发现 Skill 调用，并指向刷新/检查数据源诊断。
- 静态文件不可读：只在该行显示“Skill.md 不可用”，不升级为页面错误。

## 8. 兼容与迁移

- 数据库变更是 additive `CREATE TABLE IF NOT EXISTS`；旧 DB 首次打开无需数据迁移，下一次成功 usage scan 自动填充 metadata。
- `skill_calls`、provider parser、近 30 天卡片徽标和 5 分钟 scan TTL 保持不变。
- `usage_resolve_skill_id` 保持命令名，避免已有 fixture/测试和潜在调用方断裂；返回语义收紧为“仅唯一匹配”。
- 浏览器演示 fixture 必须包含 matched、ambiguous、unmatched、静态指标缺失、缓存失败五类状态。
- 不新增 npm/Cargo 依赖；本地时区使用现有 `chrono`，视觉使用现有 React/Tailwind/lucide 组件。

## 9. 风险与控制

| 风险 | 控制 |
| --- | --- |
| 同名误跳中央技能 | id 优先、name 唯一才匹配；其余显式 ambiguous/unmatched |
| 远程读取拖慢刷新 | 只读唯一 matched 的 distinct Skill.md；复用批量 FsBackend 与 64 文件 chunk |
| 巨大 Skill.md 占用资源 | 复用 `ResourceBudget::default_skill`，超预算仅令静态指标不可用 |
| DST/午夜归日错误 | 每事件动态 Local offset；可注入 resolver 测试 offset 切换 |
| 快速筛选串台 | target/source/sequence 三重提交守卫，overview+recent 原子 set |
| 新 UI 变成 SaaS 指标卡 | 单一 metric strip、紧凑排行、诊断降级，不复制 codexU 皮肤 |
| 既有 Cargo/package 脏改动冲突 | 本方案不新增依赖；实施时逐文件保留用户现有修改，不重写配置文件 |

## 10. 回滚策略

前端可以先回退到旧 panels 而不删除数据库表；新 metadata 表是派生缓存，无业务写入依赖。若 enrichment 导致扫描性能回退，可暂时停止构建 metadata，overview 会以 `unmatched + NULL static metrics` 降级，calls/provider health 仍可工作。禁止通过删除用户已有 usage 数据回滚。
