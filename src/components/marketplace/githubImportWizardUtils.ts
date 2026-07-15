import type {
  AgentWithStatus,
  BatchInstallResult,
  DuplicateResolution,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  SkillWithLinks,
} from "@/types";
import type {
  GitHubImportAiSummaryEntry,
  SkillMarkdownEntry,
} from "@/stores/marketplaceStore";

export type WizardStep = "input" | "preview" | "confirm" | "result";

export type SelectionState = {
  selected: boolean;
  resolution: DuplicateResolution;
  renamedSkillId: string;
};

export type DetailTab = "overview" | "files" | "ai";

export interface GitHubRepoImportWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  repoUrl: string;
  onRepoUrlChange: (value: string) => void;
  preview: GitHubRepoPreview | null;
  previewError: string | null;
  isPreviewLoading: boolean;
  isImporting: boolean;
  importResult: GitHubRepoImportResult | null;
  onPreview: () => Promise<GitHubRepoPreview | null> | GitHubRepoPreview | null;
  onImport: (
    selections: GitHubSkillImportSelection[],
  ) => Promise<GitHubRepoImportResult | void> | GitHubRepoImportResult | void;
  onReset: () => void;
  launcherLabel: string;
  availableAgents?: AgentWithStatus[];
  installableSkills?: SkillWithLinks[];
  onInstallImportedSkill?: (
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) => Promise<BatchInstallResult>;
  onAfterImportSuccess?: (
    result: GitHubRepoImportResult,
  ) => Promise<void> | void;
  onOpenCentral?: () => void;
}

export const EMPTY_SKILL_MARKDOWN: Record<string, SkillMarkdownEntry> = {};
export const EMPTY_AI_SUMMARIES: Record<string, GitHubImportAiSummaryEntry> = {};

export async function noopFetchGitHubSkillMarkdown(
  _sourcePath: string,
  _downloadUrl: string,
): Promise<void> {}

export async function noopGenerateGitHubImportAiSummary(
  _sourcePath: string,
  _skillName: string,
  _content: string,
  _lang: string,
  _refresh?: boolean,
): Promise<void> {}

export function buildInitialSelections(
  preview: GitHubRepoPreview | null,
): Record<string, SelectionState> {
  if (!preview) return {};
  return Object.fromEntries(
    preview.skills.map((skill) => [
      skill.sourcePath,
      {
        selected: true,
        resolution: skill.conflict ? "skip" : "overwrite",
        renamedSkillId: skill.skillId,
      },
    ]),
  );
}

export function normalizeMessage(message: string) {
  return message.replace(/^Error:\s*/, "");
}

export function looksLikeGitHubAuthGuidance(message: string) {
  return /rate limit|personal access token|\bpat\b|github denied access|requires authentication|configured github token/i.test(
    message,
  );
}

export function looksLikeConfiguredGitHubTokenFailure(message: string) {
  return /configured github token was used/i.test(message);
}

export function looksLikeMissingSshPassword(message: string) {
  return /ssh password for target .* is not available/i.test(message);
}

export function clampPercent(value: number) {
  return Math.max(0, Math.min(100, value));
}
