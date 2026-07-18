import { useCallback, useEffect, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  FileArchive,
  FolderArchive,
  Loader2,
} from "lucide-react";
import type { TFunction } from "i18next";

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
import {
  RadioGroup,
  RadioItem,
} from "@/components/ui/radio-group";
import type {
  LocalArchiveImportResolution,
  LocalArchivePreview,
  LocalArchivePreviewSkill,
} from "@/types";
import {
  importLocalSkillArchive,
  previewLocalSkillArchive,
} from "@/stores/localArchiveImportSlice";

export interface LocalArchiveImportWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  t: TFunction;
  /** Called after a successful import so the parent can refresh Central. */
  onAfterImportSuccess: () => Promise<void>;
}

type WizardStep = "choose" | "preview" | "importing" | "result";

/**
 * Local ZIP skill archive import wizard.
 *
 * State machine: `choose -> preview -> importing -> result`.
 *
 * The wizard only writes to Central after the user confirms and the backend
 * has re-verified the archive fingerprint (SHA-256 + byte length) matches
 * the one returned by preview. Any mismatch fails with
 * `archive_changed_since_preview` before staging or Central mutation.
 */
export function LocalArchiveImportWizard({
  open,
  onOpenChange,
  t,
  onAfterImportSuccess,
}: LocalArchiveImportWizardProps) {
  const [step, setStep] = useState<WizardStep>("choose");
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [preview, setPreview] = useState<LocalArchivePreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [isPreviewLoading, setIsPreviewLoading] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [resolution, setResolution] =
    useState<LocalArchiveImportResolution>("overwrite");
  const [renamedSkillId, setRenamedSkillId] = useState("");

  // Reset state when the dialog closes.
  useEffect(() => {
    if (!open) {
      setStep("choose");
      setArchivePath(null);
      setPreview(null);
      setPreviewError(null);
      setIsPreviewLoading(false);
      setIsImporting(false);
      setImportError(null);
      setResolution("overwrite");
      setRenamedSkillId("");
    }
  }, [open]);

  const handleChooseArchive = useCallback(async () => {
    setPreviewError(null);
    setIsPreviewLoading(true);
    try {
      const { open: openFilePicker } = await import(
        "@tauri-apps/plugin-dialog"
      );
      const selected = await openFilePicker({
        multiple: false,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (!selected || typeof selected !== "string") {
        // User cancelled the file picker.
        setIsPreviewLoading(false);
        return;
      }
      setArchivePath(selected);
      const result = await previewLocalSkillArchive(selected);
      setPreview(result);
      setStep("preview");
      // Default resolution based on conflict.
      const skill = result.skills[0];
      if (skill?.conflict) {
        setResolution("overwrite");
      } else {
        setResolution("overwrite");
      }
    } catch (err) {
      setPreviewError(String(err));
    } finally {
      setIsPreviewLoading(false);
    }
  }, []);

  const handleImport = useCallback(async () => {
    if (!archivePath || !preview) return;
    setIsImporting(true);
    setImportError(null);
    setStep("importing");
    try {
      const finalResolution =
        resolution === "rename" && renamedSkillId.trim()
          ? ("rename" as LocalArchiveImportResolution)
          : resolution;
      await importLocalSkillArchive(
        archivePath,
        preview.fingerprint,
        finalResolution,
        finalResolution === "rename" ? renamedSkillId.trim() : undefined,
      );
      setStep("result");
      await onAfterImportSuccess();
    } catch (err) {
      setImportError(String(err));
      setStep("preview");
    } finally {
      setIsImporting(false);
    }
  }, [archivePath, preview, resolution, renamedSkillId, onAfterImportSuccess]);

  const skill: LocalArchivePreviewSkill | null = preview?.skills[0] ?? null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
            {previewError && (
              <div className="flex items-start gap-2 rounded-lg bg-destructive/10 p-3 text-sm text-destructive">
                <AlertCircle className="size-4 shrink-0" />
                <span>{previewError}</span>
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
            {importError && (
              <div className="flex items-start gap-2 rounded-lg bg-destructive/10 p-3 text-sm text-destructive">
                <AlertCircle className="size-4 shrink-0" />
                <span>{importError}</span>
              </div>
            )}
            <div className="rounded-lg border border-border/60 p-3">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium">{skill.skillName}</p>
                  <p className="text-xs text-muted-foreground">
                    id: {skill.skillId}
                  </p>
                </div>
                <span className="text-xs text-muted-foreground">
                  {preview.archiveDisplayName}
                </span>
              </div>
              {skill.description && (
                <p className="mt-1 text-xs text-muted-foreground">
                  {skill.description}
                </p>
              )}
              <div className="mt-2 flex gap-4 text-xs text-muted-foreground">
                <span>
                  {t("central.localArchiveWizard.fileCount")}:{" "}
                  {skill.fileCount}
                </span>
                <span>
                  {t("central.localArchiveWizard.totalBytes")}:{" "}
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

            <RadioGroup
              value={resolution}
              onValueChange={(v) =>
                setResolution(v as LocalArchiveImportResolution)
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
                  onChange={(e) => setRenamedSkillId(e.target.value)}
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
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t("common.cancel")}
            </Button>
          )}
          {step === "preview" && (
            <>
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t("common.cancel")}
              </Button>
              <Button
                onClick={handleImport}
                disabled={isImporting}
                data-testid="local-archive-import-confirm"
              >
                {t("central.localArchiveWizard.import")}
              </Button>
            </>
          )}
          {step === "result" && (
            <Button onClick={() => onOpenChange(false)}>
              {t("common.close")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}