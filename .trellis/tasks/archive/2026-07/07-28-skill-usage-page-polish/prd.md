# Skill Usage 页面体验优化：扫描加载态、布局空白、安装筛选

## 背景

用户在真实环境（928 次调用 / 57 技能 / 764 会话）使用 `/usage` 页面时反馈三类问题，
另经代码走查发现两处伴生缺陷。本任务为纯前端优化，不改后端 IPC 契约。

## 问题清单（用户报告 + 走查发现）

### P1 用户报告

1. **进入页面无「扫描中」加载反馈**
   - 首次进入触发 `usage_refresh` 全量扫描各平台会话日志（可能持续数秒），
     现有 `UsageSkeleton` 只是通用灰块，无「正在扫描」语义；
   - 骨架期间顶部 KPI 条仍渲染真实组件并显示 4 个 `0`，观感像坏数据。
2. **大片空白**
   - Top skills 卡片 `xl:row-span-3` 被右列（RecentCallsFeed 20 条无内部滚动）拉高，
     而卡片内表格固定 `h-[32rem]`，卡片下半部出现整片空白（截图主诉）；
   - 宽屏下 KPI 条 `grid-cols-4` 均分全宽，数字间大段留白；
   - 「All recorded history」文案在页头副标题、KPI 条脚注、Top skills 卡片头出现 3 次。
3. **无法只看「当前已安装」的技能**
   - 历史日志包含已删除技能（表现为 Unmapped），无筛选项；
   - 数据已具备：每行 `matchStatus`（matched/ambiguous/unmatched）与 `resolvedSkillId`。

### P2 走查发现（伴生修复）

4. **切换 target 后旧数据滞留**：`subscribeTargetChanged` 重置了 scope/selectedSource
   等，但未清 `overview/recent/providers`，重扫期间仍展示上一台机器的数据，违背
   spec「所有可见面板始终来自同一 target/source」的目标。
5. **匹配状态列可扫读性差**：Matched/Unmapped 为同色纯文本，无法快速区分。

## Requirements

### R1 扫描加载态

- 首次加载（`overview === null && refreshing`）时展示**最终布局形态的骨架**
  （spec 要求 stable final-layout skeleton），骨架需覆盖 KPI 条区域（不得渲染 0 值）；
- 骨架区域内展示「正在扫描各平台会话日志…」提示 + 动画（纯 CSS，如 spin/pulse）；
- 切换 target 后清空 `overview/recent/providers`，使重扫期间同样进入骨架态。

### R2 排版与空白修复

- Top skills 表格填满其卡片高度（内部滚动），消除卡片内空白；
- RecentCallsFeed 增加高度上限 + 内部滚动，不再无限拉高整行；
- KPI 条改为紧凑排布（数字聚拢，不再均分全宽），删除条内重复的范围脚注；
- 「全部历史 / 16 周 / 最近 20 条」三个固定范围标签仍必须在 UI 可见（spec 契约）。

### R3 安装状态筛选

- Top skills 表头新增三段筛选：全部 / 已安装（matched）/ 未关联（ambiguous+unmatched）；
- 筛选为**视图本地状态**（同现有 sortMode），不进 store、不发新请求（纯客户端过滤）；
- 计数行体现过滤效果（如「12 / 57 skills」）；过滤后空集展示明确空态与切回提示；
- Recent calls 列表不做筛选（本期范围外，见「非目标」）。

### R4 匹配状态可视化

- 表格「Central match」列与详情面板的匹配状态文本增加语义色点/徽标，
  必须走 `src/lib/statusTone.ts`（matched=success，ambiguous=warning，unmatched=中性灰），
  禁止直接写 Tailwind 调色板或 `dark:` 二元适配。

## 非目标

- 不改任何 Rust 后端 / IPC 命令签名（filter 纯前端）；
- 不筛选 Recent calls feed、不动 Provider health 折叠区结构；
- 不新增「删除历史记录」类数据清理功能；
- 不改热力图组件。

## Acceptance Criteria

- [x] 冷启动进入 `/usage`（DB 无缓存）：可见含扫描文案与动画的整页骨架，KPI 区域无 `0` 闪现
- [x] 宽屏（≥1280）下 Top skills 卡片内无成片空白；Recent calls 超长时在自身区域内滚动
- [x] 筛选切到「已安装」后表格只剩 `matchStatus === "matched"` 行，计数行显示过滤前后数量；
      切到「未关联」只剩 ambiguous/unmatched 行；空集有空态文案
- [x] 切换 target 后不出现上一 target 的数据，而是骨架 → 新数据
- [x] 匹配状态带语义色（statusTone），亮/暗主题下均可读
- [x] 中英文案齐备（`src/i18n/locales/{en,zh}.json`）
- [x] `pnpm typecheck`、`pnpm lint`、`pnpm test` 全绿；usage 相关组件/store 测试覆盖新行为
      （筛选、骨架、target 切换清空）
