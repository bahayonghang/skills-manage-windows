/**
 * 分面（facet）计数。
 *
 * 输入：全量 skills + 当前已选筛选 + 上下文（更新状态 / AI review）
 * 输出：每个 facet 候选项当前会命中的 skill 数
 *
 * 设计要点：
 * - **dependent counts**：计算 facet F 的某候选 v 的计数时，过滤条件中暂时
 *   "去掉 F 维度自己"再加上 `{ F: v }`。这样用户切到某个 owner 后看 tag
 *   计数能反映"在该 owner 下还剩多少"，符合业界分面搜索直觉。
 * - **多选维度内是 OR**：选 repos=[A,B] 表示属于 A 或 B；再加 tag 维度时跨
 *   维度是 AND。
 * - **零依赖**：仅复用 `centralFilters.ts` 里的 matcher，不引入 search query。
 *   search query AST 由 `centralSearchQuery.ts` 单独 AND 进结果。
 */

import { matchesRepositoryFilter, matchesTagFilter } from "@/lib/centralFilters";
import type { CentralSkillUpdateState, SkillRepositoryWithStats, SkillTag, SkillWithLinks } from "@/types";

export interface FacetSelections {
  /** 已选仓库 id（含特殊值 "unassigned"） */
  repositories: readonly string[];
  /** 已选 tag id（含特殊值 "uncategorized" / "updates" / "ai-review"） */
  tags: readonly string[];
}

export interface FacetCountsContext {
  updateStatuses: Record<string, CentralSkillUpdateState>;
  aiReviewSkillIds: ReadonlySet<string>;
}

export interface FacetCounts {
  /** key 为 repository.id；额外的 "unassigned" 与 "all" 也包含在内。 */
  repositories: Record<string, number>;
  /** key 为 tag.id。 */
  tags: Record<string, number>;
  /** 智能视图固定计数。 */
  smartViews: {
    all: number;
    uncategorized: number;
    updates: number;
    aiReview: number;
  };
}

function passesAllExceptRepository(
  skill: SkillWithLinks,
  selections: FacetSelections,
  ctx: FacetCountsContext
): boolean {
  return matchesTagFilter(skill, selections.tags, ctx);
}

function passesAllExceptTag(
  skill: SkillWithLinks,
  selections: FacetSelections
): boolean {
  return matchesRepositoryFilter(skill, selections.repositories);
}

/**
 * 计算所有 facet 维度的命中数。
 *
 * @param skills        全量 skills
 * @param repositories  仓库元数据（用于补齐计数 key 即使该 repo 当前 0 个 skill）
 * @param tags          标签元数据
 * @param selections    当前已选 facets（不计算时也要传，至少给空）
 * @param ctx           更新状态与 AI review 上下文
 */
export function computeFacetCounts(
  skills: readonly SkillWithLinks[],
  repositories: readonly SkillRepositoryWithStats[],
  tags: readonly SkillTag[],
  selections: FacetSelections,
  ctx: FacetCountsContext
): FacetCounts {
  const repositoriesCount: Record<string, number> = { all: 0, unassigned: 0 };
  for (const repo of repositories) {
    repositoriesCount[repo.id] = 0;
  }
  const tagsCount: Record<string, number> = {};
  for (const tag of tags) {
    tagsCount[tag.id] = 0;
  }

  let totalCount = 0;
  let uncategorizedCount = 0;
  let updatesCount = 0;
  let aiReviewCount = 0;

  // 仓库维度：对每个 skill 依次按"除 repo 外的过滤"过一遍，命中后对其 repo 增加计数
  // 仓库 + 标签 + smart 三类都各扫一遍，O(3N)。当 skills < 1k 时性能足够。
  for (const skill of skills) {
    if (passesAllExceptRepository(skill, selections, ctx)) {
      repositoriesCount.all += 1;
      const repoId = skill.repository?.id;
      const isUnassigned =
        skill.is_source_unknown === true || skill.repository?.is_unknown === true;
      if (isUnassigned) {
        repositoriesCount.unassigned += 1;
      }
      if (repoId && repoId in repositoriesCount) {
        repositoriesCount[repoId] += 1;
      }
    }
  }

  for (const skill of skills) {
    if (passesAllExceptTag(skill, selections)) {
      const skillTags = skill.tags ?? [];
      for (const t of skillTags) {
        if (t.id in tagsCount) {
          tagsCount[t.id] += 1;
        }
      }
    }
  }

  // Smart views：相对于"无 facet"的全量计数，方便用户切到任意视图
  for (const skill of skills) {
    totalCount += 1;
    const skillTags = skill.tags ?? [];
    if (skillTags.length === 0 || skillTags.every((t) => t.id === "uncategorized")) {
      uncategorizedCount += 1;
    }
    if (ctx.updateStatuses[skill.id]?.status === "update_available") {
      updatesCount += 1;
    }
    if (ctx.aiReviewSkillIds.has(skill.id)) {
      aiReviewCount += 1;
    }
  }

  return {
    repositories: repositoriesCount,
    tags: tagsCount,
    smartViews: {
      all: totalCount,
      uncategorized: uncategorizedCount,
      updates: updatesCount,
      aiReview: aiReviewCount,
    },
  };
}
