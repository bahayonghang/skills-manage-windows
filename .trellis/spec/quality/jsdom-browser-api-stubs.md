# jsdom 浏览器 API 替身约定（matchMedia 等）

> 建立于 2026-08-18（任务 08-18-small-screen-card-grid）。响应式布局把 `useMediaQuery`
> 引入组件后，页面级测试批量失效，排查出两条可复用教训。

## 约定 1：媒体查询按"非默认分支"表述

**What**：`src/test/support/setup.ts` 的 matchMedia polyfill 恒返回 `matches:false`。
写组件里的查询时，让 `false` 落在期望的默认（通常是宽屏/桌面）分支。

```ts
// Correct：jsdom 默认 false → 宽屏，既有 pinned 展开语义不变
const isNarrowViewport = useMediaQuery("(max-width: 1399px)");
const isPinned = isPinnedPreference && !isNarrowViewport;

// Wrong：jsdom 默认 false → 窄屏，所有依赖宽屏布局的既有测试批量失效
const canPin = useMediaQuery("(min-width: 1400px)");
```

**Why**：页面级测试（CentralSkillsView.* 等）不经组件层 mock，直接吃 polyfill 默认值；
查询表述方向决定要不要动几十条既有用例。

## 约定 2：改 matchMedia 行为用 `vi.stubGlobal`，不用 `vi.spyOn`

**What**：需要命中查询的用例：

```ts
vi.stubGlobal("matchMedia", (query: string) => ({
  matches: true, media: query, onchange: null,
  addListener: vi.fn(), removeListener: vi.fn(),
  addEventListener: vi.fn(), removeEventListener: vi.fn(),
  dispatchEvent: vi.fn(),
}));
// 用例所在 describe 的 beforeEach/afterEach：vi.unstubAllGlobals()
```

**Why**：`vi.spyOn(window, "matchMedia")` + `mockRestore`/`restoreAllMocks` 会把
setup 里的 `vi.fn(implementation)` polyfill 一并重置成无实现的 noop，
之后 `matchMedia()` 返回 `undefined`（已用探针用例实证）。
`stubGlobal`/`unstubAllGlobals` 做的是属性值替换，原 polyfill 实现原样恢复。

**注意**：`useSyncExternalStore` 的 subscribe 会调用 `addEventListener`/`removeEventListener`，
stub 对象必须带这两个方法（themeStore 测试里常见的 `{ matches } as MediaQueryList`
最小替身只适用于不订阅的同步读取场景）。

## 约定 3：Tailwind 变体组合要验证产物 CSS

**What**：新增非常规变体组合（如容器查询叠 named group：
`@max-[22rem]:group-hover/skill-card:flex`）后，构建并 grep `dist/assets/*.css`
确认规则真的生成了。

**Why**：Tailwind 对无法解析的类名静默跳过；且组合顺序会改变条件规则嵌套
（`@media (hover:hover)` 可能嵌进 `@container` 内），肉眼无法从类名推断产物。
