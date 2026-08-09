import type { TFunction } from "i18next";
import { toast } from "sonner";

import { formatGitHubImportToast } from "@/components/marketplace/githubImportWizardUtils";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import type { GitHubSkillImportSelection } from "@/types";

export interface CentralSkillsImportWorkflowDeps {
  t: TFunction;
  githubRepoUrl: string;
}

export function useCentralSkillsImportWorkflow({
  t,
  githubRepoUrl,
}: CentralSkillsImportWorkflowDeps) {
  const skillsByAgent = useSkillStore((store) => store.skillsByAgent);
  const getSkillsByAgent = useSkillStore((store) => store.getSkillsByAgent);
  const previewGitHubRepoImport = useMarketplaceStore(
    (store) => store.previewGitHubRepoImport,
  );
  const importGitHubRepoSkills = useMarketplaceStore(
    (store) => store.importGitHubRepoSkills,
  );
  const installSkill = useCentralSkillsStore((store) => store.installSkill);
  const loadCentralSkills = useCentralSkillsStore(
    (store) => store.loadCentralSkills,
  );
  const refreshCounts = usePlatformStore((store) => store.refreshCounts);

  async function handleGitHubPreview(branch: string) {
    try {
      return await previewGitHubRepoImport(githubRepoUrl, branch);
    } catch {
      return null;
    }
  }

  async function handleGitHubImport(selections: GitHubSkillImportSelection[]) {
    try {
      const result = await importGitHubRepoSkills(githubRepoUrl, selections);
      await Promise.all([refreshCounts(), loadCentralSkills()]);
      toast.success(t("marketplace.githubImportCentralSuccess"));
      return result;
    } catch (err) {
      // Snapshot lifecycle failures arrive as a coded envelope; localize them so
      // the toast tells the user to preview the repository again.
      toast.error(
        t("marketplace.githubImportError", {
          error: formatGitHubImportToast(err, t),
        }),
      );
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) {
    const result = await installSkill(skillId, agentIds, method, projectPath);
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
    return result;
  }

  async function handleAfterImportSuccess() {
    const agentIds = Object.keys(skillsByAgent);
    if (agentIds.length === 0) return;
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
  }

  return {
    handleAfterImportSuccess,
    handleGitHubImport,
    handleGitHubPreview,
    handleInstallImportedSkill,
  };
}
