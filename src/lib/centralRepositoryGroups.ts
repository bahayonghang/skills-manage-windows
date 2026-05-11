/**
 * 仓库分组：在 sidebar 里把 GitHub 仓库按 owner 折叠，本地 / 未来源单列一组。
 *
 * UX 目标：用户面对 13+ 仓库时可一眼看清来源类型，并按 owner 折叠减少视觉噪音。
 */

import type { SkillRepositoryWithStats } from "@/types";

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

function sortByRepoName(a: SkillRepositoryWithStats, b: SkillRepositoryWithStats): number {
  return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
}
