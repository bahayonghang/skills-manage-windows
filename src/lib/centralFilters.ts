/**
 * Central Skills 多选筛选 helper。
 *
 * 现有 view-model（`src/pages/centralSkillsViewModel.ts`）使用单选 string 表达
 * filter（"all" / "unassigned" / repo.id 等特殊值）。M1 引入新 sidebar 时会改成
 * `string[]` 多选，但旧 UI 仍依赖单选 API。
 *
 * 这里提供一组 helper：同时接受 `string` 与 `readonly string[]`，让两个调用点
 * 可以共用同一份匹配逻辑，避免行为漂移。
 */

import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

/** 把单选 / 多选 / undefined 统一成数组。空字符串视为空数组。 */
export function coerceToArray(value: string | readonly string[] | undefined): string[] {
  if (value === undefined) return [];
  if (typeof value === "string") return value.length > 0 ? [value] : [];
  return value.filter((v) => v.length > 0);
}

/**
 * Repository filter 的特殊值。
 * - `"all"`：不过滤（命中所有）
 * - `"unassigned"`：仅命中 is_source_unknown 或 repository.is_unknown
 * - 其他：当作 repository.id
 */
export function matchesRepositoryFilter(
  skill: SkillWithLinks,
  filter: string | readonly string[] | undefined
): boolean {
  const values = coerceToArray(filter);
  if (values.length === 0 || values.includes("all")) return true;

  const repoId = skill.repository?.id ?? null;
  const isUnassigned =
    skill.is_source_unknown === true || skill.repository?.is_unknown === true;

  return values.some((value) => {
    if (value === "unassigned") return isUnassigned;
    return repoId === value;
  });
}

/**
 * Tag filter 的特殊值。
 * - `"all"`：不过滤
 * - `"uncategorized"`：tag 为空或全部是 "uncategorized" 占位
 * - `"updates"`：update_status === "update_available"
 * - `"ai-review"`：在 AI review 队列中
 * - 其他：当作 tag.id
 *
 * 多选语义：任一命中即视为匹配（OR）。这与多选 facet 直觉一致。
 */
export interface TagFilterContext {
  updateStatuses: Record<string, CentralSkillUpdateState>;
  aiReviewSkillIds: ReadonlySet<string>;
}

export function matchesTagFilter(
  skill: SkillWithLinks,
  filter: string | readonly string[] | undefined,
  ctx: TagFilterContext
): boolean {
  const values = coerceToArray(filter);
  if (values.length === 0 || values.includes("all")) return true;

  const skillTags = skill.tags ?? [];

  return values.some((value) => {
    if (value === "uncategorized") {
      return (
        skillTags.length === 0
        || skillTags.every((tag) => tag.id === "uncategorized")
      );
    }
    if (value === "updates") {
      return ctx.updateStatuses[skill.id]?.status === "update_available";
    }
    if (value === "ai-review") {
      return ctx.aiReviewSkillIds.has(skill.id);
    }
    return skillTags.some((tag) => tag.id === value);
  });
}
