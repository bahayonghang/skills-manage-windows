/**
 * Central Skills V2 view-model (M1)。
 *
 * 区别于 `centralSkillsViewModel.ts`：
 * - `repositoryFilter` / `tagFilter` 改为 `string[]` 多选
 * - 搜索框接受结构化语法（tag:/repo:/owner:/has:/...），由
 *   `centralSearchQuery.ts` 解析为 AST，filters 与多选 facet 合取（AND）
 * - 暴露 `CentralViewState`（grid/list、group-by、sort），由 URL state 同步
 * - 暴露 `FacetCounts`，sidebar 直接消费
 *
 * 组件层只接受 V2 view-model 的派生数据；旧 view-model 仍服务旧 Shell。
 */

import { useMemo } from "react";

import { matchesRepositoryFilter, matchesTagFilter } from "@/lib/centralFilters";
import {
  computeFacetCounts,
  type FacetCounts,
  type FacetSelections,
} from "@/lib/centralFacetCounts";
import {
  matchSkillAgainstFilters,
  parseCentralQuery,
  type CentralQueryAst,
  type CentralQueryContext,
} from "@/lib/centralSearchQuery";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import type { CentralViewState } from "@/lib/centralViewState";
import type {
  CentralSkillUpdateState,
  SkillAiTagReview,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";

import type {
  CentralSortDirection,
  CentralSortField,
} from "@/pages/centralSkillsViewModel";

export interface CentralViewModelV2Input {
  skills: readonly SkillWithLinks[];
  repositories: readonly SkillRepositoryWithStats[];
  tags: readonly SkillTag[];
  aiTagReviews: readonly SkillAiTagReview[];
  updateStatuses: Record<string, CentralSkillUpdateState>;
  /** 视图状态（来自 URL state hook 或顶层组件）。 */
  state: CentralViewState;
}

export interface CentralViewModelV2Output {
  /** 经过结构化查询 + facet 多选 + 自由词搜索后的命中列表。 */
  filteredSkills: SkillWithLinks[];
  /** 经过排序的结果（消费此字段渲染列表）。 */
  sortedSkills: SkillWithLinks[];
  /** AST：方便组件层把 invalid token 提示给用户。 */
  queryAst: CentralQueryAst;
  /** Sidebar 的动态 facet 计数。 */
  facetCounts: FacetCounts;
  /** 当前是否在搜索（用于切换列表渲染样式）。 */
  isSearchActive: boolean;
  /** 标准化后的自由词，方便上层调试展示。 */
  normalizedFreeText: string;
}

function parseSortableTimestamp(value?: string | null): number {
  if (!value) return 0;
  const t = Date.parse(value);
  return Number.isFinite(t) ? t : 0;
}

function getSkillSortTimestamp(
  skill: SkillWithLinks,
  field: CentralSortField
): number {
  return parseSortableTimestamp(
    field === "createdAt"
      ? skill.created_at ?? skill.scanned_at
      : skill.updated_at ?? skill.scanned_at
  );
}

function compareSkillsForSort(
  a: SkillWithLinks,
  b: SkillWithLinks,
  field: CentralSortField,
  direction: CentralSortDirection
): number {
  const dir = direction === "asc" ? 1 : -1;
  const nameComparison = a.name.localeCompare(b.name, undefined, {
    numeric: true,
    sensitivity: "base",
  });
  if (field === "name") return nameComparison * dir;
  const timeComparison =
    getSkillSortTimestamp(a, field) - getSkillSortTimestamp(b, field);
  return timeComparison === 0 ? nameComparison : timeComparison * dir;
}

/**
 * 派生 V2 视图所需的数据。纯函数 + memo，可在 React render 中安全调用。
 */
export function useCentralSkillsViewModelV2(
  input: CentralViewModelV2Input
): CentralViewModelV2Output {
  const { skills, repositories, tags, aiTagReviews, updateStatuses, state } = input;

  const aiReviewSkillIds = useMemo(
    () => new Set(aiTagReviews.map((r) => r.skill_id)),
    [aiTagReviews]
  );

  const queryAst = useMemo(() => parseCentralQuery(state.q), [state.q]);

  const queryContext: CentralQueryContext = useMemo(
    () => ({ updateStatuses, aiReviewSkillIds }),
    [updateStatuses, aiReviewSkillIds]
  );

  const normalizedFreeText = useMemo(
    () => normalizeSearchQuery(queryAst.freeText),
    [queryAst.freeText]
  );

  const isSearchActive = normalizedFreeText.length > 0 || queryAst.filters.length > 0;

  const searchableSkills = useMemo(
    () =>
      skills.map((skill) => ({
        skill,
        searchText: buildSearchText([
          skill.name,
          skill.description,
          skill.repository?.name,
          skill.source_path,
          ...(skill.tags ?? []).map((t) => t.name),
        ]),
      })),
    [skills]
  );

  const filteredSkills = useMemo(() => {
    return searchableSkills
      .filter(({ searchText }) =>
        normalizedFreeText.length === 0 || searchText.includes(normalizedFreeText)
      )
      .filter(({ skill }) => matchSkillAgainstFilters(skill, queryAst, queryContext))
      .map(({ skill }) => skill)
      .filter((skill) => matchesRepositoryFilter(skill, state.repos))
      .filter((skill) =>
        matchesTagFilter(skill, state.tags, {
          updateStatuses,
          aiReviewSkillIds,
        })
      );
  }, [
    aiReviewSkillIds,
    normalizedFreeText,
    queryAst,
    queryContext,
    searchableSkills,
    state.repos,
    state.tags,
    updateStatuses,
  ]);

  const sortedSkills = useMemo(() => {
    return [...filteredSkills].sort((a, b) =>
      compareSkillsForSort(a, b, state.sortField, state.sortDir)
    );
  }, [filteredSkills, state.sortField, state.sortDir]);

  const selections: FacetSelections = useMemo(
    () => ({ repositories: state.repos, tags: state.tags }),
    [state.repos, state.tags]
  );

  const facetCounts = useMemo(
    () =>
      computeFacetCounts(skills, repositories, tags, selections, {
        updateStatuses,
        aiReviewSkillIds,
      }),
    [aiReviewSkillIds, repositories, selections, skills, tags, updateStatuses]
  );

  return {
    filteredSkills,
    sortedSkills,
    queryAst,
    facetCounts,
    isSearchActive,
    normalizedFreeText,
  };
}
