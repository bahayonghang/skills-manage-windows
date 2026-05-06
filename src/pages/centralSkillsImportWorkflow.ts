import type { TFunction } from "i18next";
import { toast } from "sonner";

import type {
  BatchInstallResult,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  ScannedSkill,
} from "@/types";

export interface CentralSkillsImportWorkflowDeps {
  t: TFunction;

  githubRepoUrl: string;
  skillsByAgent: Record<string, ScannedSkill[]>;

  getSkillsByAgent: (agentId: string) => Promise<void>;
  importGitHubRepoSkills: (
    repoUrl: string,
    selections: GitHubSkillImportSelection[]
  ) => Promise<GitHubRepoImportResult>;
  installSkill: (
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) => Promise<BatchInstallResult>;
  loadCentralSkills: () => Promise<void>;
  previewGitHubRepoImport: (repoUrl: string) => Promise<GitHubRepoPreview | null>;
  refreshCounts: () => Promise<void>;
}

export function createCentralSkillsImportWorkflow(
  deps: CentralSkillsImportWorkflowDeps
) {
  const {
    t,
    githubRepoUrl,
    skillsByAgent,
    getSkillsByAgent,
    importGitHubRepoSkills,
    installSkill,
    loadCentralSkills,
    previewGitHubRepoImport,
    refreshCounts,
  } = deps;

  async function handleGitHubPreview() {
    try {
      return await previewGitHubRepoImport(githubRepoUrl);
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
      toast.error(t("marketplace.githubImportError", { error: String(err) }));
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
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
