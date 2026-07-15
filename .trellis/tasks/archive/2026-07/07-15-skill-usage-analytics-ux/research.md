# codexU Skill 统计对照研究

## 结论

值得借鉴的是数据语义和信息层级，而不是视觉皮肤。codexU 让一行 Skill 同时回答“用了几次、跨多少线程、来自哪里、多久前使用、Skill.md 多重”；SkillPort 当前只能回答“名字和次数”，但拥有 codexU 不具备的跨 provider、跨 target、项目统计和中央技能导航能力。

推荐路径：保留 SkillPort 的跨平台采集底座，先补正确性、技能元数据与详情闭环；工具调用统计单独规划。

## 能力对照

| 维度 | codexU | SkillPort 当前 | 规划判断 |
| --- | --- | --- | --- |
| 数据源 | Codex + Claude Code 本机 session/transcript | 5 个真实 provider + 3 个 stub，本地/SSH/WSL | 保留 SkillPort 优势 |
| Skill identity | 以解析到的 `SKILL.md` path 为 id | 仅 skill name；同名会合并 | 唯一匹配才关联中央技能，歧义显式化 |
| Skill 行信息 | loads、threads、source、last used、静态 tokens/bytes | count；其余字段只在后端聚合或详情里 | 补齐会话、最近时间、静态体量与匹配状态 |
| 工具统计 | TOP20、分类、calls、估算 tokens | 无工具事件模型 | 建议独立任务 |
| 趋势 | 本地/配置时区，数据质量可解释 | 16 周，UTC 分日，按最大值线性分档 | 修正本地日界；改用分位数/小样本降级 |
| 详情动作 | 信息型 tooltip | 中央详情路由 + 未接通的统计详情 | 打通导航/回退闭环 |
| 诊断 | 缺失/估算解释 | provider health 占半屏 | 保留但降为诊断区 |

## codexU 可复用思想

1. `SkillUsage` 模型把 `loadCount`、`threadCount`、`sourceLabel`、`staticTokenEstimate`、`staticByteCount`、`lastLoadedAt` 放在同一业务对象上（`ref/codexU/Sources/CodexUsageWidget/main.swift:203`）。
2. 静态 Token 是对 Skill.md 内容体量的估算，不是该技能导致的任务消耗；UI tooltip 明确区分（同文件 `:7487`、`:7590`）。
3. 工具 Token 通过 session 总 token 按调用占比分摊，属于估算；无 token 数据时只保留稳定的调用次数（同文件 `:1524`）。
4. Skill load 从 tool payload 中提取 `SKILL.md` 路径并规范化，路径作为稳定 identity，同时允许缓存版本映射（同文件 `:2491`）。
5. 日期统计使用明确的时区上下文，并测试上海跨日及 DST 23/25 小时日（`ref/codexU/Sources/CodexUsageWidget/Domain/StatisticsTimeZone.swift:60`）。
6. 数据缺失不伪造为 0，估算和回退口径在标题旁解释（`ref/codexU/docs/PRD-v0.3.0.md:381`）。

## SkillPort 当前风险

1. `skill_calls.skill` 是唯一技能维度，同名不同来源会被合并；`usage_resolve_skill_id` 又按名称取一个中央记录，可能把统计导航到错误技能（`src-tauri/src/commands/usage.rs:360`）。
2. `SkillUsageDetail`、`loadDetail` 和 `detail` state 存在，但 `SkillUsageView` 不渲染详情；未匹配技能点击无反馈。
3. `list_daily_counts_since` 和单技能 daily query 用 SQLite `strftime(..., 'unixepoch')`，即 UTC 日期；纯函数测试也把 UTC 当契约。
4. 页面同时展示全量 KPI/排行、16 周热力图和最近 20 次调用，但没有显式说明各自时间口径。
5. 排序使用一个循环切换按钮再配独立方向按钮，识别成本高；行内又只显示 count，`projects/sessions/lastUsedMs` 已返回却未展示。
6. provider health 与 top skills 在首屏占用同等栅格权重；对主要任务“判断技能使用结构”帮助较低。
7. 热力图按全局最大值四等分，单日峰值会压扁其余日期；codexU PRD 的 P25/P50/P75/P90 分位数方案更适合长尾使用分布。

## 方案比较

### A. 仅改 UI

复用现有字段，展示 sessions/projects/lastUsed，调整布局。成本最低，但保留 UTC 分日、同名误关联和统计详情断路，不推荐作为完整任务。

### B. 技能统计闭环（推荐）

修正本地日界和请求一致性；扩充 overview/detail 契约；唯一匹配时从当前 target 技能记录读取静态内容并估算 Token；重构排行、详情、趋势与 provider 诊断层级。范围与 SkillPort 产品定位一致。

### C. 同时新增工具/Token 分析

新增工具事件、token delta、归因质量、工具分类、缓存和 UI 双排行。信息更丰富，但需要新的事实表和 provider parser 契约，风险与验证面接近独立子系统，建议另建任务。

## 视觉方向

- 不复制截图中的大面积紫色渐变和圆形额度图；沿用 6 主题 x 14 accent 的语义 token。
- 将排行从“名称 + 粉色条 + 次数”改为紧凑数据行：名称/匹配状态为主，次数/会话/最近使用为稳定列，静态 Token 为次级列。
- 进度条只承担相对量级，不作为唯一信息；排序用明确 menu 或 segmented control。
- 热力图减小无意义留白，增加月份/图例/数值 tooltip，并用分位数分档。
- provider 状态移到折叠诊断区或次级 tab；首屏空间优先给排行与趋势。

## 已确认范围

- 用户确认不纳入工具 TOP20 和工具 Token 估算。本任务按方案 B“技能统计闭环”规划，不需要父子任务拆分；工具/Token 事件分析如有需要另建任务。
