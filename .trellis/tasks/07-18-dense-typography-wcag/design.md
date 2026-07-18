# Design: Dense typography tokens and WCAG governance

## 1. Boundary

这是纯前端设计系统与回归治理任务。生产变化限定为 CSS token、现有组件 class 和必要的虚拟卡片高度适配；不修改 Zustand store、Tauri IPC、Rust、数据库、导入/安装状态机或 i18n 文案，除非可访问名称确实需要补充。

主要边界：

- `src/index.css`：Tailwind 4 语义字号与现有主题前景 token。
- `src/components/skill/UnifiedSkillCard*.tsx`、`SkillCardMeta.tsx`、`SkillCardBadges.tsx`：高频共享卡片。
- Central / Marketplace / GitHub import / layout / settings / projects / usage / dashboard / logs 中当前数值型 arbitrary 字号命中。
- `src/test/typographyContract.test.ts`（新增）与现有 `fontContract.test.ts`、`themeContrast.test.ts`。
- `.trellis/spec/frontend/font-preferences.md` 和新增/扩展的前端排版规范索引。

技能详情文件只做同尺寸 token 替换。`SkillDetailSidebar` 的顺序、文件分类和状态层级不在本任务设计范围。

## 2. Token model

优先复用 Tailwind 标准阶梯：

| Role | Utility | Default size | Usage |
| --- | --- | ---: | --- |
| Body / action | `text-sm` 或现有组件尺寸 | 0.875rem | 正文、主要控件、需要持续阅读的状态说明 |
| Label | `text-xs` | 0.75rem | section、status、action、table header 等决策相关标签 |
| UI meta | `text-ui-meta` | 0.6875rem | 次要 metadata、ID、路径、keycap、紧凑辅助说明 |
| UI micro | `text-ui-micro` | 0.625rem | 数字计数、heatmap 轴、冗余 chevron/step marker |

`text-ui-meta` 和 `text-ui-micro` 通过 `@theme inline` 或等价 Tailwind 4 utility 声明为 `rem`。它们只定义 font-size，不擅自覆盖现有 `leading-*`；这样默认 Scale=1 可保持现有几何，line-height 继续由拥有布局语义的组件控制。

不要新增 `text-ui-label` 的同义 token：标准 `text-xs` 已表达该层级。不要把当前 11px 一对一替换为 meta；先按角色分类，真正的 label 提升到 `text-xs`，路径/ID/辅助信息才使用 meta。

现有 40 处 arbitrary rem 也必须离开 JSX/TS class。小字号按上述角色迁移；`0.8rem` compact button、`1.05rem` dialog title 等优先落到最接近且不破坏几何的标准阶梯或共享 control token；Dashboard hero/score 等 deliberate display 值若不能无损映射到标准阶梯，则使用 dashboard 所有的命名 component utility。不要为每个字面值增加全局 token，也不要保留 `text-[...]` 数值 allowlist。

## 3. Classification rules

按以下优先级判断：

1. 文本是否触发操作、解释状态、决定冲突/安装/来源或标识 section。是则至少 `text-xs`。
2. 文本是否是可读但次要的路径、ID、快捷键或重复 metadata。是则 `text-ui-meta`。
3. 文本是否是计数、坐标轴、空间受限且上下文已命名的辅助标记。是则 `text-ui-micro`。
4. 纯装饰 glyph 必须 `aria-hidden`；若没有等价可访问名称，不能按装饰处理。

同一 role 在不同 surface 使用同一字号语义。状态重要性通过 weight、foreground、icon 和位置表达，不通过再造 10.5px/11.5px 值表达。

## 4. Contrast model

现有 `themeContrast.test.ts` 是核心，而不是引入浏览器 a11y 依赖。扩展方式：

- 解析六个 theme block 的 background/card/popover/sidebar 与对应 foreground。
- 将 14 accent 覆盖叠加到可读 `primary-text`，并继续区分 fill `primary` / `primary-foreground`。
- 覆盖 success/warning/info/destructive foreground 在实际使用 surface 上的 4.5:1。
- 对 alpha foreground 先与真实 surface 合成再计算；不能拿未合成的 base token 比率替代结果。
- 静态测试只证明 token pair。组件审计仍需确认 class 实际落在所测 surface 上。

有意义文本默认不使用 `/60`、`/70`、`/80` 透明前景色。disabled 或纯装饰例外必须同时具有不可操作语义/等价 label，并记录在 research 决策表；不建立按行号 allowlist。

## 5. Migration slices

### Slice A: contract first

1. 固化 planning 173 项、task-start inventory/delta 和 22 项 alpha-risk 表。
2. 增加 token 与 `typographyContract.test.ts`，先让“生产 TS/TSX 禁止任何 `text-[...]`”测试失败。
3. 扩展 contrast helper，先覆盖即将使用的 token/surface 对。

### Slice B: high-frequency and safety-critical

1. `UnifiedSkillCard`、meta/badges/footer 与 Central shell/sidebar/filter/menu。
2. GitHub import wizard chrome/body/preview/file tree，确保 conflict、selection、source path、status 都不是 micro。
3. App shell、target switcher、remote settings，确保导航/凭据/错误说明可读。

### Slice C: remaining surfaces

Marketplace、Projects、Usage、AI settings、platform grouping 与低风险计数/axis。最后处理技能详情中的默认尺寸等价替换，不改变结构或信息权重。

每个 slice 后运行 focused tests 和 inventory，避免一次机械替换后再猜语义。

## 6. Font scale and virtualization

固定 px 迁移为 rem 后，三档 Scale 都必须真实影响文本。验证分两层：

- DOM/CSS contract：在 0.875 / 1 / 1.125 下确认 computed font-size 随根字号变化，且 micro < meta < xs <= sm。
- Layout contract：使用超过阈值的 Central fixtures，检查每个绝对定位 virtual row/cell 的 bounding rect 不与相邻项相交，卡片 scrollHeight 不超过分配高度，首末项可达。

`VirtualizedList` / `VirtualizedGrid` 当前是固定高度实现。先保持其算法、阈值和 overscan；若 1.125 实测失败，只调整 Central 提供的 item height 或与字号档关联的最小高度。只有固定高度模型无法满足三档时，才另行提出动态测量虚拟器，不能在本任务内无证据重写共享组件。

## 7. Performance shape

- token 在构建期生成 CSS utility，不增加组件状态、effect、observer 或事件监听。
- inventory/no-growth detector 只在 Vitest/CI 读取源码，不进入生产 bundle；它禁止整个 `text-[...]` class 家族，不允许用 arbitrary rem/color 绕过。
- class 迁移不得改变列表虚拟化阈值、overscan、数据请求或 store selector。
- QA 记录 scale 切换前后可见 virtual item 数和滚动高度；只验证无异常增长，不设置脆弱的精确毫秒阈值。

## 8. Compatibility and rollback

- 无持久化或数据库迁移；已有 Font Scale setting 继续生效。
- token 值以 rem 表达，Scale=1 下 micro/meta 与原 10/11px 等价，因此可按 slice 回滚。
- deliberate display 值通过命名 component utility 保持原几何；回滚 class 时不得重新引入 arbitrary syntax。
- 如果 label 提升到 12px 造成局部溢出，优先修复容器换行/截断约束；不能退回任意 px。若它实际不是 label，应重新分类为 meta，而不是新增第三个近似字号。
- 对比度修复只替换 foreground role，不改变 theme/accent 身份或背景结构。
