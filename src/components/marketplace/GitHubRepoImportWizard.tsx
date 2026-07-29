import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { Dialog, DialogContent } from "@/components/ui/dialog";
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
import { isTauriRuntime } from "@/lib/ipc";
import {
  buildInitialSelections,
  type DetailTab,
  type GitHubRepoImportWizardProps,
  type SelectionState,
  type WizardStep,
} from "@/components/marketplace/githubImportWizardUtils";
import { createGitHubImportWizardActions } from "@/components/marketplace/githubImportWizardActions";
import { useGitHubImportWizardBindings } from "@/components/marketplace/githubImportWizardBindings";
import {
  getGitHubImportPreviewState,
  getGitHubImportWizardShellState,
  getInitialGitHubImportWizardStep,
  getPostImportSkill,
  getPreviewToolbarRepoHref,
  type GitHubImportWizardStatusMessage,
} from "@/components/marketplace/githubImportWizardViewModel";

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
  const {
    activeTarget,
    loadTargets,
    updateSshTargetPassword,
    skillMarkdown,
    aiSummaries,
    fetchGitHubSkillMarkdown,
    generateGitHubImportAiSummary,
    importProgress,
    importStartedAt,
  } = useGitHubImportWizardBindings();
  const [step, setStep] = useState<WizardStep>(() =>
    getInitialGitHubImportWizardStep(preview, importResult),
  );
  const [selectionState, setSelectionState] = useState<
    Record<string, SelectionState>
  >(() => buildInitialSelections(preview));
  const [postImportTargetSkillId, setPostImportTargetSkillId] = useState<
    string | null
  >(null);
  const [selectedSkillPath, setSelectedSkillPath] = useState<string | null>(
    () => preview?.skills[0]?.sourcePath ?? null,
  );
  const [detailTab, setDetailTab] = useState<DetailTab>("overview");
  const [isRenameEditing, setIsRenameEditing] = useState(false);
  const [sshPasswordRepairValue, setSshPasswordRepairValue] = useState("");
  const [sshPasswordRepairMessage, setSshPasswordRepairMessage] =
    useState<GitHubImportWizardStatusMessage>(null);
  const [isSavingSshPassword, setIsSavingSshPassword] = useState(false);
  const [progressNow, setProgressNow] = useState(() => Date.now());
  const detailScrollRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const browserMode = !isTauriRuntime();

  const previewState = useMemo(
    () =>
      getGitHubImportPreviewState({
        preview,
        selectionState,
        selectedSkillPath,
      }),
    [preview, selectionState, selectedSkillPath],
  );
  const shellState = useMemo(
    () =>
      getGitHubImportWizardShellState({
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
        selectedSkillsCount: previewState.selectedSkills.length,
        hasBlockingConflict: Boolean(previewState.blockingConflict),
        hasBlockingFileManifest: Boolean(previewState.blockingFileManifest),
        decisionWriteCount: previewState.decisionCounts.write,
      }),
    [
      activeTarget,
      importProgress,
      importResult,
      importStartedAt,
      isImporting,
      preview,
      previewError,
      previewState.blockingConflict,
      previewState.blockingFileManifest,
      previewState.decisionCounts.write,
      previewState.selectedSkills.length,
      progressNow,
      sshPasswordRepairMessage,
      step,
    ],
  );
  const previewToolbarRepoHref = useMemo(
    () => getPreviewToolbarRepoHref(preview),
    [preview],
  );
  const postImportSkill = useMemo(
    () => getPostImportSkill(installableSkills, postImportTargetSkillId),
    [installableSkills, postImportTargetSkillId],
  );
  const {
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
  } = createGitHubImportWizardActions({
    t,
    preview,
    importResult,
    selectionState,
    selectedSkillPath,
    selectedImportPayload: previewState.selectedImportPayload,
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
  });

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

  useEffect(() => {
    if (step === "preview") {
      detailScrollRef.current?.scrollTo?.({ top: 0, behavior: "auto" });
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
    if (!previewState.selectedPreviewSkill) {
      setIsRenameEditing(false);
      return;
    }

    const currentSelection =
      selectionState[previewState.selectedPreviewSkill.sourcePath];
    const currentResolution =
      currentSelection?.resolution ??
      (previewState.selectedPreviewSkill.conflict ? "skip" : "overwrite");

    if (currentResolution !== "rename") {
      setIsRenameEditing(false);
    }
  }, [previewState.selectedPreviewSkill, selectionState]);

  useEffect(() => {
    if (!isRenameEditing) return;
    renameInputRef.current?.focus();
    renameInputRef.current?.select();
  }, [isRenameEditing]);

  useEffect(() => {
    if (step !== "preview") return;
    if (detailTab !== "overview") return;
    if (browserMode) return;
    if (!preview) return;
    if (!previewState.selectedPreviewSkill) return;

    fetchGitHubSkillMarkdown(
      preview.repo,
      previewState.selectedPreviewSkill.sourcePath,
    );
  }, [
    browserMode,
    detailTab,
    fetchGitHubSkillMarkdown,
    preview,
    previewState.selectedPreviewSkill,
    step,
  ]);

  useEffect(() => {
    if (step !== "preview") return;
    if (detailTab !== "ai") return;
    if (!previewState.selectedPreviewSkill) return;

    const markdownEntry =
      skillMarkdown[previewState.selectedPreviewSkill.sourcePath];
    const content =
      markdownEntry?.status === "ready" && markdownEntry.content?.trim()
        ? markdownEntry.content
        : previewState.selectedPreviewSkill.description?.trim();

    if (!content) return;

    void generateGitHubImportAiSummary(
      previewState.selectedPreviewSkill.sourcePath,
      previewState.selectedPreviewSkill.skillName,
      content,
      i18n.language,
    );
  }, [
    detailTab,
    generateGitHubImportAiSummary,
    i18n.language,
    previewState.selectedPreviewSkill,
    skillMarkdown,
    step,
  ]);

  function handleOpenCentralClick() {
    onOpenCentral?.();
    handleClose(false);
    navigate("/central");
  }

  const resultHub = importResult ? (
    <GitHubRepoImportResultHub
      importResult={importResult}
      canInstallImportedSkills={Boolean(onInstallImportedSkill)}
      onInstallImported={handleInstallImported}
      onOpenCentral={handleOpenCentralClick}
      onStartAnotherImport={handleStartAnotherImport}
    />
  ) : null;

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className={shellState.dialogContentClassName}>
        <GitHubRepoImportWizardHeader
          launcherLabel={launcherLabel}
          step={step}
          preview={preview}
          showRepoToolbar={shellState.showRepoToolbar}
          repoUrl={repoUrl}
          previewError={previewError}
          isPreviewLoading={isPreviewLoading}
          browserMode={browserMode}
          previewToolbarRepoHref={previewToolbarRepoHref}
          selectedSkillsCount={previewState.selectedSkills.length}
          activeTarget={activeTarget}
          onRepoUrlChange={onRepoUrlChange}
          onPreviewSubmit={handlePreviewSubmit}
        />

        <div
          className={cn(
            "px-6 py-4",
            shellState.showSharedShellBody
              ? "min-h-0 flex-1 overflow-hidden"
              : "overflow-visible",
          )}
        >
          {preview ? (
            step === "confirm" ? (
              <GitHubRepoImportConfirmSummary
                selectedSkills={previewState.selectedSkills}
                selectionState={selectionState}
                decisionCounts={previewState.decisionCounts}
                skippedPreviewSkills={previewState.skippedPreviewSkills}
                overwriteSelections={previewState.overwriteSelections}
                renamedSelections={previewState.renamedSelections}
                skippedConflictSelections={previewState.skippedConflictSelections}
                blockingConflict={previewState.blockingConflict}
              />
            ) : step === "result" && importResult ? (
              resultHub
            ) : step !== "result" ? (
              <GitHubRepoImportPreviewWorkspace
                preview={preview}
                selectionState={selectionState}
                selectedPreviewSkill={previewState.selectedPreviewSkill}
                detailTab={detailTab}
                isRenameEditing={isRenameEditing}
                browserMode={browserMode}
                allSkillsSelected={previewState.allSkillsSelected}
                noSkillsSelected={previewState.noSkillsSelected}
                blockingFileManifest={previewState.blockingFileManifest}
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
                onRetryMarkdown={(sourcePath) => {
                  void fetchGitHubSkillMarkdown(preview.repo, sourcePath);
                }}
                onRegenerateAiSummary={(sourcePath, skillName, content) => {
                  void generateGitHubImportAiSummary(
                    sourcePath,
                    skillName,
                    content,
                    i18n.language,
                    true,
                  );
                }}
              />
            ) : null
          ) : step === "result" && importResult ? (
            resultHub
          ) : null}
        </div>

        <GitHubRepoImportWizardFooter
          step={step}
          showSharedShellBody={shellState.showSharedShellBody}
          canReview={shellState.canReview}
          canConfirm={shellState.canConfirm}
          isImporting={isImporting}
          importProgress={step === "confirm" ? importProgress : null}
          importProgressPercent={shellState.importProgressPercent}
          importEtaSeconds={shellState.importEtaSeconds}
          showSshPasswordRepair={shellState.showSshPasswordRepair}
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
