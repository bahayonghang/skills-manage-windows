import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Download, FileJson, Loader2, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import {
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import type {
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResolutionType,
  SkillportStateImportResult,
  SkillportStateSkillPreview,
} from "@/types";

interface CentralStatePortabilityDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  exportState: () => Promise<string>;
  previewImport: (json: string) => Promise<SkillportStateImportPreview>;
  importState: (
    json: string,
    resolutions: SkillportStateImportResolution[]
  ) => Promise<SkillportStateImportResult>;
  onAfterImport?: () => Promise<void> | void;
}

interface ExportSummary {
  githubSources: number;
  centralSkills: number;
  unrestorableSkills: number;
}

type TabId = "export" | "import";

const EMPTY_SUMMARY: ExportSummary = {
  githubSources: 0,
  centralSkills: 0,
  unrestorableSkills: 0,
};

function parseExportSummary(json: string): ExportSummary {
  const parsed = JSON.parse(json) as {
    githubSources?: unknown[];
    centralSkills?: unknown[];
    unrestorableSkills?: unknown[];
  };
  return {
    githubSources: parsed.githubSources?.length ?? 0,
    centralSkills: parsed.centralSkills?.length ?? 0,
    unrestorableSkills: parsed.unrestorableSkills?.length ?? 0,
  };
}

function defaultExportFileName() {
  const date = new Date().toISOString().slice(0, 10);
  return `skillport-state-${date}.json`;
}

function statusTone(status: SkillportStateSkillPreview["status"]) {
  if (status === "ready") return "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300";
  if (status === "conflict") return "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300";
  if (status === "missing") return "border-destructive/40 bg-destructive/10 text-destructive";
  return "border-muted-foreground/30 bg-muted text-muted-foreground";
}

function isManifestPreviewError(error: unknown) {
  const message = String(error);
  return (
    message.includes("Invalid SkillPort state JSON:") ||
    message.includes("Unsupported SkillPort state export kind") ||
    message.includes("Unsupported SkillPort state export version:")
  );
}

export function CentralStatePortabilityDialog({
  open,
  onOpenChange,
  exportState,
  previewImport,
  importState,
  onAfterImport,
}: CentralStatePortabilityDialogProps) {
  const { t } = useTranslation();
  const tRef = useRef(t);
  const [activeTab, setActiveTab] = useState<TabId>("export");
  const [exportJson, setExportJson] = useState("");
  const [exportSummary, setExportSummary] = useState<ExportSummary>(EMPTY_SUMMARY);
  const [isExportLoading, setIsExportLoading] = useState(false);
  const [importJson, setImportJson] = useState("");
  const [preview, setPreview] = useState<SkillportStateImportPreview | null>(null);
  const [isPreviewLoading, setIsPreviewLoading] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [conflictResolutions, setConflictResolutions] = useState<
    Record<string, SkillportStateImportResolutionType>
  >({});
  const [renameValues, setRenameValues] = useState<Record<string, string>>({});
  const [lastImportResult, setLastImportResult] = useState<SkillportStateImportResult | null>(null);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  const refreshExportPreview = useCallback(async () => {
    setIsExportLoading(true);
    try {
      const json = await exportState();
      setExportJson(json);
      setExportSummary(parseExportSummary(json));
    } catch (err) {
      toast.error(tRef.current("central.portabilityExportError", { error: String(err) }));
    } finally {
      setIsExportLoading(false);
    }
  }, [exportState]);

  useEffect(() => {
    if (!open) return;
    setActiveTab("export");
    setPreview(null);
    setLastImportResult(null);
    setConflictResolutions({});
    setRenameValues({});
    void refreshExportPreview();
  }, [open, refreshExportPreview]);

  async function handleSaveExport() {
    try {
      const json = exportJson || (await exportState());
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeTextFile } = await import("@tauri-apps/plugin-fs");
      const path = await save({
        defaultPath: defaultExportFileName(),
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await writeTextFile(path, json);
      toast.success(t("central.portabilityExportSuccess"));
    } catch (err) {
      toast.error(t("central.portabilityExportError", { error: String(err) }));
    }
  }

  async function handleChooseImportFile() {
    try {
      const { open: openFile } = await import("@tauri-apps/plugin-dialog");
      const { readTextFile } = await import("@tauri-apps/plugin-fs");
      const selected = await openFile({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof selected !== "string") return;
      const text = await readTextFile(selected);
      setImportJson(text);
      setLastImportResult(null);
      await handlePreview(text);
    } catch (err) {
      toast.error(t("central.portabilityImportError", { error: String(err) }));
    }
  }

  async function handlePreview(json = importJson) {
    const trimmed = json.trim();
    if (!trimmed) return;
    setIsPreviewLoading(true);
    try {
      const nextPreview = await previewImport(trimmed);
      setPreview(nextPreview);
      setLastImportResult(null);
      setConflictResolutions(
        Object.fromEntries(
          nextPreview.skills
            .filter((skill) => skill.status === "conflict")
            .map((skill) => [skill.id, "skip" as SkillportStateImportResolutionType])
        )
      );
      setRenameValues({});
    } catch (err) {
      if (isManifestPreviewError(err)) {
        setPreview(null);
        setLastImportResult(null);
      }
      toast.error(t("central.portabilityPreviewError", { error: String(err) }));
    } finally {
      setIsPreviewLoading(false);
    }
  }

  function buildResolutions(): SkillportStateImportResolution[] {
    if (!preview) return [];
    return preview.skills
      .filter((skill) => skill.status === "ready" || skill.status === "conflict")
      .map((skill) => {
        const resolution =
          skill.status === "ready"
            ? "overwrite"
            : conflictResolutions[skill.id] ?? "skip";
        return {
          skillId: skill.id,
          sourcePath: skill.sourcePath,
          resolution,
          renamedSkillId: resolution === "rename" ? renameValues[skill.id]?.trim() : null,
        };
      });
  }

  async function handleImport() {
    if (!preview) return;
    const resolutions = buildResolutions();
    const missingRename = resolutions.find(
      (resolution) => resolution.resolution === "rename" && !resolution.renamedSkillId
    );
    if (missingRename) {
      toast.error(t("central.portabilityRenameRequired"));
      return;
    }

    setIsImporting(true);
    try {
      const result = await importState(importJson.trim(), resolutions);
      setLastImportResult(result);
      toast.success(
        t("central.portabilityImportSuccess", {
          imported: result.importedSkills.length,
          failed: result.failedSkills.length,
          skipped: result.skippedSkills.length,
        })
      );
      if (result.failedSkills.length === 0) {
        onOpenChange(false);
      }
      await onAfterImport?.();
    } catch (err) {
      toast.error(t("central.portabilityImportError", { error: String(err) }));
    } finally {
      setIsImporting(false);
    }
  }

  const importableCount = useMemo(() => {
    if (!preview) return 0;
    return preview.skills.filter((skill) => skill.status === "ready" || skill.status === "conflict").length;
  }, [preview]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("central.portabilityTitle")}</DialogTitle>
          <DialogDescription>{t("central.portabilityDesc")}</DialogDescription>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <div className="inline-flex rounded-md border border-border bg-muted/40 p-1">
            <button
              type="button"
              data-testid="central-portability-export-tab"
              className={cn(
                "inline-flex items-center gap-2 rounded-sm px-3 py-1.5 text-sm",
                activeTab === "export" ? "bg-background shadow-sm" : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("export")}
            >
              <Download className="size-4" />
              {t("central.portabilityExportTab")}
            </button>
            <button
              type="button"
              data-testid="central-portability-import-tab"
              className={cn(
                "inline-flex items-center gap-2 rounded-sm px-3 py-1.5 text-sm",
                activeTab === "import" ? "bg-background shadow-sm" : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("import")}
            >
              <Upload className="size-4" />
              {t("central.portabilityImportTab")}
            </button>
          </div>

          {activeTab === "export" ? (
            <div className="space-y-4">
              <div className="grid gap-3 sm:grid-cols-3">
                <SummaryTile label={t("central.portabilityGithubSources")} value={exportSummary.githubSources} />
                <SummaryTile label={t("central.portabilityCentralSkills")} value={exportSummary.centralSkills} />
                <SummaryTile
                  label={t("central.portabilityUnrestorable")}
                  value={exportSummary.unrestorableSkills}
                />
              </div>
              <div className="rounded-md border border-amber-500/30 bg-amber-500/10 p-3 text-sm text-amber-800 dark:text-amber-200">
                <div className="flex gap-2">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                  <span>{t("central.portabilityExportBoundary")}</span>
                </div>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  variant="outline"
                  data-testid="central-portability-choose-file"
                  onClick={handleChooseImportFile}
                  disabled={isPreviewLoading || isImporting}
                >
                  <FileJson className="size-4" />
                  {t("central.portabilityChooseFile")}
                </Button>
                <Button
                  variant="outline"
                  data-testid="central-portability-preview"
                  onClick={() => void handlePreview()}
                  disabled={!importJson.trim() || isPreviewLoading || isImporting}
                >
                  {isPreviewLoading ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
                  {t("central.portabilityPreview")}
                </Button>
              </div>
              <Textarea
                data-testid="central-portability-json-input"
                value={importJson}
                onChange={(event) => {
                  setImportJson(event.target.value);
                  setLastImportResult(null);
                }}
                placeholder={t("central.portabilityPastePlaceholder")}
                className="min-h-28 font-mono text-xs"
              />
              {preview && (
                <div className="space-y-3">
                  <div className="grid gap-2 sm:grid-cols-4">
                    <SummaryTile label={t("central.portabilityReady")} value={preview.summary.ready} />
                    <SummaryTile label={t("central.portabilityConflicts")} value={preview.summary.conflicts} />
                    <SummaryTile label={t("central.portabilityMissing")} value={preview.summary.missing} />
                    <SummaryTile
                      label={t("central.portabilityUnrestorable")}
                      value={preview.summary.unrestorable}
                    />
                  </div>
                  <div className="max-h-72 overflow-auto rounded-md border border-border">
                    {preview.skills.map((skill) => (
                      <div key={skill.id} className="grid gap-3 border-b border-border p-3 last:border-b-0 md:grid-cols-[1fr_auto]">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="truncate text-sm font-medium">{skill.name}</span>
                            <span className={cn("rounded-full border px-2 py-0.5 text-xs", statusTone(skill.status))}>
                              {t(`central.portabilityStatus.${skill.status}`)}
                            </span>
                          </div>
                          <div className="mt-1 truncate text-xs text-muted-foreground">
                            {skill.sourcePath ?? skill.id}
                          </div>
                          {(skill.reason || skill.detail) && (
                            <div className="mt-1 text-xs text-muted-foreground">
                              <span>
                                {skill.reason
                                  ? t(`central.portabilityReason.${skill.reason}`, {
                                      defaultValue: skill.reason,
                                    })
                                  : t("central.portabilityReason.unknown")}
                              </span>
                              {skill.detail ? <span>{`: ${skill.detail}`}</span> : null}
                            </div>
                          )}
                        </div>
                        {skill.status === "conflict" && (
                          <div className="flex items-center gap-2">
                            <select
                              className="h-9 rounded-md border border-input bg-background px-2 text-sm"
                              value={conflictResolutions[skill.id] ?? "skip"}
                              onChange={(event) =>
                                setConflictResolutions((current) => ({
                                  ...current,
                                  [skill.id]: event.target.value as SkillportStateImportResolutionType,
                                }))
                              }
                              aria-label={t("central.portabilityConflictAction")}
                            >
                              <option value="skip">{t("central.portabilitySkip")}</option>
                              <option value="overwrite">{t("central.portabilityOverwrite")}</option>
                              <option value="rename">{t("central.portabilityRename")}</option>
                            </select>
                            {conflictResolutions[skill.id] === "rename" && (
                              <input
                                className="h-9 w-40 rounded-md border border-input bg-background px-2 text-sm"
                                value={renameValues[skill.id] ?? ""}
                                onChange={(event) =>
                                  setRenameValues((current) => ({
                                    ...current,
                                    [skill.id]: event.target.value,
                                  }))
                                }
                                placeholder={t("central.portabilityRenamePlaceholder")}
                              />
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                  {lastImportResult && lastImportResult.failedSkills.length > 0 && (
                    <div className="space-y-2 rounded-md border border-destructive/30 bg-destructive/5 p-3">
                      <div className="text-sm font-medium text-destructive">
                        {t("central.portabilityImportFailuresTitle", {
                          count: lastImportResult.failedSkills.length,
                        })}
                      </div>
                      <div className="space-y-2">
                        {lastImportResult.failedSkills.map((failure) => (
                          <div
                            key={`${failure.skillId}-${failure.sourcePath ?? "unknown"}`}
                            className="rounded-md border border-border bg-background/70 p-2"
                          >
                            <div className="text-sm font-medium">{failure.skillId}</div>
                            {failure.sourcePath ? (
                              <div className="mt-1 text-xs text-muted-foreground">
                                {failure.sourcePath}
                              </div>
                            ) : null}
                            <div className="mt-1 text-xs text-muted-foreground">{failure.error}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isImporting}>
            {t("installDialog.cancel")}
          </Button>
          {activeTab === "export" ? (
            <Button
              data-testid="central-portability-save-export"
              onClick={handleSaveExport}
              disabled={isExportLoading}
            >
              {isExportLoading ? <Loader2 className="size-4 animate-spin" /> : <Download className="size-4" />}
              {t("central.portabilitySaveExport")}
            </Button>
          ) : (
            <Button
              data-testid="central-portability-run-import"
              onClick={handleImport}
              disabled={!preview || importableCount === 0 || isImporting}
            >
              {isImporting ? <Loader2 className="size-4 animate-spin" /> : <Upload className="size-4" />}
              {t("central.portabilityRunImport", { count: importableCount })}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SummaryTile({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-border bg-muted/30 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-2xl font-semibold">{value}</div>
    </div>
  );
}
