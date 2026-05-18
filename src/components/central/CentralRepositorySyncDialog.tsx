import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Check, Download, Loader2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

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
import { Checkbox } from "@/components/ui/checkbox";
import type {
  AgentWithStatus,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  DuplicateResolution,
} from "@/types";
import type {
  CentralRepositoryAddedSkillSelection,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";

type MissingDecision = "keep" | "delete";
type AddedDecision = {
  selected: boolean;
  resolution: DuplicateResolution;
  renamedSkillId: string;
};

interface CentralRepositorySyncDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preview: CentralRepositorySyncPreview | null;
  deletePreview: BatchDeleteCentralSkillPreviewResult | null;
  agents: AgentWithStatus[];
  isPreviewLoading: boolean;
  isApplying: boolean;
  error: string | null;
  onConfirm: (
    keepSkillIds: string[],
    deleteRequests: BatchDeleteCentralSkillRequest[],
    additions: CentralRepositoryAddedSkillSelection[]
  ) => Promise<void>;
}

function uniqueIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

function copyKey(skillId: string, agentId: string): string {
  return `${skillId}\0${agentId}`;
}

function addedKey(repositoryId: string, sourcePath: string): string {
  return `${repositoryId}\0${sourcePath}`;
}

export function CentralRepositorySyncDialog({
  open,
  onOpenChange,
  preview,
  deletePreview,
  agents,
  isPreviewLoading,
  isApplying,
  error,
  onConfirm,
}: CentralRepositorySyncDialogProps) {
  const { t } = useTranslation();
  const [missingDecisions, setMissingDecisions] = useState<Record<string, MissingDecision>>({});
  const [addedDecisions, setAddedDecisions] = useState<Record<string, AddedDecision>>({});
  const [selectedCopyKeys, setSelectedCopyKeys] = useState<Set<string>>(new Set());
  const previewKey = useMemo(
    () =>
      [
        ...(preview?.remoteAdded ?? []).map((item) =>
          addedKey(item.repositoryId, item.preview.sourcePath)
        ),
        ...(preview?.remoteMissing ?? []).map((item) => item.skill_id),
      ].join("\0"),
    [preview]
  );

  useEffect(() => {
    if (!open || !preview) return;
    setMissingDecisions(
      Object.fromEntries(preview.remoteMissing.map((state) => [state.skill_id, "keep"]))
    );
    setAddedDecisions(
      Object.fromEntries(
        preview.remoteAdded.map((item) => [
          addedKey(item.repositoryId, item.preview.sourcePath),
          {
            selected: true,
            resolution: item.preview.conflict ? "skip" : "overwrite",
            renamedSkillId: item.preview.skillId,
          },
        ])
      )
    );
    setSelectedCopyKeys(new Set());
  }, [open, previewKey, preview]);

  const deletePreviewBySkillId = useMemo(
    () => new Map((deletePreview?.previews ?? []).map((item) => [item.skill_id, item])),
    [deletePreview]
  );
  const failedDeletePreviewBySkillId = useMemo(
    () => new Map((deletePreview?.failed ?? []).map((item) => [item.skill_id, item.error])),
    [deletePreview]
  );
  const agentNameById = useMemo(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name])),
    [agents]
  );

  function updateAddedDecision(key: string, patch: Partial<AddedDecision>) {
    setAddedDecisions((current) => ({
      ...current,
      [key]: {
        ...(current[key] ?? { selected: true, resolution: "overwrite", renamedSkillId: "" }),
        ...patch,
      },
    }));
  }

  function toggleCopy(skillId: string, agentId: string, checked: boolean) {
    setSelectedCopyKeys((current) => {
      const next = new Set(current);
      const key = copyKey(skillId, agentId);
      if (checked) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  }

  async function handleConfirm() {
    if (!preview) return;
    const keepSkillIds = preview.remoteMissing
      .filter((state) => missingDecisions[state.skill_id] !== "delete")
      .map((state) => state.skill_id);
    const deleteRequests = preview.remoteMissing
      .filter((state) => missingDecisions[state.skill_id] === "delete")
      .flatMap((state) => {
        const item = deletePreviewBySkillId.get(state.skill_id);
        if (!item) return [];
        return [
          {
            skill_id: item.skill_id,
            remove_agent_ids: uniqueIds(
              item.copy_installations
                .map((installation) => installation.agent_id)
                .filter((agentId) => selectedCopyKeys.has(copyKey(item.skill_id, agentId)))
            ),
          },
        ];
      });
    const additionsByRepo = new Map<string, CentralRepositoryAddedSkillSelection>();
    for (const item of preview.remoteAdded) {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      const decision = addedDecisions[key];
      if (!decision?.selected) continue;
      const entry =
        additionsByRepo.get(item.repositoryId) ??
        {
          repositoryId: item.repositoryId,
          previewWorkspaceId: null,
          selections: [],
        };
      entry.selections.push({
        sourcePath: item.preview.sourcePath,
        resolution: decision.resolution,
        renamedSkillId:
          decision.resolution === "rename" ? decision.renamedSkillId.trim() : null,
      });
      additionsByRepo.set(item.repositoryId, entry);
    }
    await onConfirm(keepSkillIds, deleteRequests, Array.from(additionsByRepo.values()));
  }

  const canApply = Boolean(preview) && !isPreviewLoading && !isApplying;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{t("central.repositorySyncTitle")}</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("central.repositorySyncDesc", {
              added: preview?.remoteAdded.length ?? 0,
              missing: preview?.remoteMissing.length ?? 0,
            })}
          </DialogDescription>

          {(preview?.failedRepositories.length ?? 0) > 0 && (
            <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
              <AlertTriangle className="mr-1 inline size-3.5" />
              {preview?.failedRepositories.map((item) => item.error).join("; ")}
            </div>
          )}

          {(preview?.remoteAdded.length ?? 0) > 0 && (
            <section className="space-y-2">
              <h3 className="text-sm font-semibold text-foreground">
                <Download className="mr-1 inline size-4" />
                {t("central.repositorySyncAddedTitle", {
                  count: preview?.remoteAdded.length ?? 0,
                })}
              </h3>
              <div className="max-h-56 space-y-2 overflow-auto pr-1">
                {preview?.remoteAdded.map((item) => {
                  const key = addedKey(item.repositoryId, item.preview.sourcePath);
                  const decision = addedDecisions[key] ?? {
                    selected: true,
                    resolution: item.preview.conflict ? "skip" : "overwrite",
                    renamedSkillId: item.preview.skillId,
                  };
                  return (
                    <article key={key} className="rounded-xl border border-border p-3">
                      <div className="flex items-start gap-3">
                        <Checkbox
                          checked={decision.selected}
                          onCheckedChange={(checked) =>
                            updateAddedDecision(key, { selected: !!checked })
                          }
                          aria-label={t("central.repositorySyncSelectAdded", {
                            skill: item.preview.skillName,
                          })}
                        />
                        <div className="min-w-0 flex-1">
                          <div className="font-medium text-foreground">
                            {item.preview.skillName}
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {item.repo.owner}/{item.repo.repo} · {item.preview.sourcePath}
                          </div>
                          {item.preview.conflict && (
                            <div className="mt-1 text-xs text-amber-700 dark:text-amber-300">
                              {t("central.repositorySyncConflict", {
                                skill: item.preview.conflict.existingName,
                              })}
                            </div>
                          )}
                        </div>
                        <div className="flex flex-wrap gap-1 text-xs">
                          {(["overwrite", "rename", "skip"] as DuplicateResolution[]).map(
                            (resolution) => (
                              <button
                                key={resolution}
                                type="button"
                                className={`rounded-lg border px-2 py-1 ${
                                  decision.resolution === resolution
                                    ? "border-primary bg-primary text-primary-foreground"
                                    : "border-border text-muted-foreground"
                                }`}
                                onClick={() => updateAddedDecision(key, { resolution })}
                              >
                                {t(`central.repositorySyncResolution.${resolution}`)}
                              </button>
                            )
                          )}
                        </div>
                      </div>
                      {decision.resolution === "rename" && (
                        <input
                          className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
                          value={decision.renamedSkillId}
                          onChange={(event) =>
                            updateAddedDecision(key, { renamedSkillId: event.target.value })
                          }
                          aria-label={t("central.repositorySyncRenameLabel")}
                        />
                      )}
                    </article>
                  );
                })}
              </div>
            </section>
          )}

          {(preview?.remoteMissing.length ?? 0) > 0 && (
            <section className="space-y-2">
              <h3 className="text-sm font-semibold text-foreground">
                <Trash2 className="mr-1 inline size-4" />
                {t("central.repositorySyncMissingTitle", {
                  count: preview?.remoteMissing.length ?? 0,
                })}
              </h3>
              {isPreviewLoading ? (
                <div className="flex items-center gap-2 rounded-xl border border-border p-3 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  {t("central.batchDeletePreviewLoading")}
                </div>
              ) : (
                <div className="max-h-64 space-y-2 overflow-auto pr-1">
                  {preview?.remoteMissing.map((state) => {
                    const item = deletePreviewBySkillId.get(state.skill_id);
                    const previewError = failedDeletePreviewBySkillId.get(state.skill_id);
                    const decision = missingDecisions[state.skill_id] ?? "keep";
                    return (
                      <article key={state.skill_id} className="rounded-xl border border-border p-3">
                        <div className="flex flex-wrap items-start justify-between gap-3">
                          <div>
                            <div className="font-medium text-foreground">{state.skill_id}</div>
                            <div className="text-xs text-muted-foreground">
                              {state.source_path
                                ? t("central.remoteMissingSource", { path: state.source_path })
                                : t("central.remoteMissingSourceUnknown")}
                            </div>
                          </div>
                          <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs">
                            {(["keep", "delete"] as MissingDecision[]).map((next) => (
                              <button
                                key={next}
                                type="button"
                                className={`rounded-lg px-3 py-1.5 font-medium ${
                                  decision === next
                                    ? next === "delete"
                                      ? "bg-destructive text-destructive-foreground"
                                      : "bg-primary text-primary-foreground"
                                    : "text-muted-foreground hover:bg-background"
                                }`}
                                disabled={next === "delete" && !item}
                                onClick={() =>
                                  setMissingDecisions((current) => ({
                                    ...current,
                                    [state.skill_id]: next,
                                  }))
                                }
                              >
                                {t(
                                  next === "delete"
                                    ? "central.remoteMissingDelete"
                                    : "central.remoteMissingKeep"
                                )}
                              </button>
                            ))}
                          </div>
                        </div>
                        {previewError && (
                          <div className="mt-2 text-xs text-amber-700 dark:text-amber-300">
                            {t("central.remoteMissingPreviewFailed", { error: previewError })}
                          </div>
                        )}
                        {decision === "delete" && item && item.copy_installations.length > 0 && (
                          <div className="mt-3 space-y-2 border-t border-border/70 pt-3">
                            <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                              {t("central.remoteMissingCopyInstalls")}
                            </div>
                            {item.copy_installations.map((installation) => {
                              const checked = selectedCopyKeys.has(
                                copyKey(item.skill_id, installation.agent_id)
                              );
                              return (
                                <label
                                  key={`${item.skill_id}:${installation.agent_id}`}
                                  className="flex cursor-pointer items-start gap-2 rounded-lg border border-border/70 p-2 text-sm"
                                >
                                  <Checkbox
                                    checked={checked}
                                    onCheckedChange={(value) =>
                                      toggleCopy(item.skill_id, installation.agent_id, !!value)
                                    }
                                    aria-label={t("central.remoteMissingCopyLabel", {
                                      platform:
                                        agentNameById.get(installation.agent_id) ??
                                        installation.agent_id,
                                      skill: item.skill_name,
                                    })}
                                  />
                                  <span className="min-w-0">
                                    <span className="block font-medium text-foreground">
                                      {agentNameById.get(installation.agent_id) ??
                                        installation.agent_id}
                                    </span>
                                    <span className="block truncate text-xs text-muted-foreground">
                                      {installation.installed_path}
                                    </span>
                                  </span>
                                </label>
                              );
                            })}
                          </div>
                        )}
                      </article>
                    );
                  })}
                </div>
              )}
            </section>
          )}

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </DialogBody>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApplying}>
            {t("common.cancel")}
          </Button>
          <Button onClick={handleConfirm} disabled={!canApply} data-testid="confirm-repo-sync">
            {isApplying ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.repositorySyncApplying")}
              </>
            ) : (
              <>
                <Check className="size-3.5" />
                {t("central.repositorySyncApply")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
