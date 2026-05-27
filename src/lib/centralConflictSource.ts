import type {
  SkillWithLinks,
} from "@/types";

type RepositoryDisplayLike = {
  name?: string | null;
  owner?: string | null;
  repo?: string | null;
  is_unknown?: boolean | null;
};

type RepositoryWithId = RepositoryDisplayLike & {
  id: string;
};

export interface SkillConflictSourceInfo {
  skillId: string;
  skillName: string;
  repositoryLabel: string | null;
  sourcePath: string | null;
}

export function repositoryDisplayName(
  repository?: RepositoryDisplayLike | null,
): string | null {
  if (!repository) return null;
  if (repository.is_unknown) return null;
  const owner = repository.owner?.trim();
  const repo = repository.repo?.trim();
  if (owner && repo) return `${owner}/${repo}`;
  const name = repository.name?.trim();
  return name || null;
}

export function buildRepositoryDisplayNameMap(
  repositories: readonly RepositoryWithId[],
): Map<string, string> {
  return new Map(
    repositories.map((repository) => [
      repository.id,
      repositoryDisplayName(repository) ?? repository.id,
    ]),
  );
}

export function buildSkillConflictSourceMap(
  skills: readonly Pick<
    SkillWithLinks,
    "id" | "name" | "repository" | "source_path"
  >[],
): Map<string, SkillConflictSourceInfo> {
  return new Map(
    skills.map((skill) => [
      skill.id,
      {
        skillId: skill.id,
        skillName: skill.name,
        repositoryLabel: repositoryDisplayName(skill.repository),
        sourcePath: skill.source_path ?? null,
      },
    ]),
  );
}

export function formatConflictSourceLabel(
  repositoryLabel: string | null | undefined,
  sourcePath: string | null | undefined,
  unassignedLabel: string,
): string {
  const repo = repositoryLabel?.trim();
  if (!repo) return unassignedLabel;
  const path = sourcePath?.trim();
  return path ? `${repo}/${path}` : repo;
}
