import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import type { ScannedSkill } from "@/types";

export type PlatformSourceFilter = "all" | "user" | "plugin";
export type PlatformOriginFilter =
  | { kind: "all" }
  | { kind: "standalone" }
  | { kind: "central"; repoKey?: string };
export type PlatformSortField =
  "repository" | "name" | "installedAt" | "updatedAt" | "callCount";
export type PlatformSortDirection = "asc" | "desc";
export type PlatformGroupBy = "none" | "repository";

export interface PlatformSortState {
  field: PlatformSortField;
  direction: PlatformSortDirection;
}

export interface PlatformSkillGroup {
  key: string;
  label: string;
  skills: ScannedSkill[];
  weight: number;
}

export interface PlatformSkillGroupLabels {
  all: string;
  localSource: string;
  pluginSource: string;
  unknownSource: string;
}

export interface DerivePlatformSkillRowsInput {
  skills: readonly ScannedSkill[];
  searchQuery: string;
  sourceFilter: PlatformSourceFilter;
  originFilter: PlatformOriginFilter;
  sort: PlatformSortState;
  groupBy: PlatformGroupBy;
  labels: PlatformSkillGroupLabels;
  usageCounts?: Record<string, number>;
}

export interface DerivePlatformSkillRowsOutput {
  sourceFilteredSkills: ScannedSkill[];
  originFilteredSkills: ScannedSkill[];
  filteredSkills: ScannedSkill[];
  sortedSkills: ScannedSkill[];
  groups: PlatformSkillGroup[];
}

interface RepositoryGroupInfo {
  key: string;
  label: string;
  weight: number;
}

function parseSortableTimestamp(value?: string | null): number {
  if (!value) return 0;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function compareSkillNames(a: ScannedSkill, b: ScannedSkill): number {
  return a.name.localeCompare(b.name, undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function getInstalledSortTimestamp(skill: ScannedSkill): number {
  return parseSortableTimestamp(
    skill.installed_at ?? skill.created_at ?? skill.scanned_at,
  );
}

function getUpdatedSortTimestamp(skill: ScannedSkill): number {
  return parseSortableTimestamp(skill.updated_at ?? skill.scanned_at);
}

function getSkillCallCount(
  skill: ScannedSkill,
  usageCounts?: Record<string, number>,
): number {
  const count = usageCounts?.[skill.name] ?? 0;
  return Number.isFinite(count) ? count : 0;
}

function compareTimestamps(
  a: ScannedSkill,
  b: ScannedSkill,
  direction: PlatformSortDirection,
  resolveTimestamp: (skill: ScannedSkill) => number,
): number {
  const dir = direction === "asc" ? 1 : -1;
  const timeComparison = resolveTimestamp(a) - resolveTimestamp(b);
  if (timeComparison !== 0) return timeComparison * dir;
  return compareSkillNames(a, b);
}

function sourceRootLabel(sourceRoot?: string | null): string | null {
  if (!sourceRoot?.trim()) return null;
  const parts = sourceRoot
    .replace(/\\/g, "/")
    .split("/")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return null;

  const cacheIndex = parts.findIndex(
    (part, index) =>
      part === "cache" && index > 0 && parts[index - 1] === "plugins",
  );
  if (cacheIndex >= 0) {
    const publisher = parts[cacheIndex + 1];
    const plugin = parts[cacheIndex + 2];
    if (publisher && plugin) return `${publisher}/${plugin}`;
  }

  if (parts.length >= 2) return parts.slice(-2).join("/");
  return parts[0] ?? null;
}

function keySegment(value: string): string {
  return (
    value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "") || "unknown"
  );
}

export function getPlatformRepositoryGroupInfo(
  skill: ScannedSkill,
  labels: PlatformSkillGroupLabels,
): RepositoryGroupInfo {
  const repository = skill.repository ?? null;
  if (repository && !repository.is_unknown) {
    const label =
      repository.name ||
      [repository.owner, repository.repo].filter(Boolean).join("/") ||
      repository.id;
    return {
      key: `repo:${repository.id}`,
      label,
      weight: 1,
    };
  }

  if (skill.source_kind === "plugin") {
    const label = sourceRootLabel(skill.source_root) ?? labels.pluginSource;
    return {
      key: `plugin:${keySegment(skill.source_root ?? label)}`,
      label,
      weight: 2,
    };
  }

  if (skill.source_kind === "user" || skill.dir_path || skill.file_path) {
    return {
      key: "source:local",
      label: labels.localSource,
      weight: 9998,
    };
  }

  return {
    key: "source:unknown",
    label: labels.unknownSource,
    weight: 9999,
  };
}

/** 与 SkillCardBadges.SourceIndicator 同语义：symlink 即中央链接，其余归独立安装 */
export function getPlatformSkillOrigin(
  skill: ScannedSkill,
): "central" | "standalone" {
  return skill.link_type === "symlink" ? "central" : "standalone";
}

export const PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY = "unassigned";

/** central 组内的仓库桶 key：仓库已指派 → `repo:<id>`；否则 "unassigned" */
export function getPlatformOriginRepoKey(skill: ScannedSkill): string {
  const repository = skill.repository ?? null;
  if (repository && !repository.is_unknown) return `repo:${repository.id}`;
  return PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY;
}

export interface PlatformOriginNavModel {
  total: number;
  centralCount: number;
  standaloneCount: number;
  /** 仅统计 origin=central 且仓库已指派的行；按 label 排序 */
  repos: Array<{ key: string; label: string; count: number }>;
  /** symlink 且仓库未指派（缺失或 is_unknown）的行数 */
  unassignedCentralCount: number;
}

export function derivePlatformOriginNav(
  skills: readonly ScannedSkill[],
): PlatformOriginNavModel {
  let centralCount = 0;
  let standaloneCount = 0;
  let unassignedCentralCount = 0;
  const repoBuckets = new Map<
    string,
    { key: string; label: string; count: number }
  >();

  for (const skill of skills) {
    if (getPlatformSkillOrigin(skill) === "standalone") {
      standaloneCount += 1;
      continue;
    }
    centralCount += 1;
    const repoKey = getPlatformOriginRepoKey(skill);
    if (repoKey === PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY || !skill.repository) {
      unassignedCentralCount += 1;
      continue;
    }
    const bucket = repoBuckets.get(repoKey);
    if (bucket) {
      bucket.count += 1;
      continue;
    }
    // label 回退顺序与 getPlatformRepositoryGroupInfo 一致：name → owner/repo → id
    const repository = skill.repository;
    const label =
      repository.name ||
      [repository.owner, repository.repo].filter(Boolean).join("/") ||
      repository.id;
    repoBuckets.set(repoKey, { key: repoKey, label, count: 1 });
  }

  const repos = Array.from(repoBuckets.values()).sort((a, b) =>
    a.label.localeCompare(b.label, undefined, {
      numeric: true,
      sensitivity: "base",
    }),
  );

  return {
    total: skills.length,
    centralCount,
    standaloneCount,
    repos,
    unassignedCentralCount,
  };
}

function matchesOriginFilter(
  skill: ScannedSkill,
  filter: PlatformOriginFilter,
): boolean {
  if (filter.kind === "all") return true;
  const origin = getPlatformSkillOrigin(skill);
  if (filter.kind === "standalone") return origin === "standalone";
  if (origin !== "central") return false;
  if (filter.repoKey === undefined) return true;
  return getPlatformOriginRepoKey(skill) === filter.repoKey;
}

export function comparePlatformSkills(
  a: ScannedSkill,
  b: ScannedSkill,
  sort: PlatformSortState,
  labels: PlatformSkillGroupLabels,
  usageCounts?: Record<string, number>,
): number {
  const dir = sort.direction === "asc" ? 1 : -1;

  if (sort.field === "name") {
    return compareSkillNames(a, b) * dir;
  }

  if (sort.field === "installedAt") {
    return compareTimestamps(a, b, sort.direction, getInstalledSortTimestamp);
  }

  if (sort.field === "updatedAt") {
    return compareTimestamps(a, b, sort.direction, getUpdatedSortTimestamp);
  }

  if (sort.field === "callCount") {
    const countComparison =
      getSkillCallCount(a, usageCounts) - getSkillCallCount(b, usageCounts);
    if (countComparison !== 0) return countComparison * dir;
    return compareSkillNames(a, b);
  }

  const aGroup = getPlatformRepositoryGroupInfo(a, labels);
  const bGroup = getPlatformRepositoryGroupInfo(b, labels);
  if (aGroup.weight !== bGroup.weight)
    return (aGroup.weight - bGroup.weight) * dir;
  const labelComparison = aGroup.label.localeCompare(bGroup.label, undefined, {
    numeric: true,
    sensitivity: "base",
  });
  if (labelComparison !== 0) return labelComparison * dir;
  return compareSkillNames(a, b);
}

function matchesSearch(skill: ScannedSkill, query: string): boolean {
  if (!query) return true;
  const repository = skill.repository ?? null;
  const repositoryLabel = repository
    ? [
        repository.name,
        repository.owner,
        repository.repo,
        repository.branch,
        repository.url,
      ].join(" ")
    : "";
  return buildSearchText([
    skill.id,
    skill.name,
    skill.description,
    repositoryLabel,
    skill.source_root,
    skill.source_path,
    skill.dir_path,
    skill.file_path,
  ]).includes(query);
}

function groupPlatformSkills(
  skills: ScannedSkill[],
  groupBy: PlatformGroupBy,
  labels: PlatformSkillGroupLabels,
): PlatformSkillGroup[] {
  if (groupBy === "none") {
    return [{ key: "__all__", label: labels.all, skills, weight: 0 }];
  }

  const map = new Map<string, PlatformSkillGroup>();
  for (const skill of skills) {
    const groupInfo = getPlatformRepositoryGroupInfo(skill, labels);
    let group = map.get(groupInfo.key);
    if (!group) {
      group = { ...groupInfo, skills: [] };
      map.set(groupInfo.key, group);
    }
    group.skills.push(skill);
  }

  return Array.from(map.values()).sort((a, b) => {
    if (a.weight !== b.weight) return a.weight - b.weight;
    return a.label.localeCompare(b.label, undefined, {
      numeric: true,
      sensitivity: "base",
    });
  });
}

export function derivePlatformSkillRows(
  input: DerivePlatformSkillRowsInput,
): DerivePlatformSkillRowsOutput {
  const sourceFilteredSkills = input.skills.filter((skill) => {
    if (input.sourceFilter === "all") return true;
    return skill.source_kind === input.sourceFilter;
  });
  const originFilteredSkills = sourceFilteredSkills.filter((skill) =>
    matchesOriginFilter(skill, input.originFilter),
  );
  const query = normalizeSearchQuery(input.searchQuery);
  const filteredSkills = originFilteredSkills.filter((skill) =>
    matchesSearch(skill, query),
  );
  const sortedSkills = [...filteredSkills].sort((a, b) =>
    comparePlatformSkills(a, b, input.sort, input.labels, input.usageCounts),
  );
  const groups = groupPlatformSkills(sortedSkills, input.groupBy, input.labels);

  return {
    sourceFilteredSkills,
    originFilteredSkills,
    filteredSkills,
    sortedSkills,
    groups,
  };
}
