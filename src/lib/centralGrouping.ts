/**
 * Central Skills V2 列表分组（M4）。
 *
 * 纯函数：把已排序的 `SkillWithLinks[]` 按 `GroupByMode` 切分成有序的
 * `SkillGroup[]`。组内顺序保留输入顺序（即调用方已 sort 过的结果）。
 *
 * 设计：
 * - `none` 仍然返回单一 group（key `__all__`），让上层渲染逻辑统一。
 * - 多值 facet（`tag`）按"每个 tag 一个 group，skill 出现在每个所属 group"
 *   语义；无 tag 的 skill 进入 `__uncategorized__` 组。
 * - `owner` 复用 V2 sidebar 的 owner/local 划分（GitHub repo owner / local
 *   skills 单独成组）；与 `repository` 的区别在于粒度。
 * - `status` 按更新状态（needs_update / up_to_date / unknown）分组。这里
 *   状态信息来自 `updateStatuses[skill.id]?.status`，调用方在 ctx 中传入。
 */

import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";
import type { GroupByMode } from "@/lib/centralViewState";

export interface SkillGroup {
  /** 稳定的 key，用于 React render key 和滚动定位。 */
  key: string;
  /** 显示名（已 i18n）。 */
  label: string;
  /** 组内技能。 */
  skills: SkillWithLinks[];
  /** 该组的二级排序权重（越小越靠前；同权重按 label 字典序）。 */
  weight: number;
}

export interface GroupingContext {
  updateStatuses: Record<string, CentralSkillUpdateState>;
  /** i18n 标签解析器，调用方注入以避免库依赖 i18n 实例。 */
  labels: {
    all: string;
    uncategorized: string;
    unknownOwner: string;
    localRepos: string;
    statusUpToDate: string;
    statusNeedsUpdate: string;
    statusUnknown: string;
  };
}

export function groupSkillsByMode(
  skills: SkillWithLinks[],
  mode: GroupByMode,
  ctx: GroupingContext,
): SkillGroup[] {
  if (mode === "none") {
    return [{ key: "__all__", label: ctx.labels.all, skills, weight: 0 }];
  }

  if (mode === "repository") return groupByRepository(skills, ctx);
  if (mode === "owner") return groupByOwner(skills, ctx);
  if (mode === "tag") return groupByTag(skills, ctx);
  if (mode === "status") return groupByStatus(skills, ctx);

  return [{ key: "__all__", label: ctx.labels.all, skills, weight: 0 }];
}

function groupByRepository(
  skills: SkillWithLinks[],
  ctx: GroupingContext,
): SkillGroup[] {
  const map = new Map<string, SkillGroup>();
  for (const skill of skills) {
    const repoId = skill.repository?.id ?? "__unassigned__";
    const label = skill.repository?.name ?? ctx.labels.uncategorized;
    let group = map.get(repoId);
    if (!group) {
      group = {
        key: `repo:${repoId}`,
        label,
        skills: [],
        weight: repoId === "__unassigned__" ? 9999 : 1,
      };
      map.set(repoId, group);
    }
    group.skills.push(skill);
  }
  return sortGroups(Array.from(map.values()));
}

function groupByOwner(
  skills: SkillWithLinks[],
  ctx: GroupingContext,
): SkillGroup[] {
  const map = new Map<string, SkillGroup>();
  for (const skill of skills) {
    const repo = skill.repository;
    let ownerKey: string;
    let label: string;
    let weight: number;

    if (!repo) {
      ownerKey = "__unassigned__";
      label = ctx.labels.uncategorized;
      weight = 9999;
    } else if (repo.source_type === "github" && repo.owner) {
      ownerKey = `gh:${repo.owner}`;
      label = repo.owner;
      weight = 1;
    } else if (repo.source_type === "github") {
      ownerKey = "gh:__no_owner__";
      label = ctx.labels.unknownOwner;
      weight = 5000;
    } else {
      ownerKey = "local";
      label = ctx.labels.localRepos;
      weight = 5001;
    }

    let group = map.get(ownerKey);
    if (!group) {
      group = { key: `owner:${ownerKey}`, label, skills: [], weight };
      map.set(ownerKey, group);
    }
    group.skills.push(skill);
  }
  return sortGroups(Array.from(map.values()));
}

function groupByTag(
  skills: SkillWithLinks[],
  ctx: GroupingContext,
): SkillGroup[] {
  const map = new Map<string, SkillGroup>();
  for (const skill of skills) {
    const tags = skill.tags ?? [];
    if (tags.length === 0) {
      addToGroup(map, "__uncategorized__", ctx.labels.uncategorized, 9999, skill);
      continue;
    }
    for (const tag of tags) {
      addToGroup(map, `tag:${tag.id}`, tag.name, 1, skill);
    }
  }
  return sortGroups(Array.from(map.values()));
}

function groupByStatus(
  skills: SkillWithLinks[],
  ctx: GroupingContext,
): SkillGroup[] {
  const map = new Map<string, SkillGroup>();
  for (const skill of skills) {
    const status = ctx.updateStatuses[skill.id]?.status;
    let key: string;
    let label: string;
    let weight: number;
    if (status === "update_available") {
      key = "status:update_available";
      label = ctx.labels.statusNeedsUpdate;
      weight = 1;
    } else if (status === "up_to_date") {
      key = "status:up_to_date";
      label = ctx.labels.statusUpToDate;
      weight = 2;
    } else {
      key = "status:unknown";
      label = ctx.labels.statusUnknown;
      weight = 3;
    }
    let group = map.get(key);
    if (!group) {
      group = { key, label, skills: [], weight };
      map.set(key, group);
    }
    group.skills.push(skill);
  }
  return sortGroups(Array.from(map.values()));
}

function addToGroup(
  map: Map<string, SkillGroup>,
  key: string,
  label: string,
  weight: number,
  skill: SkillWithLinks,
): void {
  let group = map.get(key);
  if (!group) {
    group = { key, label, skills: [], weight };
    map.set(key, group);
  }
  group.skills.push(skill);
}

function sortGroups(groups: SkillGroup[]): SkillGroup[] {
  return groups.sort((a, b) => {
    if (a.weight !== b.weight) return a.weight - b.weight;
    return a.label.localeCompare(b.label, undefined, { sensitivity: "base" });
  });
}
