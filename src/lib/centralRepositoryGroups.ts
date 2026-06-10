/**
 * 仓库分组：在 sidebar 里把 GitHub 仓库按 owner 折叠，本地 / 未来源单列一组。
 *
 * UX 目标：用户面对 13+ 仓库时可一眼看清来源类型，并按 owner 折叠减少视觉噪音。
 */

import type { SkillRepositoryWithStats } from "@/types";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";

export type RepositoryGroupKind = "github" | "local";

export interface RepositoryOwnerGroup {
  kind: "owner";
  /** owner 名称，用作 group key */
  owner: string;
  repositories: SkillRepositoryWithStats[];
  /** 该 owner 下所有 repo 的 skill_count 之和（用于 group header 徽章） */
  totalSkillCount: number;
}

export interface RepositoryFlatGroup {
  kind: "flat";
  /** "local" / "unassigned" / "github-no-owner" 等不需 owner 折叠的桶 */
  groupId: string;
  repositories: SkillRepositoryWithStats[];
  totalSkillCount: number;
}

export type RepositorySidebarGroup = RepositoryOwnerGroup | RepositoryFlatGroup;

export interface RepositorySidebarSection {
  kind: RepositoryGroupKind;
  groups: RepositorySidebarGroup[];
  totalSkillCount: number;
}

function isGithubRepo(repo: SkillRepositoryWithStats): boolean {
  return repo.source_type === "github" || (Boolean(repo.owner) && Boolean(repo.repo));
}

function getEffectiveSkillCount(repo: SkillRepositoryWithStats): number {
  return repo.is_unknown ? repo.unknown_skill_count : repo.skill_count;
}

function shouldShowRepository(repo: SkillRepositoryWithStats): boolean {
  return !repo.is_unknown || getEffectiveSkillCount(repo) > 0;
}

/**
 * 按 owner 折叠 GitHub 仓库；本地 / 无 owner 的仓库放进 flat 组。
 *
 * 排序：
 * - GitHub section 内按 owner 字母升序，组内按 repo.name 升序
 * - Local section 内按 repo.name 升序
 * - GitHub section 整体在 Local 前面
 */
export function groupRepositoriesForSidebar(
  repositories: readonly SkillRepositoryWithStats[]
): RepositorySidebarSection[] {
  const githubByOwner = new Map<string, SkillRepositoryWithStats[]>();
  const githubNoOwner: SkillRepositoryWithStats[] = [];
  const local: SkillRepositoryWithStats[] = [];

  for (const repo of repositories) {
    if (!shouldShowRepository(repo)) continue;

    if (isGithubRepo(repo)) {
      const owner = (repo.owner ?? "").trim();
      if (owner.length === 0) {
        githubNoOwner.push(repo);
      } else {
        const list = githubByOwner.get(owner);
        if (list) list.push(repo);
        else githubByOwner.set(owner, [repo]);
      }
    } else {
      local.push(repo);
    }
  }

  const sections: RepositorySidebarSection[] = [];

  if (githubByOwner.size > 0 || githubNoOwner.length > 0) {
    const ownerGroups: RepositorySidebarGroup[] = Array.from(githubByOwner.entries())
      .sort((a, b) => a[0].localeCompare(b[0], undefined, { sensitivity: "base" }))
      .map(([owner, repos]) => ({
        kind: "owner" as const,
        owner,
        repositories: repos.slice().sort(sortByRepoName),
        totalSkillCount: repos.reduce((acc, r) => acc + getEffectiveSkillCount(r), 0),
      }));

    if (githubNoOwner.length > 0) {
      ownerGroups.push({
        kind: "flat",
        groupId: "github-no-owner",
        repositories: githubNoOwner.slice().sort(sortByRepoName),
        totalSkillCount: githubNoOwner.reduce(
          (acc, r) => acc + getEffectiveSkillCount(r),
          0
        ),
      });
    }

    sections.push({
      kind: "github",
      groups: ownerGroups,
      totalSkillCount: ownerGroups.reduce((acc, g) => acc + g.totalSkillCount, 0),
    });
  }

  if (local.length > 0) {
    sections.push({
      kind: "local",
      groups: [
        {
          kind: "flat",
          groupId: "local",
          repositories: local.slice().sort(sortByRepoName),
          totalSkillCount: local.reduce((acc, r) => acc + getEffectiveSkillCount(r), 0),
        },
      ],
      totalSkillCount: local.reduce((acc, r) => acc + getEffectiveSkillCount(r), 0),
    });
  }

  return sections;
}

export function filterRepositorySectionsForSearch(
  sections: readonly RepositorySidebarSection[],
  query: string
): RepositorySidebarSection[] {
  const normalizedQuery = normalizeSearchQuery(query);
  if (!normalizedQuery) return sections.slice();

  return sections
    .map((section) => {
      const groups = section.groups
        .map((group): RepositorySidebarGroup | null => {
          if (group.kind === "owner") {
            const ownerMatches = normalizeSearchQuery(group.owner).includes(normalizedQuery);
            const repositories = ownerMatches
              ? group.repositories
              : group.repositories.filter((repo) =>
                  repositoryMatchesSearch(repo, normalizedQuery)
                );

            if (repositories.length === 0) return null;
            return {
              ...group,
              repositories,
              totalSkillCount: repositories.reduce(
                (acc, repo) => acc + getEffectiveSkillCount(repo),
                0
              ),
            };
          }

          const repositories = group.repositories.filter((repo) =>
            repositoryMatchesSearch(repo, normalizedQuery)
          );
          if (repositories.length === 0) return null;
          return {
            ...group,
            repositories,
            totalSkillCount: repositories.reduce(
              (acc, repo) => acc + getEffectiveSkillCount(repo),
              0
            ),
          };
        })
        .filter((group): group is RepositorySidebarGroup => group !== null);

      if (groups.length === 0) return null;
      return {
        ...section,
        groups,
        totalSkillCount: groups.reduce((acc, group) => acc + group.totalSkillCount, 0),
      };
    })
    .filter((section): section is RepositorySidebarSection => section !== null);
}

function sortByRepoName(a: SkillRepositoryWithStats, b: SkillRepositoryWithStats): number {
  if (a.pinned !== b.pinned) {
    return a.pinned ? -1 : 1;
  }
  return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
}

function repositoryMatchesSearch(
  repo: SkillRepositoryWithStats,
  normalizedQuery: string
): boolean {
  const fullName = repo.owner && repo.repo ? `${repo.owner}/${repo.repo}` : undefined;
  return buildSearchText([
    repo.owner,
    repo.repo,
    fullName,
    repo.name,
    repo.url,
    repo.id,
  ]).includes(normalizedQuery);
}
