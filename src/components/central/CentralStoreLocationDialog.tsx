import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, FolderOpen } from "lucide-react";
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
import type {
  CentralStoreLocationChangeResult,
  CentralStoreLocationPreview,
} from "@/types";

interface CentralStoreLocationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  t: TFunction;
  currentPath: string;
  preview: (targetPath: string) => Promise<CentralStoreLocationPreview>;
  apply: (targetPath: string) => Promise<CentralStoreLocationChangeResult>;
  onApplied: (result: CentralStoreLocationChangeResult) => void;
}

export function CentralStoreLocationDialog({
  open: isOpen,
  onOpenChange,
  t,
  currentPath,
  preview,
  apply,
  onApplied,
}: CentralStoreLocationDialogProps) {
  const [targetPath, setTargetPath] = useState("");
  const [previewResult, setPreviewResult] =
    useState<CentralStoreLocationPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isPicking, setIsPicking] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      setTargetPath("");
      setPreviewResult(null);
      setError(null);
      setIsPreviewing(false);
      setIsApplying(false);
      setIsPicking(false);
    }
  }, [isOpen]);

  const handlePreview = async () => {
    setError(null);
    setPreviewResult(null);
    setIsPreviewing(true);
    try {
      setPreviewResult(await preview(targetPath));
    } catch (err) {
      setError(formatCentralStoreLocationError(t, err));
    } finally {
      setIsPreviewing(false);
    }
  };

  const handleApply = async () => {
    setError(null);
    setIsApplying(true);
    try {
      const result = await apply(targetPath);
      onApplied(result);
      onOpenChange(false);
    } catch (err) {
      setError(formatCentralStoreLocationError(t, err));
    } finally {
      setIsApplying(false);
    }
  };

  const handlePickFolder = async () => {
    setIsPicking(true);
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        defaultPath: targetPath.trim() || currentPath || undefined,
        canCreateDirectories: true,
      });
      if (typeof selectedPath === "string") {
        setTargetPath(selectedPath);
        setPreviewResult(null);
        setError(null);
      }
    } catch (err) {
      setError(t("central.storeLocation.pickError", { error: String(err) }));
    } finally {
      setIsPicking(false);
    }
  };

  const canPreview =
    targetPath.trim().length > 0 && !isPreviewing && !isApplying;
  const canApply = Boolean(previewResult) && !isPreviewing && !isApplying;

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("central.storeLocation.title")}</DialogTitle>
          <DialogDescription>
            {t("central.storeLocation.description")}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <div className="text-xs font-medium text-muted-foreground">
              {t("central.storeLocation.currentPath")}
            </div>
            <div
              className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground break-all"
              title={currentPath}
            >
              {currentPath}
            </div>
          </div>

          <div className="space-y-1.5">
            <label
              className="text-xs font-medium"
              htmlFor="central-store-location-target"
            >
              {t("central.storeLocation.newPath")}
            </label>
            <div className="flex gap-2">
              <Input
                id="central-store-location-target"
                value={targetPath}
                onChange={(event) => {
                  setTargetPath(event.target.value);
                  setPreviewResult(null);
                  setError(null);
                }}
                placeholder={t("central.storeLocation.newPathPlaceholder")}
              />
              <Button
                type="button"
                variant="outline"
                disabled={isPicking || isApplying}
                onClick={handlePickFolder}
                aria-label={t("central.storeLocation.browseAria")}
              >
                <FolderOpen className="size-3.5" aria-hidden="true" />
                <span className="hidden sm:inline">
                  {t("central.storeLocation.browse")}
                </span>
              </Button>
            </div>
          </div>

          {error ? (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive-text">
              {error}
            </div>
          ) : null}

          {previewResult ? (
            <div className="space-y-2 rounded-md border border-border bg-muted/30 p-3 text-xs">
              <div className="font-medium">
                {t("central.storeLocation.previewTitle")}
              </div>
              <dl className="grid grid-cols-2 gap-2 text-muted-foreground">
                <dt>{t("central.storeLocation.skillsToCopy")}</dt>
                <dd className="text-right text-foreground">
                  {previewResult.skillsToCopy}
                </dd>
                <dt>{t("central.storeLocation.skillsToOverwrite")}</dt>
                <dd className="text-right text-foreground">
                  {previewResult.skillsToOverwrite}
                </dd>
                <dt>{t("central.storeLocation.targetOnlySkills")}</dt>
                <dd className="text-right text-foreground">
                  {previewResult.targetOnlySkills}
                </dd>
              </dl>
              <div className="flex gap-2 rounded-md border border-warning/30 bg-warning/10 p-2 text-warning-foreground">
                <AlertTriangle
                  className="mt-0.5 size-3.5 shrink-0"
                  aria-hidden="true"
                />
                <span>{t("central.storeLocation.overwriteWarning")}</span>
              </div>
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={!canPreview}
            onClick={handlePreview}
          >
            {isPreviewing
              ? t("central.storeLocation.previewing")
              : t("central.storeLocation.preview")}
          </Button>
          <Button type="button" disabled={!canApply} onClick={handleApply}>
            {isApplying
              ? t("central.storeLocation.applying")
              : t("central.storeLocation.apply")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function formatCentralStoreLocationError(t: TFunction, err: unknown): string {
  const raw = String(err);
  const key = CENTRAL_STORE_LOCATION_ERROR_KEYS[raw];
  return key ? t(key) : t("central.storeLocation.error", { error: raw });
}

const CENTRAL_STORE_LOCATION_ERROR_KEYS: Record<string, string> = {
  central_store_location_unsupported_target:
    "central.storeLocation.unsupportedTarget",
  central_store_location_empty_path: "central.storeLocation.emptyPath",
  central_store_location_same_path: "central.storeLocation.samePath",
  central_store_location_nested_path: "central.storeLocation.nestedPath",
  central_store_location_requires_overwrite:
    "central.storeLocation.requiresOverwrite",
};
