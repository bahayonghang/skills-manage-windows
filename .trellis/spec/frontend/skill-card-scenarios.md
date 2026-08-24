# 技能卡片显式场景 interface 约定

> 建立于 2026-07-05（任务 07-04-skill-card-scenarios）。背景：`UnifiedSkillCard` 曾是约 40 个扁平可选 props 的 shallow module，5 类场景由 prop 组合隐式决定（如可点击变体 = `onClick && !hasActions && !hasCheckbox && !hasPlatformIcons`），每个调用点面对全部 40 props 的组合空间，映射知识由调用方携带。

## 约定 1：卡片调用必须声明 `variant`，props 按场景收窄

**What**：`src/components/skill/UnifiedSkillCard.tsx` 的 props 是判别联合，调用方声明场景名 + 该场景的窄 props；跨场景 props 在编译期被 excess property check 拒绝。

**签名**：

```ts
export type UnifiedSkillCardProps =
  | CentralSkillCardProps // variant: "central"     中央库全功能管理面（唯一构造方：buildCentralSkillCardProps）
  | PlatformSkillCardProps // variant: "platform"    平台已装技能（来源类型 + 装/卸该平台 + lifetimeUsage + installOrigin）
  | ProjectSkillCardProps // variant: "project"     项目技能（来源徽章 + 卸载）
  | ImportSkillCardProps // variant: "import"      Obsidian vault 导入候选（原 discover 簇）
  | MarketplaceSkillCardProps // variant: "marketplace" 远程技能浏览/安装
  | CollectionSkillCardProps // variant: "collection"  集合成员
  | SkillsCliSkillCardProps; // variant: "skillsCli"   Skills CLI 全局技能（卸载确认由页面完成）
```

平台场景可有 `lifetimeUsage?: { rank: number | null; count: number }`（全历史当前列表名次）。`undefined` = 未就绪，不画右下角；`rank < 1` 或 `null` = 「无记录」；`rank >= 1` = `#N · count`。来源竖条由 `toModel` 从 `originKind` / `installOrigin` / `sourceType` 派生（plugin / central / skillsCli）。**`link_type === "symlink"` 不再等于 Central**：Skills CLI junction/symlink 必须传 `installOrigin: "skillsCli"`。不要把 `statusAccent` 挪来表达来源。

**Why**：单场景可见 props 从 40 收敛到 9–23；隐式组合判定内化后，调用点不再可能拼出无名义的 prop 组合。「唯一卡片实现」约束不变——一个 module 一份渲染实现。

**违反成本**：绕过 variant 重建内联卡片组件 = 复活第二套卡片实现；把场景 A 的 prop 塞进场景 B 的成员 = 互斥保证失效，回到组合空间爆炸。

## 约定 2：渲染代码只面向模块私有 `SkillCardModel`

**What**：组件内部经 `toModel(props)` 把各场景归一化为扁平渲染模型（不导出）；渲染树不感知 `variant`。给某场景加 prop 的固定动作序列：

1. 在对应场景成员 interface 加字段（跨场景复用需先论证是否进 core——core 仅 `name/description/aiSummary/className`）；
2. 在 `toModel` 对应分支拷贝该字段；
3. 渲染层从 model 解构使用。

**Why**：场景→渲染的映射只存在于 `toModel` 一个点；渲染逻辑改动无需理解 7 个场景。

**违反成本**：在渲染层直接 `props.variant === …` 分支 = 场景知识二次泄漏；导出 `SkillCardModel` = 判别联合被旁路。

## 约定 3：互斥负例是活文档，必须随场景演进维护

**What**：`src/test/components/skill/unifiedSkillCardVariants.test.tsx` 持有每场景最小正例 + 跨场景 `@ts-expect-error` 负例（带中文描述）。`pnpm typecheck` 双向强制：互斥失效时 directive 报 Unused 错误。新增场景 / 移动 prop 归属时同步增删负例。

**Why**：类型互斥是本约定的核心交付物，负例是它唯一的回归探测器。

## 参考

- 设计依据与调用点矩阵：`.trellis/tasks/archive/2026-07/07-04-skill-card-scenarios/design.md`（归档后路径）
- central 场景 props 唯一构造方：`src/components/central/centralSkillCardProps.ts`
