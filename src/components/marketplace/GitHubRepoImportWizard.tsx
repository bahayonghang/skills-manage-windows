import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
  GitHubSkillImportSelection,
  GitHubSkillPreview,
} from "@/types";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { isTauriRuntime } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { InstallDialog } from "@/components/central/InstallDialog";
import {
  GitHubRepoImportWizardFooter,
  GitHubRepoImportWizardHeader,
} from "@/components/marketplace/GitHubRepoImportWizardChrome";
import {
  GitHubRepoImportConfirmSummary,
  GitHubRepoImportResultHub,
} from "@/components/marketplace/GitHubRepoImportWizardBody";
import { GitHubRepoImportPreviewWorkspace } from "@/components/marketplace/GitHubRepoImportWizardPreview";
import {
  useMarketplaceStore,
} from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  buildInitialSelections,
  clampPercent,
  EMPTY_AI_SUMMARIES,
  EMPTY_SKILL_MARKDOWN,
  type DetailTab,
  type GitHubRepoImportWizardProps,
  looksLikeMissingSshPassword,
  noopFetchGitHubSkillMarkdown,
  noopGenerateGitHubImportAiSummary,
  type SelectionState,
  type WizardStep,
} from "@/components/marketplace/githubImportWizardUtils";

export function GitHubRepoImportWizard({
  open,
  onOpenChange,
  repoUrl,
  onRepoUrlChange,
  preview,
  previewError,
  isPreviewLoading,
  isImporting,
  importResult,
  onPreview,
  onImport,
  onReset,
  launcherLabel,
  availableAgents = [],
  installableSkills = [],
  onInstallImportedSkill,
  onAfterImportSuccess,
  onOpenCentral,
}: GitHubRepoImportWizardProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [step, setStep] = useState<WizardStep>(() =>
    importResult ? "result" : preview ? "preview" : "input",
  );
  const [selectionState, setSelectionState] = useState<
    Record<string, SelectionState>
  >(() => buildInitialSelections(preview));
  const [postImportTargetSkillId, setPostImportTargetSkillId] = useState<
    string | null
  >(null);
  const [selectedSkillPath, setSelectedSkillPath] = useState<string | null>(() =>
    preview?.skills[0]?.sourcePath ?? null,
  );
  const [detailTab, setDetailTab] = useState<DetailTab>("overview");
  const [isRenameEditing, setIsRenameEditing] = useState(false);
  const [sshPasswordRepairValue, setSshPasswordRepairValue] = useState("");
  const [sshPasswordRepairMessage, setSshPasswordRepairMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);
  const [isSavingSshPassword, setIsSavingSshPassword] = useState(false);
  const detailScrollRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const browserMode = !isTauriRuntime();
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
  const [progressNow, setProgressNow] = useState(() => Date.now());

  useEffect(() => {
    if (!open) {
      setStep("input");
      setSelectionState({});
      setPostImportTargetSkillId(null);
      setSelectedSkillPath(null);
      setDetailTab("overview");
      setSshPasswordRepairValue("");
      setSshPasswordRepairMessage(null);
      return;
    }
    if (importResult) {
      setStep("result");
      return;
    }
    if (preview) {
      setSelectionState(buildInitialSelections(preview));
      setSelectedSkillPath((current) =>
        current && preview.skills.some((skill) => skill.sourcePath === current)
          ? current
          : (preview.skills[0]?.sourcePath ?? null),
      );
      setStep("preview");
      return;
    }
    setSelectedSkillPath(null);
    setStep("input");
  }, [open, preview, importResult]);

  useEffect(() => {
    setSshPasswordRepairValue("");
    setSshPasswordRepairMessage(null);
  }, [activeTarget.id]);

  const postImportSkill = useMemo(() => {
    if (!postImportTargetSkillId) return null;
    return (
      installableSkills.find((skill) => skill.id === postImportTargetSkillId) ??
      null
    );
  }, [installableSkills, postImportTargetSkillId]);

  const selectedSkills = useMemo(() => {
    if (!preview) return [];
    return preview.skills.filter(
      (skill) => selectionState[skill.sourcePath]?.selected,
    );
  }, [preview, selectionState]);
  const allSkillsSelected = Boolean(
    preview?.skills.length && selectedSkills.length === preview.skills.length,
  );
  const noSkillsSelected = selectedSkills.length === 0;

  const selectedPreviewSkill = useMemo(() => {
    if (!preview) return null;
    if (selectedSkillPath) {
      return (
        preview.skills.find(
          (skill) => skill.sourcePath === selectedSkillPath,
        ) ?? null
      );
    }
    return preview.skills[0] ?? null;
  }, [preview, selectedSkillPath]);
  const previewToolbarRepoHref = useMemo(() => {
    if (!preview) return null;
    return `https://github.com/${preview.repo.owner}/${preview.repo.repo}`;
  }, [preview]);

  const blockingConflict = useMemo(() => {
    return selectedSkills.find((skill) => {
      if (!skill.conflict) return false;
      const state = selectionState[skill.sourcePath];
      if (!state) return true;
      if (state.resolution === "skip") return false;
      if (state.resolution === "rename") {
        return !state.renamedSkillId.trim();
      }
      return false;
    });
  }, [selectedSkills, selectionState]);

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
  const importProgressPercent = useMemo(() => {
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
  }, [importProgress]);
  const importEtaSeconds = useMemo(() => {
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
  }, [
    importProgress,
    importProgressPercent,
    importStartedAt,
    isImporting,
    progressNow,
  ]);

  useEffect(() => {
    if (step === "preview") {
      detailScrollRef.current?.scrollTo?.({ top: 0, behavior: "auto" });
    }
  }, [selectedSkillPath, step]);

  useEffect(() => {
    if (step === "preview") {
      setDetailTab("overview");
    }
  }, [selectedSkillPath, step]);

  useEffect(() => {
    if (!isImporting || !importProgress || !importStartedAt) {
      return;
    }

    const timer = window.setInterval(() => {
      setProgressNow(Date.now());
    }, 1000);

    return () => window.clearInterval(timer);
  }, [importProgress, importStartedAt, isImporting]);

  useEffect(() => {
    if (!selectedPreviewSkill) {
      setIsRenameEditing(false);
      return;
    }

    const currentSelection = selectionState[selectedPreviewSkill.sourcePath];
    const currentResolution =
      currentSelection?.resolution ??
      (selectedPreviewSkill.conflict ? "skip" : "overwrite");

    if (currentResolution !== "rename") {
      setIsRenameEditing(false);
    }
  }, [selectedPreviewSkill, selectionState]);

  useEffect(() => {
    if (!isRenameEditing) return;
    renameInputRef.current?.focus();
    renameInputRef.current?.select();
  }, [isRenameEditing]);

  useEffect(() => {
    if (step !== "preview") return;
    if (detailTab !== "overview") return;
    if (browserMode) return;
    if (!selectedPreviewSkill) return;
    fetchGitHubSkillMarkdown(
      selectedPreviewSkill.sourcePath,
      selectedPreviewSkill.downloadUrl,
    );
  }, [
    step,
    detailTab,
    browserMode,
    selectedPreviewSkill,
    fetchGitHubSkillMarkdown,
  ]);

  useEffect(() => {
    if (step !== "preview") return;
    if (detailTab !== "ai") return;
    if (!selectedPreviewSkill) return;

    const markdownEntry = skillMarkdown[selectedPreviewSkill.sourcePath];
    const content =
      markdownEntry?.status === "ready" && markdownEntry.content?.trim()
        ? markdownEntry.content
        : selectedPreviewSkill.description?.trim();

    if (!content) return;

    void generateGitHubImportAiSummary(
      selectedPreviewSkill.sourcePath,
      selectedPreviewSkill.skillName,
      content,
      i18n.language,
    );
  }, [
    step,
    detailTab,
    selectedPreviewSkill,
    skillMarkdown,
    generateGitHubImportAiSummary,
    i18n.language,
  ]);

  const selectedImportPayload = useMemo<GitHubSkillImportSelection[]>(() => {
    return selectedSkills.map((skill) => {
      const state = selectionState[skill.sourcePath];
      return {
        sourcePath: skill.sourcePath,
        resolution:
          state?.resolution ?? (skill.conflict ? "skip" : "overwrite"),
        renamedSkillId:
          state?.resolution === "rename"
            ? state.renamedSkillId.trim() || null
            : null,
      };
    });
  }, [selectedSkills, selectionState]);

  const skippedPreviewSkills = useMemo(
    () =>
      preview
        ? preview.skills.filter(
            (skill) => !selectionState[skill.sourcePath]?.selected,
          )
        : [],
    [preview, selectionState],
  );

  const decisionCounts = useMemo(() => {
    const counts = { write: 0, overwrite: 0, rename: 0, skip: 0 };

    selectedSkills.forEach((skill) => {
      const state = selectionState[skill.sourcePath];
      const resolution =
        state?.resolution ?? (skill.conflict ? "skip" : "overwrite");
      if (!skill.conflict) {
        counts.write += 1;
        return;
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
    });

    return counts;
  }, [selectedSkills, selectionState]);

  const activePasswordCredentialStatus =
    activeTarget.kind === "ssh" && activeTarget.authMethod === "password"
      ? (activeTarget.credentialStatus ??
        (activeTarget.hasStoredPassword ? "stored" : "missing"))
      : null;
  const activePasswordCredentialAvailable =
    activePasswordCredentialStatus === "stored" ||
    activePasswordCredentialStatus === "session";
  const activePasswordTargetNeedsPassword =
    activeTarget.kind === "ssh" &&
    activeTarget.authMethod === "password" &&
    !activePasswordCredentialAvailable;
  const showSshPasswordRepairSuccess =
    sshPasswordRepairMessage?.type === "success";
  const missingSshPasswordError =
    previewError && looksLikeMissingSshPassword(previewError);
  const showSshPasswordRepair =
    step === "confirm" &&
    (activePasswordTargetNeedsPassword ||
      showSshPasswordRepairSuccess ||
      (Boolean(missingSshPasswordError) &&
        sshPasswordRepairMessage?.type !== "success"));
  const canReview = selectedSkills.length > 0 && !blockingConflict;
  const canConfirm =
    canReview && decisionCounts.write > 0 && !activePasswordTargetNeedsPassword;

  const renamedSelections = useMemo(
    () =>
      selectedSkills.filter(
        (skill) => selectionState[skill.sourcePath]?.resolution === "rename",
      ),
    [selectedSkills, selectionState],
  );

  const overwriteSelections = useMemo(
    () =>
      selectedSkills.filter(
        (skill) =>
          skill.conflict &&
          selectionState[skill.sourcePath]?.resolution === "overwrite",
      ),
    [selectedSkills, selectionState],
  );

  const skippedConflictSelections = useMemo(
    () =>
      selectedSkills.filter(
        (skill) =>
          skill.conflict &&
          selectionState[skill.sourcePath]?.resolution === "skip",
      ),
    [selectedSkills, selectionState],
  );

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

  async function handlePreviewSubmit() {
    const nextSelectedSkillPath = selectedSkillPath;
    try {
      const nextPreview = await onPreview();
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
      if (looksLikeMissingSshPassword(String(err))) {
        setSshPasswordRepairMessage(null);
        await loadTargets().catch(() => undefined);
      }
    }
  }

  async function handleSaveSshPasswordForImport() {
    if (activeTarget.kind !== "ssh") return;
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
    const result = await onInstallImportedSkill(skillId, agentIds, method, projectPath);
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

  function handleOpenCentralClick() {
    onOpenCentral?.();
    handleClose(false);
    navigate("/central");
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className={dialogContentClassName}>
        <GitHubRepoImportWizardHeader
          launcherLabel={launcherLabel}
          step={step}
          preview={preview}
          showRepoToolbar={showRepoToolbar}
          repoUrl={repoUrl}
          previewError={previewError}
          isPreviewLoading={isPreviewLoading}
          browserMode={browserMode}
          previewToolbarRepoHref={previewToolbarRepoHref}
          selectedSkillsCount={selectedSkills.length}
          activeTarget={activeTarget}
          onRepoUrlChange={onRepoUrlChange}
          onPreviewSubmit={handlePreviewSubmit}
        />

        <div
          className={cn(
            "px-6 py-4",
            showSharedShellBody
              ? "min-h-0 flex-1 overflow-hidden"
              : "overflow-visible",
          )}
        >
          {preview ? (
            step === "confirm" ? (
              <GitHubRepoImportConfirmSummary
                selectedSkills={selectedSkills}
                selectionState={selectionState}
                decisionCounts={decisionCounts}
                skippedPreviewSkills={skippedPreviewSkills}
                overwriteSelections={overwriteSelections}
                renamedSelections={renamedSelections}
                skippedConflictSelections={skippedConflictSelections}
                blockingConflict={blockingConflict}
              />
            ) : step === "result" && importResult ? (
              <GitHubRepoImportResultHub
                importResult={importResult}
                canInstallImportedSkills={Boolean(onInstallImportedSkill)}
                onInstallImported={handleInstallImported}
                onOpenCentral={handleOpenCentralClick}
                onStartAnotherImport={handleStartAnotherImport}
              />
            ) : preview && step !== "result" ? (
              <GitHubRepoImportPreviewWorkspace
                preview={preview}
                selectionState={selectionState}
                selectedPreviewSkill={selectedPreviewSkill}
                detailTab={detailTab}
                isRenameEditing={isRenameEditing}
                browserMode={browserMode}
                allSkillsSelected={allSkillsSelected}
                noSkillsSelected={noSkillsSelected}
                skillMarkdown={skillMarkdown}
                aiSummaries={aiSummaries}
                detailScrollRef={detailScrollRef}
                renameInputRef={renameInputRef}
                onSelectAll={() => updateAllSelections(true)}
                onDeselectAll={() => updateAllSelections(false)}
                onSelectSkillPath={setSelectedSkillPath}
                onToggleSkillSelected={(skill, selected) =>
                  updateSelection(skill, { selected })
                }
                onSetDetailTab={setDetailTab}
                onStartRenameEditing={startRenameEditing}
                onCancelRenameEditing={cancelRenameEditing}
                onConfirmRenameEditing={confirmRenameEditing}
                onUpdateSelection={updateSelection}
                onRetryMarkdown={(sourcePath, downloadUrl) =>
                  void fetchGitHubSkillMarkdown(sourcePath, downloadUrl)
                }
                onRegenerateAiSummary={(sourcePath, skillName, content) =>
                  void generateGitHubImportAiSummary(
                    sourcePath,
                    skillName,
                    content,
                    i18n.language,
                    true,
                  )
                }
              />
            ) : null
          ) : step === "result" && importResult ? (
            <GitHubRepoImportResultHub
              importResult={importResult}
              canInstallImportedSkills={Boolean(onInstallImportedSkill)}
              onInstallImported={handleInstallImported}
              onOpenCentral={handleOpenCentralClick}
              onStartAnotherImport={handleStartAnotherImport}
            />
          ) : null}
        </div>

        <GitHubRepoImportWizardFooter
          step={step}
          showSharedShellBody={showSharedShellBody}
          canReview={canReview}
          canConfirm={canConfirm}
          isImporting={isImporting}
          importProgress={step === "confirm" ? importProgress : null}
          importProgressPercent={importProgressPercent}
          importEtaSeconds={importEtaSeconds}
          showSshPasswordRepair={showSshPasswordRepair}
          activeTarget={activeTarget}
          sshPasswordRepairValue={sshPasswordRepairValue}
          sshPasswordRepairMessage={sshPasswordRepairMessage}
          isSavingSshPassword={isSavingSshPassword}
          onSshPasswordRepairValueChange={setSshPasswordRepairValue}
          onClearSshPasswordRepairError={() => setSshPasswordRepairMessage(null)}
          onSaveSshPasswordForImport={() => {
            void handleSaveSshPasswordForImport();
          }}
          onStartAnotherImport={handleStartAnotherImport}
          onClose={() => handleClose(false)}
          onRetryToInput={() => setStep("input")}
          onReviewImport={() => setStep("confirm")}
          onBackToPreview={() => setStep("preview")}
          onImportConfirm={() => {
            void handleImportConfirmClick();
          }}
        />
      </DialogContent>

      <InstallDialog
        open={Boolean(postImportSkill)}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) {
            setPostImportTargetSkillId(null);
          }
        }}
        skill={postImportSkill}
        agents={availableAgents}
        onInstall={handleInstallDialogConfirm}
      />
    </Dialog>
  );
}
