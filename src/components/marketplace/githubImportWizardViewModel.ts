import { cn } from "@/lib/utils";
import type {
  GitHubImportProgressPayload,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  GitHubSkillPreview,
  SkillWithLinks,
  TargetSummary,
} from "@/types";
import {
  clampPercent,
  looksLikeMissingSshPassword,
  type SelectionState,
  type WizardStep,
} from "@/components/marketplace/githubImportWizardUtils";
import { requiresSshPasswordRepair } from "@/lib/targetKind";

export interface GitHubImportDecisionCounts {
  write: number;
  overwrite: number;
  rename: number;
  skip: number;
}

export type GitHubImportWizardStatusMessage = {
  type: "success" | "error";
  text: string;
} | null;

export interface GitHubImportPreviewState {
  selectedSkills: GitHubSkillPreview[];
  allSkillsSelected: boolean;
  noSkillsSelected: boolean;
  selectedPreviewSkill: GitHubSkillPreview | null;
  blockingConflict: GitHubSkillPreview | undefined;
  selectedImportPayload: GitHubSkillImportSelection[];
  skippedPreviewSkills: GitHubSkillPreview[];
  decisionCounts: GitHubImportDecisionCounts;
  renamedSelections: GitHubSkillPreview[];
  overwriteSelections: GitHubSkillPreview[];
  skippedConflictSelections: GitHubSkillPreview[];
}

export function getInitialGitHubImportWizardStep(
  preview: GitHubRepoPreview | null,
  importResult: GitHubRepoImportResult | null,
) {
  return importResult ? "result" : preview ? "preview" : "input";
}

export function getPostImportSkill(
  installableSkills: SkillWithLinks[],
  postImportTargetSkillId: string | null,
) {
  if (!postImportTargetSkillId) return null;
  return (
    installableSkills.find((skill) => skill.id === postImportTargetSkillId) ??
    null
  );
}

export function getPreviewToolbarRepoHref(preview: GitHubRepoPreview | null) {
  if (!preview) return null;
  return `https://github.com/${preview.repo.owner}/${preview.repo.repo}`;
}

export function getGitHubImportPreviewState({
  preview,
  selectionState,
  selectedSkillPath,
}: {
  preview: GitHubRepoPreview | null;
  selectionState: Record<string, SelectionState>;
  selectedSkillPath: string | null;
}): GitHubImportPreviewState {
  const selectedSkills = preview
    ? preview.skills.filter((skill) => selectionState[skill.sourcePath]?.selected)
    : [];
  const allSkillsSelected = Boolean(
    preview?.skills.length && selectedSkills.length === preview.skills.length,
  );
  const noSkillsSelected = selectedSkills.length === 0;
  const selectedPreviewSkill = preview
    ? selectedSkillPath
      ? preview.skills.find((skill) => skill.sourcePath === selectedSkillPath) ??
        null
      : preview.skills[0] ?? null
    : null;
  const blockingConflict = selectedSkills.find((skill) => {
    if (!skill.conflict) return false;
    const state = selectionState[skill.sourcePath];
    if (!state) return true;
    if (state.resolution === "skip") return false;
    if (state.resolution === "rename") {
      return !state.renamedSkillId.trim();
    }
    return false;
  });
  const selectedImportPayload = selectedSkills.map((skill) => {
    const state = selectionState[skill.sourcePath];
    return {
      sourcePath: skill.sourcePath,
      resolution: state?.resolution ?? (skill.conflict ? "skip" : "overwrite"),
      renamedSkillId:
        state?.resolution === "rename"
          ? state.renamedSkillId.trim() || null
          : null,
    };
  });
  const skippedPreviewSkills = preview
    ? preview.skills.filter((skill) => !selectionState[skill.sourcePath]?.selected)
    : [];
  const decisionCounts = selectedSkills.reduce<GitHubImportDecisionCounts>(
    (counts, skill) => {
      const state = selectionState[skill.sourcePath];
      const resolution =
        state?.resolution ?? (skill.conflict ? "skip" : "overwrite");

      if (!skill.conflict) {
        counts.write += 1;
        return counts;
      }

      if (resolution === "overwrite") {
        counts.overwrite += 1;
        counts.write += 1;
      } else if (resolution === "rename") {
        counts.rename += 1;
        counts.write += 1;
      } else {
        counts.skip += 1;
      }

      return counts;
    },
    { write: 0, overwrite: 0, rename: 0, skip: 0 },
  );
  const renamedSelections = selectedSkills.filter(
    (skill) => selectionState[skill.sourcePath]?.resolution === "rename",
  );
  const overwriteSelections = selectedSkills.filter(
    (skill) =>
      skill.conflict &&
      selectionState[skill.sourcePath]?.resolution === "overwrite",
  );
  const skippedConflictSelections = selectedSkills.filter(
    (skill) =>
      skill.conflict &&
      selectionState[skill.sourcePath]?.resolution === "skip",
  );

  return {
    selectedSkills,
    allSkillsSelected,
    noSkillsSelected,
    selectedPreviewSkill,
    blockingConflict,
    selectedImportPayload,
    skippedPreviewSkills,
    decisionCounts,
    renamedSelections,
    overwriteSelections,
    skippedConflictSelections,
  };
}

export function getGitHubImportWizardShellState({
  step,
  preview,
  importResult,
  isImporting,
  importProgress,
  importStartedAt,
  progressNow,
  activeTarget,
  sshPasswordRepairMessage,
  previewError,
  selectedSkillsCount,
  hasBlockingConflict,
  decisionWriteCount,
}: {
  step: WizardStep;
  preview: GitHubRepoPreview | null;
  importResult: GitHubRepoImportResult | null;
  isImporting: boolean;
  importProgress: GitHubImportProgressPayload | null;
  importStartedAt: number | null;
  progressNow: number;
  activeTarget: TargetSummary;
  sshPasswordRepairMessage: GitHubImportWizardStatusMessage;
  previewError: string | null;
  selectedSkillsCount: number;
  hasBlockingConflict: boolean;
  decisionWriteCount: number;
}) {
  const isInputStep = step === "input" && !preview && !importResult;
  const showRepoToolbar =
    Boolean(preview) && (step === "preview" || step === "confirm");
  const showSharedShellBody = Boolean(preview || importResult);
  const dialogContentClassName = cn(
    "flex flex-col overflow-hidden p-0",
    isInputStep
      ? "h-auto max-h-[min(92vh,32rem)] !w-[min(92vw,48rem)] !max-w-[min(92vw,48rem)]"
      : "h-[min(90vh,760px)] !w-[min(94vw,1180px)] !max-w-[min(94vw,1180px)] xl:!w-[min(95vw,1280px)] xl:!max-w-[min(95vw,1280px)]",
  );
  const importProgressPercent = getImportProgressPercent(importProgress);
  const importEtaSeconds = getImportEtaSeconds({
    isImporting,
    importProgress,
    importStartedAt,
    importProgressPercent,
    progressNow,
  });
  const activePasswordCredentialStatus = requiresSshPasswordRepair(activeTarget)
    ? (activeTarget.credentialStatus ??
      (activeTarget.hasStoredPassword ? "stored" : "missing"))
    : null;
  const activePasswordCredentialAvailable =
    activePasswordCredentialStatus === "stored" ||
    activePasswordCredentialStatus === "session";
  const activePasswordTargetNeedsPassword =
    requiresSshPasswordRepair(activeTarget) && !activePasswordCredentialAvailable;
  const showSshPasswordRepairSuccess = sshPasswordRepairMessage?.type === "success";
  const missingSshPasswordError =
    previewError && looksLikeMissingSshPassword(previewError);
  const showSshPasswordRepair =
    step === "confirm" &&
    (activePasswordTargetNeedsPassword ||
      showSshPasswordRepairSuccess ||
      (Boolean(missingSshPasswordError) &&
        sshPasswordRepairMessage?.type !== "success"));
  const canReview = selectedSkillsCount > 0 && !hasBlockingConflict;
  const canConfirm =
    canReview && decisionWriteCount > 0 && !activePasswordTargetNeedsPassword;

  return {
    isInputStep,
    showRepoToolbar,
    showSharedShellBody,
    dialogContentClassName,
    importProgressPercent,
    importEtaSeconds,
    activePasswordTargetNeedsPassword,
    showSshPasswordRepair,
    canReview,
    canConfirm,
  };
}

function getImportProgressPercent(importProgress: GitHubImportProgressPayload | null) {
  if (!importProgress) return 0;
  if (importProgress.totalBytes > 0) {
    return clampPercent(
      (importProgress.completedBytes / importProgress.totalBytes) * 100,
    );
  }
  if (importProgress.totalFiles > 0) {
    return clampPercent(
      (importProgress.completedFiles / importProgress.totalFiles) * 100,
    );
  }
  return importProgress.phase === "finalizing" ? 100 : 0;
}

function getImportEtaSeconds({
  isImporting,
  importProgress,
  importStartedAt,
  importProgressPercent,
  progressNow,
}: {
  isImporting: boolean;
  importProgress: GitHubImportProgressPayload | null;
  importStartedAt: number | null;
  importProgressPercent: number;
  progressNow: number;
}) {
  if (
    !isImporting ||
    !importProgress ||
    !importStartedAt ||
    importProgressPercent <= 0 ||
    importProgressPercent >= 100
  ) {
    return null;
  }

  const elapsedMs = Math.max(0, progressNow - importStartedAt);
  if (elapsedMs < 1000) return null;

  const remainingRatio = (100 - importProgressPercent) / importProgressPercent;
  const etaSeconds = Math.ceil((elapsedMs * remainingRatio) / 1000);
  return Number.isFinite(etaSeconds) && etaSeconds > 0 ? etaSeconds : null;
}
