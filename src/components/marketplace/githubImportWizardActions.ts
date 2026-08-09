import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";

import type {
  BatchInstallResult,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  GitHubSkillPreview,
  SshTargetTestResult,
  TargetSummary,
} from "@/types";
import {
  looksLikeMissingSshPassword,
  type SelectionState,
  type WizardStep,
} from "@/components/marketplace/githubImportWizardUtils";
import type { GitHubImportWizardStatusMessage } from "@/components/marketplace/githubImportWizardViewModel";
import { isSshTarget } from "@/lib/targetKind";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export function createGitHubImportWizardActions({
  t,
  preview,
  importResult,
  selectionState,
  selectedSkillPath,
  selectedImportPayload,
  activeTarget,
  sshPasswordRepairValue,
  onPreview,
  onImport,
  onReset,
  onOpenChange,
  onAfterImportSuccess,
  onInstallImportedSkill,
  loadTargets,
  updateSshTargetPassword,
  setStep,
  setSelectionState,
  setPostImportTargetSkillId,
  setSelectedSkillPath,
  setIsRenameEditing,
  setSshPasswordRepairValue,
  setSshPasswordRepairMessage,
  setIsSavingSshPassword,
}: {
  t: TFunction;
  preview: GitHubRepoPreview | null;
  importResult: GitHubRepoImportResult | null;
  selectionState: Record<string, SelectionState>;
  selectedSkillPath: string | null;
  selectedImportPayload: GitHubSkillImportSelection[];
  activeTarget: TargetSummary;
  sshPasswordRepairValue: string;
  onPreview: (
    branch: string,
  ) => Promise<GitHubRepoPreview | null> | GitHubRepoPreview | null;
  onImport: (
    selections: GitHubSkillImportSelection[],
  ) => Promise<GitHubRepoImportResult | void> | GitHubRepoImportResult | void;
  onReset: () => void;
  onOpenChange: (open: boolean) => void;
  onAfterImportSuccess?: (result: GitHubRepoImportResult) => Promise<void> | void;
  onInstallImportedSkill?: (
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) => Promise<BatchInstallResult>;
  loadTargets: () => Promise<void>;
  updateSshTargetPassword: (
    targetId: string,
    password: string,
  ) => Promise<SshTargetTestResult>;
  setStep: StateSetter<WizardStep>;
  setSelectionState: StateSetter<Record<string, SelectionState>>;
  setPostImportTargetSkillId: StateSetter<string | null>;
  setSelectedSkillPath: StateSetter<string | null>;
  setIsRenameEditing: StateSetter<boolean>;
  setSshPasswordRepairValue: StateSetter<string>;
  setSshPasswordRepairMessage: StateSetter<GitHubImportWizardStatusMessage>;
  setIsSavingSshPassword: StateSetter<boolean>;
}) {
  function updateSelection(
    skill: GitHubSkillPreview,
    next: Partial<SelectionState>,
  ) {
    setSelectionState((current) => ({
      ...current,
      [skill.sourcePath]: {
        ...current[skill.sourcePath],
        ...next,
      },
    }));
  }

  function updateAllSelections(selected: boolean) {
    if (!preview) return;

    setSelectionState((current) => {
      const next = { ...current };
      preview.skills.forEach((skill) => {
        const existing = current[skill.sourcePath];
        next[skill.sourcePath] = {
          selected,
          resolution:
            existing?.resolution ?? (skill.conflict ? "skip" : "overwrite"),
          renamedSkillId: existing?.renamedSkillId ?? skill.skillId,
        };
      });
      return next;
    });
  }

  function startRenameEditing(skill: GitHubSkillPreview) {
    updateSelection(skill, {
      resolution: "rename",
      renamedSkillId:
        selectionState[skill.sourcePath]?.renamedSkillId || skill.skillId,
    });
    setIsRenameEditing(true);
  }

  function cancelRenameEditing(skill: GitHubSkillPreview) {
    const currentResolution =
      selectionState[skill.sourcePath]?.resolution ??
      (skill.conflict ? "skip" : "overwrite");

    if (currentResolution === "rename") {
      updateSelection(skill, { resolution: "skip" });
    }
    setIsRenameEditing(false);
  }

  function confirmRenameEditing(skill: GitHubSkillPreview) {
    const nextId =
      selectionState[skill.sourcePath]?.renamedSkillId?.trim() || skill.skillId;
    updateSelection(skill, {
      resolution: "rename",
      renamedSkillId: nextId,
    });
    setIsRenameEditing(false);
  }

  async function handlePreviewSubmit(branch: string) {
    const nextSelectedSkillPath = selectedSkillPath;
    try {
      const nextPreview = await onPreview(branch);
      if (nextPreview) {
        if (!preview) {
          setSelectedSkillPath(nextSelectedSkillPath);
        }
        setStep("preview");
        return;
      }
    } catch {
      // keep input step; recoverable error is rendered below the URL input
    }
    setStep("input");
  }

  function handleClose(nextOpen: boolean) {
    if (!nextOpen) {
      setPostImportTargetSkillId(null);
      onReset();
    }
    onOpenChange(nextOpen);
  }

  async function handleImportConfirmClick() {
    try {
      const result = await onImport(selectedImportPayload);
      if (result) {
        await onAfterImportSuccess?.(result);
      } else if (importResult) {
        await onAfterImportSuccess?.(importResult);
      }
    } catch (err) {
      if (looksLikeMissingSshPassword(err)) {
        setSshPasswordRepairMessage(null);
        await loadTargets().catch(() => undefined);
      }
    }
  }

  async function handleSaveSshPasswordForImport() {
    if (!isSshTarget(activeTarget)) return;
    const password = sshPasswordRepairValue.trim();
    if (!password) {
      setSshPasswordRepairMessage({
        type: "error",
        text: t("marketplace.githubImportSshPasswordRequired"),
      });
      return;
    }

    setIsSavingSshPassword(true);
    setSshPasswordRepairMessage(null);
    try {
      const result = await updateSshTargetPassword(activeTarget.id, password);
      if (!result.ok) {
        setSshPasswordRepairMessage({
          type: "error",
          text: result.message,
        });
        return;
      }

      const credentialAvailable =
        result.credentialStatus === "stored" ||
        result.credentialStatus === "session" ||
        (!result.credentialStatus && result.ok);
      if (!credentialAvailable) {
        setSshPasswordRepairMessage({
          type: "error",
          text: result.credentialError || result.message,
        });
        return;
      }

      setSshPasswordRepairValue("");
      setSshPasswordRepairMessage({
        type: "success",
        text:
          result.credentialStatus === "session"
            ? t("marketplace.githubImportSshPasswordSession", {
                label: activeTarget.label,
              })
            : t("marketplace.githubImportSshPasswordSaved", {
                label: activeTarget.label,
              }),
      });
    } catch (err) {
      setSshPasswordRepairMessage({
        type: "error",
        text: String(err),
      });
    } finally {
      setIsSavingSshPassword(false);
    }
  }

  function handleInstallImported(skillId: string) {
    setPostImportTargetSkillId(skillId);
  }

  async function handleInstallDialogConfirm(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) {
    if (!onInstallImportedSkill) {
      return { succeeded: [], failed: [] };
    }

    const result = await onInstallImportedSkill(
      skillId,
      agentIds,
      method,
      projectPath,
    );
    if (result.failed.length === 0) {
      setPostImportTargetSkillId(null);
    }
    return result;
  }

  function handleStartAnotherImport() {
    setSelectionState({});
    setPostImportTargetSkillId(null);
    setSelectedSkillPath(null);
    onReset();
    setStep("input");
  }

  return {
    updateSelection,
    updateAllSelections,
    startRenameEditing,
    cancelRenameEditing,
    confirmRenameEditing,
    handlePreviewSubmit,
    handleClose,
    handleImportConfirmClick,
    handleSaveSshPasswordForImport,
    handleInstallImported,
    handleInstallDialogConfirm,
    handleStartAnotherImport,
  };
}
