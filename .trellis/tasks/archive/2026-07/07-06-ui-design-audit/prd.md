# UI 设计审查落地：调度台一致性优化

## Goal

落地 2026-07-06 双 agent UI 设计审查的结论：界面整体健康（Nielsen ≈28/40，无 P0，token 体系在主产品面执行干净），主要失分集中在**一致性**——两套卡片惯用法、两套按压语言、焦点环覆盖不均、一处绕过 token 体系的彩虹色。本任务把四条线收敛回 DESIGN.md 既有承诺，不引入新设计方向。

## 审查来源（Confirmed Facts）

- 双 agent 评审：Assessment A（设计总监视角，六角度：交互/细节/样式/排版/a11y/i18n）+ Assessment B（impeccable detect.mjs 确定性扫描）。快照存于 `.impeccable/critique/`（slug `src`）。
- Nielsen 评分 ≈28/40；最低分：#4 一致性 = 2、#10 帮助文档 = 2；无 P0（无阻断任务缺陷）。
- 检测器 20 处命中中约 14 处误报，**主产品面（UnifiedSkillCard、弹层、各页面）零命中**；真实命中集中在启动闪屏等边缘 chrome。
- AI slop 判定：可信、不 slop——等宽正文、主题 token 换肤、`transition-[具体属性]` + `active:scale-[0.96]` 纪律都是真诚工艺。
- 关键正面样板（后续修复须向它们看齐）：`src/components/ui/card.tsx:15`（卡片签名标准答案）、`src/components/ui/inline-confirm-action.tsx`（两段式就地确认）、`src/index.css:137-169`（reduced-motion 成熟降级）、`UnifiedSkillCard.tsx:451`（checkbox 40px 命中区扩展）。

## 子任务地图

| 子任务 | 优先级 | 范围 |
| --- | --- | --- |
| `07-06-ui-semantic-status-color` | P1 | SourceMeta 彩虹原生调色板回收；`statusAccent` API 语义化；`destructive/5` 漂移统一 |
| `07-06-ui-keyboard-focus-a11y` | P1 | 手写按钮共享 focus-visible 环；平台 toggle 去颜色单独编码；Dialog "Close" i18n；InlineConfirm Esc 取消 |
| `07-06-ui-surface-language` | P2 | 圆角越界收敛（rounded-3xl / 24px+）；border→ring 统一；dashboard 外装饰性玻璃回收；DESIGN.md 登记例外 |
| `07-06-ui-micro-interaction-polish` | P3 | 按压语言统一与漂移值归一；transition-all 清除；10px 标签托底；accent 降噪 |
| `07-06-ui-help-shortcuts-sheet` | P2 | 应用内快捷键速查浮层（`?` / `mod+/` 唤起）+ 集中式快捷键清单（决策 ③） |
| `07-06-ui-dashboard-metric-strip` | P3 | MetricStrip 4 张等权卡弱化为单行紧凑统计条，保留跳转与 tabular-nums（决策 ①） |

子任务间无实现依赖，可独立开工；建议按优先级 P1 → P2 → P3 排。

## 跨子任务验收（父任务 AC）

- [ ] 6 个子任务全部完成并归档。
- [ ] `just ci` 绿（全量门禁）。
- [ ] 主题回归抽查：至少 Mocha / Latte / Claude Light / Claude Dark 4 套主题 × 2 种 accent 下，中央库、技能详情、Settings、Dashboard、更新中心（SourceMeta 所在）目视无违和。
- [ ] DESIGN.md 已更新：所有保留的例外（dashboard 玻璃面板圆角、浮动操作条 overlay、HealthOrbit tracking 等，若保留）均有明文登记；无登记则视为未收敛。
- [ ] DESIGN.md 已明文记录「Dashboard 高密度为有意定位」（决策 ②，由 `07-06-ui-surface-language` 承载）。
- [ ] 复跑 `$impeccable critique src`，一致性（Nielsen #4）不低于 3 分。

## 产品决策（2026-07-06 用户确认，按推荐执行）

1. **MetricStrip 弱化为单行紧凑统计条** → 子任务 `07-06-ui-dashboard-metric-strip`（腾出的纵向空间不新增内容）。
2. **Dashboard 密度维持现状**（PRODUCT.md 高密度调度台定位使然，"单屏 8+ 区超出单一焦点"属有意张力），并把这条定位明文写进 DESIGN.md → 并入 `07-06-ui-surface-language` 的 DESIGN.md 更新。
3. **补应用内快捷键速查**（Nielsen #10 = 2 分的针对性补强；只做速查浮层，不做完整帮助中心）→ 子任务 `07-06-ui-help-shortcuts-sheet`。

## Out Of Scope

- 信息架构改版、页面重排、新功能。
- 后端（Rust/IPC）改动。
- UnifiedSkillCard 的 `.central-skill-card-surface` box-shadow 描边实现重写（与 `ui/card.tsx` 的字面 ring 是两种实现，暂列观察）。
