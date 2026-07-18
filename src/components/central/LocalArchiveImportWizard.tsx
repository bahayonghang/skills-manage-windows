import { useCallback } from "react";
import {
  AlertCircle,
  CheckCircle2,
  FileArchive,
  FolderArchive,
  Loader2,
} from "lucide-react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import { GitHubImportFileTree } from "@/components/marketplace/GitHubImportFileTree";
import type { LocalArchiveImportResolution } from "@/types";
import {
  formatLocalArchiveError,
  useLocalArchiveImportStore,
} from "@/stores/localArchiveImportSlice";

export interface LocalArchiveImportWizardProps {
  t: TFunction;
  onAfterImportSuccess: () => Promise<void>;
}

/**
 * Local ZIP import surface. The Zustand controller owns the full business
 * state machine; this component only chooses a file, dispatches actions, and
 * renders the current state.
 */
export function LocalArchiveImportWizard({
  t,
  onAfterImportSuccess,
}: LocalArchiveImportWizardProps) {
  const isOpen = useLocalArchiveImportStore((state) => state.isOpen);
  const step = useLocalArchiveImportStore((state) => state.step);
  const preview = useLocalArchiveImportStore((state) => state.preview);
  const previewError = useLocalArchiveImportStore((state) => state.previewError);
  const importError = useLocalArchiveImportStore((state) => state.importError);
  const isPreviewLoading = useLocalArchiveImportStore(
    (state) => state.isPreviewLoading,
  );
  const isImporting = useLocalArchiveImportStore((state) => state.isImporting);
  const resolution = useLocalArchiveImportStore((state) => state.resolution);
  const renamedSkillId = useLocalArchiveImportStore(
    (state) => state.renamedSkillId,
  );
  const closeWizard = useLocalArchiveImportStore((state) => state.closeWizard);
  const previewArchive = useLocalArchiveImportStore(
    (state) => state.previewArchive,
  );
  const reportPreviewFailure = useLocalArchiveImportStore(
    (state) => state.reportPreviewFailure,
  );
  const importArchive = useLocalArchiveImportStore((state) => state.importArchive);
  const setResolution = useLocalArchiveImportStore(
    (state) => state.setResolution,
  );
  const setRenamedSkillId = useLocalArchiveImportStore(
    (state) => state.setRenamedSkillId,
  );

  const previewErrorMessage = formatLocalArchiveError(previewError, t);
  const importErrorMessage = formatLocalArchiveError(importError, t);
  const skill = preview?.skills[0] ?? null;

  const handleChooseArchive = useCallback(async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (!selected || typeof selected !== "string") return;
      await previewArchive(selected);
    } catch (error) {
      reportPreviewFailure(error);
      const message = formatLocalArchiveError(
        useLocalArchiveImportStore.getState().previewError,
        t,
      );
      if (message) toast.error(message);
    }
  }, [previewArchive, reportPreviewFailure, t]);

  const handleImport = useCallback(async () => {
    try {
      await importArchive();
    } catch {
      const message = formatLocalArchiveError(
        useLocalArchiveImportStore.getState().importError,
        t,
      );
      if (message) toast.error(message);
      return;
    }

    try {
      await onAfterImportSuccess();
    } catch {
      toast.error(t("central.refreshError", {
        error: t("backendErrors.local_archive.unknown"),
      }));
    }
  }, [importArchive, onAfterImportSuccess, t]);

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open) closeWizard();
      }}
    >
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FolderArchive className="size-4" />
            {t("central.localArchiveWizard.title")}
          </DialogTitle>
          <DialogDescription>
            {t("central.localArchiveWizard.description")}
          </DialogDescription>
        </DialogHeader>

        {step === "choose" && (
          <div className="flex flex-col items-center gap-4 py-8">
            {previewErrorMessage && (
              <div className="flex items-start gap-2 rounded-lg bg-destructive/10 p-3 text-sm text-destructive-text">
                <AlertCircle className="size-4 shrink-0" />
                <span>{previewErrorMessage}</span>
              </div>
            )}
            <Button
              variant="outline"
              onClick={handleChooseArchive}
              disabled={isPreviewLoading}
              data-testid="local-archive-choose-file"
            >
              {isPreviewLoading ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <FileArchive className="size-4" />
              )}
              {t("central.localArchiveWizard.chooseFile")}
            </Button>
            <p className="text-xs text-muted-foreground">
              {t("central.localArchiveWizard.chooseFileHint")}
            </p>
          </div>
        )}

        {step === "preview" && skill && preview && (
          <div className="flex flex-col gap-4">
            {importErrorMessage && (
              <div className="flex items-start gap-2 rounded-lg bg-destructive/10 p-3 text-sm text-destructive-text">
                <AlertCircle className="size-4 shrink-0" />
                <span>{importErrorMessage}</span>
              </div>
            )}
            <div className="rounded-lg border border-border/60 p-3">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{skill.skillName}</p>
                  <p className="truncate text-xs text-muted-foreground">
                    id: {skill.skillId}
                  </p>
                </div>
                <span className="max-w-48 truncate text-xs text-muted-foreground">
                  {preview.archiveDisplayName}
                </span>
              </div>
              {skill.description && (
                <p className="mt-1 text-xs text-muted-foreground">
                  {skill.description}
                </p>
              )}
              <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                <span>
                  {t("central.localArchiveWizard.fileCount")}: {skill.fileCount}
                </span>
                <span>
                  {t("central.localArchiveWizard.totalBytes")}: {" "}
                  {skill.totalExpandedBytes} bytes
                </span>
              </div>
            </div>

            {skill.conflict && (
              <div className="rounded-lg border border-warning/40 bg-warning/5 p-3">
                <p className="text-sm font-medium text-warning-foreground">
                  {t("central.localArchiveWizard.conflictDetected")}
                </p>
                <p className="text-xs text-muted-foreground">
                  {t("central.localArchiveWizard.conflictDesc", {
                    existingId: skill.conflict.existingSkillId,
                  })}
                </p>
              </div>
            )}

            <div className="h-56 min-h-0 overflow-hidden rounded-lg border border-border/60 p-3">
              <GitHubImportFileTree
                files={skill.files}
                rootName={skill.rootDirectory || skill.skillId}
              />
            </div>

            <RadioGroup
              value={resolution}
              onValueChange={(value) =>
                setResolution(value as LocalArchiveImportResolution)
              }
              className="flex flex-col gap-2"
            >
              <div className="flex items-center gap-2">
                <RadioItem value="overwrite" id="res-overwrite" />
                <label htmlFor="res-overwrite" className="text-sm">
                  {t("central.localArchiveWizard.resolutionOverwrite")}
                </label>
              </div>
              <div className="flex items-center gap-2">
                <RadioItem value="rename" id="res-rename" />
                <label htmlFor="res-rename" className="text-sm">
                  {t("central.localArchiveWizard.resolutionRename")}
                </label>
              </div>
              {resolution === "rename" && (
                <Input
                  value={renamedSkillId}
                  onChange={(event) => setRenamedSkillId(event.target.value)}
                  placeholder={t("central.localArchiveWizard.renamePlaceholder")}
                  className="mt-1"
                  data-testid="local-archive-rename-input"
                />
              )}
              <div className="flex items-center gap-2">
                <RadioItem value="skip" id="res-skip" />
                <label htmlFor="res-skip" className="text-sm">
                  {t("central.localArchiveWizard.resolutionSkip")}
                </label>
              </div>
            </RadioGroup>
          </div>
        )}

        {step === "importing" && (
          <div className="flex flex-col items-center gap-4 py-8">
            <Loader2 className="size-6 animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">
              {t("central.localArchiveWizard.importing")}
            </p>
          </div>
        )}

        {step === "result" && (
          <div className="flex flex-col items-center gap-4 py-8">
            <CheckCircle2 className="size-6 text-emerald-500" />
            <p className="text-sm font-medium">
              {t("central.localArchiveWizard.importSuccess")}
            </p>
          </div>
        )}

        <DialogFooter>
          {step === "choose" && (
            <Button variant="outline" onClick={closeWizard}>
              {t("common.cancel")}
            </Button>
          )}
          {step === "preview" && (
            <>
              <Button variant="outline" onClick={closeWizard}>
                {t("common.cancel")}
              </Button>
              <Button
                onClick={handleImport}
                disabled={
                  isImporting ||
                  (resolution === "rename" && renamedSkillId.trim().length === 0)
                }
                data-testid="local-archive-import-confirm"
              >
                {t("central.localArchiveWizard.import")}
              </Button>
            </>
          )}
          {step === "result" && (
            <Button onClick={closeWizard}>{t("common.close")}</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
