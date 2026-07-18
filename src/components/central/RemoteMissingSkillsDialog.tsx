import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Check, Loader2, Trash2 } from "lucide-react";
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
  CentralSkillUpdateState,
} from "@/types";

type RemoteMissingDecision = "keep" | "delete";

interface RemoteMissingSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  states: CentralSkillUpdateState[];
  preview: BatchDeleteCentralSkillPreviewResult | null;
  agents: AgentWithStatus[];
  isPreviewLoading: boolean;
  isApplying: boolean;
  error: string | null;
  onConfirm: (
    keepSkillIds: string[],
    deleteRequests: BatchDeleteCentralSkillRequest[],
  ) => Promise<void>;
}

function uniqueIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

function copyKey(skillId: string, agentId: string): string {
  return `${skillId}\0${agentId}`;
}

/**
 * @deprecated 使用 `UpdateCenterDialog` 代替（plans/update-mechanism-overhaul-plan.md P5/P6）。
 * 本组件保留以兼容旧 workflow（更新检查后弹出的远端缺失处理流程），
 * 将在下个 minor release 删除。
 */
export function RemoteMissingSkillsDialog({
  open,
  onOpenChange,
  states,
  preview,
  agents,
  isPreviewLoading,
  isApplying,
  error,
  onConfirm,
}: RemoteMissingSkillsDialogProps) {
  const { t } = useTranslation();
  const [decisions, setDecisions] = useState<
    Record<string, RemoteMissingDecision>
  >({});
  const [selectedCopyKeys, setSelectedCopyKeys] = useState<Set<string>>(
    new Set(),
  );
  const statesKey = useMemo(
    () => states.map((state) => state.skill_id).join("\0"),
    [states],
  );

  useEffect(() => {
    if (open) {
      setDecisions(
        Object.fromEntries(states.map((state) => [state.skill_id, "keep"])),
      );
      setSelectedCopyKeys(new Set());
    }
  }, [open, statesKey, states]);

  const previewBySkillId = useMemo(
    () =>
      new Map((preview?.previews ?? []).map((item) => [item.skill_id, item])),
    [preview],
  );
  const failedPreviewBySkillId = useMemo(
    () =>
      new Map(
        (preview?.failed ?? []).map((item) => [item.skill_id, item.error]),
      ),
    [preview],
  );
  const agentNameById = useMemo(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name])),
    [agents],
  );

  function setDecision(skillId: string, decision: RemoteMissingDecision) {
    setDecisions((current) => ({
      ...current,
      [skillId]: decision,
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
    const keepSkillIds = states
      .filter((state) => decisions[state.skill_id] !== "delete")
      .map((state) => state.skill_id);
    const deleteRequests = states
      .filter((state) => decisions[state.skill_id] === "delete")
      .flatMap((state) => {
        const item = previewBySkillId.get(state.skill_id);
        if (!item) {
          return [];
        }
        return [
          {
            skill_id: item.skill_id,
            remove_agent_ids: uniqueIds(
              item.copy_installations
                .map((installation) => installation.agent_id)
                .filter((agentId) =>
                  selectedCopyKeys.has(copyKey(item.skill_id, agentId)),
                ),
            ),
          },
        ];
      });

    await onConfirm(keepSkillIds, deleteRequests);
  }

  const canApply = states.length > 0 && !isPreviewLoading && !isApplying;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("central.remoteMissingTitle")}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("central.remoteMissingDesc", { count: states.length })}
          </DialogDescription>

          {isPreviewLoading ? (
            <div className="flex items-center gap-2 rounded-xl border border-border p-3 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              {t("central.batchDeletePreviewLoading")}
            </div>
          ) : (
            <div className="max-h-[26rem] space-y-3 overflow-auto pr-1">
              {states.map((state) => {
                const item = previewBySkillId.get(state.skill_id);
                const previewError = failedPreviewBySkillId.get(state.skill_id);
                const decision = decisions[state.skill_id] ?? "keep";
                const canDelete = Boolean(item);
                return (
                  <section
                    key={state.skill_id}
                    className="rounded-xl border border-border bg-background p-3"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="font-medium text-foreground">
                          {state.skill_id}
                        </div>
                        <div className="mt-1 text-xs text-muted-foreground">
                          {state.source_path
                            ? t("central.remoteMissingSource", {
                                path: state.source_path,
                              })
                            : t("central.remoteMissingSourceUnknown")}
                        </div>
                        {state.error && (
                          <div className="mt-1 text-xs text-warning-foreground">
                            {state.error}
                          </div>
                        )}
                      </div>
                      <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs">
                        <button
                          type="button"
                          className={`rounded-lg px-3 py-1.5 font-medium transition-colors ${
                            decision === "keep"
                              ? "bg-primary text-primary-foreground shadow-sm"
                              : "text-muted-foreground hover:bg-background"
                          }`}
                          onClick={() => setDecision(state.skill_id, "keep")}
                        >
                          <Check className="mr-1 inline size-3" />
                          {t("central.remoteMissingKeep")}
                        </button>
                        <button
                          type="button"
                          className={`rounded-lg px-3 py-1.5 font-medium transition-colors ${
                            decision === "delete"
                              ? "bg-destructive text-destructive-foreground shadow-sm"
                              : "text-muted-foreground hover:bg-background"
                          }`}
                          disabled={!canDelete}
                          onClick={() => setDecision(state.skill_id, "delete")}
                        >
                          <Trash2 className="mr-1 inline size-3" />
                          {t("central.remoteMissingDelete")}
                        </button>
                      </div>
                    </div>

                    {previewError && (
                      <div className="mt-3 rounded-lg border border-warning/30 bg-warning/10 p-2 text-xs text-warning-foreground">
                        <AlertTriangle className="mr-1 inline size-3.5" />
                        {t("central.remoteMissingPreviewFailed", {
                          error: previewError,
                        })}
                      </div>
                    )}

                    {decision === "delete" &&
                      item &&
                      item.copy_installations.length > 0 && (
                        <div className="mt-3 space-y-2 border-t border-border/70 pt-3">
                          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                            {t("central.remoteMissingCopyInstalls")}
                          </div>
                          {item.copy_installations.map((installation) => {
                            const checked = selectedCopyKeys.has(
                              copyKey(item.skill_id, installation.agent_id),
                            );
                            return (
                              <label
                                key={`${item.skill_id}:${installation.agent_id}`}
                                className="flex cursor-pointer items-start gap-2 rounded-lg border border-border/70 p-2 text-sm"
                              >
                                <Checkbox
                                  checked={checked}
                                  onCheckedChange={(value) =>
                                    toggleCopy(
                                      item.skill_id,
                                      installation.agent_id,
                                      !!value,
                                    )
                                  }
                                  aria-label={t(
                                    "central.remoteMissingCopyLabel",
                                    {
                                      platform:
                                        agentNameById.get(
                                          installation.agent_id,
                                        ) ?? installation.agent_id,
                                      skill: item.skill_name,
                                    },
                                  )}
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
                  </section>
                );
              })}
            </div>
          )}

          {error && (
            <p className="text-xs text-destructive-text" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isApplying}
          >
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={!canApply}
            data-testid="confirm-remote-missing-skills"
          >
            {isApplying ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.remoteMissingApplying")}
              </>
            ) : (
              <>
                <Check className="size-3.5" />
                {t("central.remoteMissingApply")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
