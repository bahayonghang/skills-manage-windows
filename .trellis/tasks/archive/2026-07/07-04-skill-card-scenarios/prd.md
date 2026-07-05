# UnifiedSkillCard 显式场景 interface

## Goal

在**不破坏「唯一卡片实现」约束**的前提下，把 `UnifiedSkillCard` 的 interface 从约 40 个扁平 props 收窄为命名场景（central / platform / project / marketplace / collection）+ 各场景窄 props，隐式的场景组合判定吸进模块内部——interface 缩水，implementation 吸收组合。

## 背景与证据（2026-07-04 架构评审）

- `src/components/skill/UnifiedSkillCard.tsx` — 717 行，**~40 个顶层 props**（仅 `name` 必填、39 个可选）、**11 个动作回调**、**3 个嵌套配置对象**（platformIcons 6 字段、editableTags 5 字段、footer 2 字段）。
- 场景由 prop 组合**隐式**决定：`:256` 平台可点击变体 = `onClick && !hasActions && !hasCheckbox && !hasPlatformIcons`；其中 `hasActions` 是 10 个回调的 OR（`:214-225`）。5 个场景没有名字，映射知识由调用方携带。
- **9 个**非测试调用点，每个都要面对全部 40 props 的组合空间——渲染一张卡「必须知道的事」接近 5 张卡片之和，模块是 shallow 的（interface 逼近 implementation 复杂度）。

## Requirements

1. interface 重设计为显式命名场景：调用方声明场景名 + 该场景的窄 props；场景间互斥的 props 不再同时暴露。具体形状（单 `variant` 判别联合 / 场景专属 props 对象等）在 design 阶段做方案比较后裁决。
2. 隐式组合判定（`:256` 一类）全部内化进模块。
3. 9 个调用点全部迁移到新 interface。
4. 渲染结果零视觉变化：统一样式（`rounded-xl` + `ring-1 ring-border` + `bg-card` + `shadow-sm`）、platformIcons 双行分组、既有交互全部保持。

## Constraints

- **唯一卡片约束不变**（CLAUDE.md / CONTEXT.md）：仍是一个 module 一份 implementation，禁止拆成 5 个卡片组件。
- i18n、statusTone、`togglePlatformLink` 走 `centralSkillsStore` 的现状保持。
- TypeScript 类型必须让「传了场景无关 props」在编译期报错（interface 收窄要由类型系统强制，不靠约定）。

## Acceptance Criteria

- [ ] 新 interface 下，单个调用点可见的 props 面显著缩小（硬指标由 design 定，如：单场景必知 props ≤ 15）。
- [ ] 传场景无关 props 会 typecheck 失败（以负面用例证明）。
- [ ] 9 个调用点全部迁移，无兼容垫层残留。
- [ ] 既有卡片相关测试全过，视觉零回归（现有组件测试 + 抽样页面测试锁定）。
- [ ] `pnpm test`、`pnpm typecheck`、`pnpm lint` 全过。

## Notes

- 复杂度：complex（interface 重设计）→ 需 `design.md` + `implement.md`；design 阶段建议对 interface 形状做 design-it-twice（至少两个方案对比 depth / locality / 迁移成本）。
- 与其它子任务无依赖，可独立推进。
