import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  EMPTY_AI_SUMMARIES,
  EMPTY_SKILL_MARKDOWN,
  noopFetchGitHubSkillMarkdown,
  noopGenerateGitHubImportAiSummary,
} from "@/components/marketplace/githubImportWizardUtils";

export function useGitHubImportWizardBindings() {
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const loadTargets = useTargetStore((state) => state.loadTargets);
  const updateSshTargetPassword = useTargetStore(
    (state) => state.updateSshTargetPassword,
  );
  const skillMarkdown = useMarketplaceStore(
    (state) => state.githubImport.skillMarkdown,
  ) ?? EMPTY_SKILL_MARKDOWN;
  const aiSummaries = useMarketplaceStore(
    (state) => state.githubImport.aiSummaries,
  ) ?? EMPTY_AI_SUMMARIES;
  const fetchGitHubSkillMarkdown = useMarketplaceStore(
    (state) => state.fetchGitHubSkillMarkdown,
  ) ?? noopFetchGitHubSkillMarkdown;
  const generateGitHubImportAiSummary = useMarketplaceStore(
    (state) => state.generateGitHubImportAiSummary,
  ) ?? noopGenerateGitHubImportAiSummary;
  const importProgress = useMarketplaceStore(
    (state) => state.githubImport.importProgress,
  ) ?? null;
  const importStartedAt = useMarketplaceStore(
    (state) => state.githubImport.importStartedAt,
  ) ?? null;

  return {
    activeTarget,
    loadTargets,
    updateSshTargetPassword,
    skillMarkdown,
    aiSummaries,
    fetchGitHubSkillMarkdown,
    generateGitHubImportAiSummary,
    importProgress,
    importStartedAt,
  };
}
