# 前端平台分组与多选约定

> 建立于 2026-07-05（任务 07-04-frontend-platform-module）。背景：「Universal Agents 是一个虚拟分组」的领域知识曾散布为 3 份有序 ID 列表 + 4 份对话框重复多选实现 + ~18 处组件三元分支，新增平台最多要改 4 处、漏一处即静默渲染为独立平台。

## 约定 1：平台分组唯一登记点 `src/lib/platformRegistry.ts`

**What**：Universal 分组成员、成员顺序、install 代表偏好序、默认启用平台，全部只登记在 `platformRegistry.ts` 一个文件里。

**签名**：

```ts
// 一行一个 universal 成员，数组顺序即展示顺序
interface UniversalPlatformRegistration {
  id: string;
  globalGroup: boolean;        // true = 全局 Universal 分组成员；false = 仅项目场景成组
  installPreference?: number;  // install 代表偏好序（1 最优先）；undefined = 不作候选
}
export const UNIVERSAL_PLATFORM_REGISTRY: readonly UniversalPlatformRegistration[];
// 推导导出（其他文件只 import，禁止再写字面量列表）：
export const UNIVERSAL_AGENT_ID_ORDER: readonly string[];          // 表序过滤 globalGroup
export const UNIVERSAL_PROJECT_AGENT_ID_ORDER: readonly string[];  // 表序全集
export const UNIVERSAL_INSTALL_AGENT_ORDER: readonly string[];     // installPreference 升序
export const DEFAULT_ENABLED_PLATFORM_IDS: readonly [...];         // 默认启用平台
```

**新增平台 #N 的操作**：只改 `platformRegistry.ts`（若属 Universal 分组则加一行；默认启用则进 `DEFAULT_ENABLED_PLATFORM_IDS`），同步更新 `src/test/lib/platformRegistry.test.ts` 的基准期望。后端侧平台定义在 `db/seed.rs`（与本约定正交）。

**测试锁**：`src/test/lib/platformRegistry.test.ts` 锁推导列表与不变量（id 唯一 / installPreference 唯一 / install 候选 ⊆ 全集）。

**Wrong vs Correct**：

```ts
// ❌ Wrong：在组件或其他 lib 里再写一份平台 ID 有序列表
const MY_PLATFORM_ORDER = ["codex", "cursor", ...];

// ✅ Correct：从 registry import 推导值
import { UNIVERSAL_AGENT_ID_ORDER } from "@/lib/platformRegistry";
```

## 约定 2：安装类对话框多选必须走 `PlatformMultiSelect`

**What**：`src/components/platform/PlatformMultiSelect.tsx` 是平台多选的唯一实现（hook + 网格 + 失败列表），4 个安装对话框（Install / CollectionInstall / BatchInstallCentralSkills / ProjectInstall）均为薄调用方。新增安装入口时禁止重建内联多选网格或 `selectedInstallAgentIds` 推导。

**签名**：

```ts
usePlatformTargetSelection({
  targets,                    // 调用方已滤 central
  isTargetDisabled?,          // 勾选守卫（shared-root / project-unsupported / locked）
  isTargetDefaultSelected?,   // 默认勾选语义，缺省 = 非 disabled 全选
}) => { selectedIds, isSelected, toggle, reset, selectedInstallAgentIds }
// toggle/reset 引用稳定（内部 latest-ref），effect 可安全依赖；
// selectedInstallAgentIds() 渲染期现算：排 disabled → flatMap install ids → 去重

PlatformMultiSelectGrid({ targets, isSelected, isDisabled?, onToggle,
  renderBadges?, showIcon?, labelVariant?, emptyMessage, ariaLabel })

InstallFailureList({ failures: Array<{ key: string; label: string }> })
```

**差异走 props，不走复制**：各对话框的默认勾选语义（Install=全部 enabled/visible；Collection=全部含 locked；Batch=减 excluded；Project=全部 eligible）经 `isTargetDefaultSelected` 表达；徽标差异（linked / alwaysIncluded / willReplace…）经 `renderBadges`；headline 文案与 skipped 汇总语义保留在各对话框（i18n key 不同，不强并）。

**Gotcha**：`isTargetDisabled` 谓词可随对话框状态（如 targetMode）变化——hook 的推导用当前渲染传入的谓词现算，不缓存旧结果；`PlatformMultiSelect.test.tsx` 有专门用例锁定。

## 约定 3：Universal 展示分支用 3 个 helper，不手写三元

**What**：组件里 `isUniversalPlatformTarget(x) ? … : …` 的纯展示分支一律用 `src/lib/platformTargetGroups.ts` 的 helper：

```ts
getPlatformTargetLabel(target, t, "full" | "short"); // 标签（universalLabel / universalShortLabel / display_name）
getPlatformTargetTitleHint(target); // tooltip（member 名单 vs global_skills_dir）
getPlatformTargetCountAgentId(target); // 计数代表（install_agent_id vs id）
```

**允许保留 `isUniversalPlatformTarget` 直接调用的类别**（2026-07-04 design D3 裁决）：结构性分支——成员求和（`Sidebar.platformCount`）、页面路由/区块开关（`PlatformView.isUniversalPage`）、成员遍历/分组展开（`platformCleanupGroups`）、行组件路由（settings 可见性）、以及语义与 helper 不等价处（`CentralSearchBar.titleAttr` 的 fallback 是 displayName 而非 skills dir）。替换前先确认渲染输出逐字等价，不等价不换。
