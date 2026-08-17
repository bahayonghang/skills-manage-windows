# 前端平台技能安装来源分类约定

> 建立于 2026-07-07（任务 07-07-ui-platform-origin-nav）。背景：平台视图需要区分「SkillPort 安装」与「独立安装（用户自行放置）」并按来源仓库细分；调研发现最直觉的两个信号（`installed_at`、`repository` 存在与否）都是陷阱，正确信号只有一个且必须单点收敛，否则后续任何触碰安装来源的功能都会重蹈覆辙。

## 约定 1：安装来源判定与聚合唯一入口 `src/lib/platformSkillViewModel.ts`

**What**：平台技能行（`ScannedSkill`）的安装来源分类、repo 桶 key、导航聚合，只允许调用视图模型层的这组导出；组件层禁止再写 `link_type` / `repository` 判定分支。

**签名**：

```ts
export type PlatformOriginFilter =
  | { kind: "all" }
  | { kind: "standalone" }
  | { kind: "central"; repoKey?: string }; // repoKey 缺省 = 整组；"unassigned" = 仓库未指派

/** 分类规则：link_type === "symlink" → "central"；否则 "standalone"。
 *  与卡片 SkillCardBadges.SourceIndicator（中央技能库/独立安装徽标）逐字同语义。 */
export function getPlatformSkillOrigin(skill: ScannedSkill): "central" | "standalone";

export const PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY = "unassigned";
/** repository 存在且 !is_unknown → `repo:${repository.id}`；否则 "unassigned" */
export function getPlatformOriginRepoKey(skill: ScannedSkill): string;

export interface PlatformOriginNavModel {
  total: number;
  centralCount: number;
  standaloneCount: number;
  repos: Array<{ key: string; label: string; count: number }>; // 仅 central 行；label 按 repository.name || owner/repo || id 回退
  unassignedCentralCount: number;
}
export function derivePlatformOriginNav(skills: readonly ScannedSkill[]): PlatformOriginNavModel;

// derivePlatformSkillRows 管线固定顺序：source tab 过滤 → originFilter → 搜索 → 排序 → 分组；
// sourceFilteredSkills = tab 后 origin 前（导航计数口径），originFilteredSkills = origin 后搜索前（空态判断）。
```

轴 B（`source_kind === "plugin"`）与轴 A 正交。来源 Tab（全部 / 用户 / 插件）显示条件是 `isClaudePage || pluginCount > 0`。Claude 页始终显示。无插件的非 Claude 页不显示。`PlatformOriginNav` 只表达轴 A；默认 origin 仍是 `{ kind: "all" }`。禁止把插件筛并进 origin 导航或用 `installed_at` 补分类。

**Wrong vs Correct**：

```tsx
// ❌ Wrong：组件里自行判定来源
const isSkillPort = skill.link_type === "symlink"; // 组件层重复分类逻辑
const isSkillPort = skill.installed_at != null;    // 错误信号（见约定 2）
const isSkillPort = !!skill.repository;            // 错误信号：手放技能匹配中央同名 id 也带 repo 指派

// ✅ Correct：从视图模型 import
import { getPlatformSkillOrigin } from "@/lib/platformSkillViewModel";
const origin = getPlatformSkillOrigin(skill);
```

**测试锁**：`src/test/lib/platformSkillViewModel.test.ts` 锁分类规则、聚合守恒（central + standalone = total；repo 桶之和 + unassigned = centralCount）、standalone 带 repo 指派不计入 repo 桶、管线顺序（origin 在 tab 后搜索前）。

## 约定 2（Gotcha）：`installed_at` 不是「SkillPort 装的」信号

> **Warning**：扫描器对平台目录里发现的**所有**技能（含用户手工拷贝的）都会 upsert `skill_installations` 行（`src-tauri/src/services/scanner/persistence.rs:126-143`），所以 `installed_at` 实际是「首次被扫描到」的时间，与是否经 SkillPort 安装无关。

**推论**：

- SkillPort 以 copy 方式安装的技能，落盘后与用户手工拷贝目录**不可区分**（同为普通目录，`link_type = "copy"`）。UI 一律按「独立安装」展示——与卡片徽标语义一致，用户可在界面自洽验证。
- 唯一可靠的「SkillPort 安装」信号是 `link_type === "symlink"`（symlink 到中央目录只由 SkillPort 安装流程创建）。
- 若未来需要精确区分 copy 安装来源，必须由后端在安装链路落持久化 origin 标记（新任务、动 schema），**禁止**前端用 `installed_at` / `repository` 存在性等启发式补救。
