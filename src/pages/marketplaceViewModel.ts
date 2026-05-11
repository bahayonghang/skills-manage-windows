import { useMemo } from "react";

import {
  OFFICIAL_PUBLISHERS,
  RECOMMENDED_SKILLS,
  type OfficialPublisher,
  type SkillTag,
} from "@/data/officialSources";
import type {
  AgentWithStatus,
  GitHubRepoImportResult,
  MarketplaceSkill,
  SkillRegistry,
  SkillWithLinks,
} from "@/types";

export type MarketplaceTabId = "recommended" | "official";

export type MarketplacePreviewStatus =
  | { kind: "idle" }
  | { kind: "browser-fallback"; title: string; detail: string }
  | { kind: "error"; title: string; detail: string };

export type MarketplacePreviewSkill = {
  id: string;
  name: string;
  description?: string;
  downloadUrl: string;
};

export interface MarketplaceViewModel {
  availableInstallAgents: AgentWithStatus[];
  filteredPublishers: OfficialPublisher[];
  filteredRecommended: typeof RECOMMENDED_SKILLS;
  installableImportedSkills: SkillWithLinks[];
  tabs: Array<{ id: MarketplaceTabId; label: string }>;
}

export function mapRegistrySkillToPreviewSkill(
  skill: MarketplaceSkill
): MarketplacePreviewSkill {
  return {
    id: skill.id,
    name: skill.name,
    description: skill.description ?? undefined,
    downloadUrl: skill.download_url,
  };
}

export function mapGitHubPreviewSkillToPreviewSkill(
  skill: {
    skillId: string;
    skillName: string;
    description?: string | null;
    downloadUrl: string;
  }
): MarketplacePreviewSkill {
  return {
    id: skill.skillId,
    name: skill.skillName,
    description: skill.description ?? undefined,
    downloadUrl: skill.downloadUrl,
  };
}

export function findPreviewRegistryId({
  getNormalizedRegistryIdentity,
  registries,
  repoUrl,
}: {
  getNormalizedRegistryIdentity: (url: string) => string | null;
  registries: SkillRegistry[];
  repoUrl: string;
}): string | null {
  const normalizedRepoIdentity = getNormalizedRegistryIdentity(repoUrl);
  if (!normalizedRepoIdentity) return null;

  return (
    registries.find((registry) => {
      const registryIdentity =
        registry.normalized_url ?? getNormalizedRegistryIdentity(registry.url);
      return registryIdentity === normalizedRepoIdentity;
    })?.id ?? null
  );
}

export function useMarketplaceViewModel({
  centralAgents,
  centralSkills,
  githubImportResult,
  lang,
  platformAgents,
  publisherSearch,
  recommendedSearch,
  selectedTag,
}: {
  centralAgents: AgentWithStatus[];
  centralSkills: SkillWithLinks[];
  githubImportResult: GitHubRepoImportResult | null;
  lang: string;
  platformAgents: AgentWithStatus[];
  publisherSearch: string;
  recommendedSearch: string;
  selectedTag: SkillTag | null;
}): MarketplaceViewModel {
  const filteredRecommended = useMemo(() => {
    let list = RECOMMENDED_SKILLS;
    if (selectedTag) {
      list = list.filter((skill) => skill.tags.includes(selectedTag));
    }
    if (recommendedSearch.trim()) {
      const query = recommendedSearch.toLowerCase();
      list = list.filter(
        (skill) =>
          skill.name.toLowerCase().includes(query) ||
          skill.description.toLowerCase().includes(query) ||
          skill.publisher.toLowerCase().includes(query)
      );
    }
    return list;
  }, [recommendedSearch, selectedTag]);

  const filteredPublishers = useMemo(() => {
    if (!publisherSearch.trim()) return OFFICIAL_PUBLISHERS;
    const query = publisherSearch.toLowerCase();
    return OFFICIAL_PUBLISHERS.filter(
      (publisher) =>
        publisher.name.toLowerCase().includes(query) ||
        publisher.slug.toLowerCase().includes(query)
    );
  }, [publisherSearch]);

  const installableImportedSkills = useMemo(() => {
    if (!githubImportResult) return [];
    const importedIds = new Set(
      githubImportResult.importedSkills.map((skill) => skill.importedSkillId)
    );
    return centralSkills.filter((skill) => importedIds.has(skill.id));
  }, [centralSkills, githubImportResult]);

  const availableInstallAgents = useMemo(
    () => (centralAgents.length > 0 ? centralAgents : platformAgents),
    [centralAgents, platformAgents]
  );

  const tabs = useMemo(
    () => [
      { id: "recommended" as const, label: lang === "zh" ? "推荐" : "Recommended" },
      {
        id: "official" as const,
        label: lang === "zh" ? "官方源目录" : "Official Directory",
      },
    ],
    [lang]
  );

  return {
    availableInstallAgents,
    filteredPublishers,
    filteredRecommended,
    installableImportedSkills,
    tabs,
  };
}
