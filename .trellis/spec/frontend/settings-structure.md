# Settings 页面结构约定

> 建立于 2026-07-12（任务 07-12-theme-default-fonts）。目标是维持紧凑、可扫描的产品设置界面，同时保持各业务 section 的行为边界。

## 页面层级

- `SettingsView` 只渲染当前页面一个 `h1` 与 description，不再添加通用 Settings banner。
- `SettingsSideNav` 是桌面纵向、窄窗口横向滚动的扁平导航；active item 使用 `aria-current="page"`。
- 页面内容使用受约束的单一内容列；不得以装饰卡片包住整个 section 或导航项。

## Shared Section 契约

`src/components/settings/SettingsSection.tsx` 是可折叠设置区的共享容器：

```ts
interface SettingsSectionProps {
  sectionId: string;
  title: string;
  description?: string;
  icon?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}
```

- 折叠状态继续写入 `settings.sectionCollapsed.v1`，并保留 `aria-controls`、`aria-expanded` 与 hidden content。
- section 使用 full-width 内容与 hairline divider；禁止恢复 per-section gradient、glow、icon card、decorative bar 或 card-in-card。
- 非 Appearance 页面替换共享容器时，不得顺带改变 children、事件流、字段顺序或 store 调用。

## Appearance 结构

- 固定按 Theme、Language、Typography、Density 分组。
- Display 与 Body 各自提供 Primary、Chinese Fallback、按需 Custom 输入和混排 specimen。
- 选择控件和按钮最小 hit area 40px，必须有可访问名称与可见 focus；窄窗口允许 label/value 换行，但不得产生水平不可达内容。
- Flavor、accent、language、font、scale 的业务语义继续由原 store/API 负责，视觉组件不新增第二套状态模型。

## Tests Required

- `SettingsView.test.tsx` 锁定单一 `h1`、扁平导航、section 折叠持久化、Appearance 控件和非 Appearance 业务行为。
- 视觉变更检查 900x600、1200x800、1440x900；自动化工具不可用时必须明确记录未验证项，不得声明通过。

## Wrong vs Correct

```tsx
// Wrong: section 外再套主题卡片和装饰层
<Card><GradientHeader /><SettingsSection>...</SettingsSection></Card>

// Correct: 页面内容直接由共享 section 组织
<SettingsSection sectionId="github-pat" title={title}>...</SettingsSection>
```
