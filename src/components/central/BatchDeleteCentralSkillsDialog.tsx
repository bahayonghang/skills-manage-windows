import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Loader2, Trash2 } from "lucide-react";
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
import { formatBackendError } from "@/lib/backendError";
import { groupPlatformAgentIds } from "@/lib/platformCleanupGroups";
import type {
  AgentWithStatus,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
} from "@/types";

interface BatchDeleteCentralSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillIds: string[];
  preview: BatchDeleteCentralSkillPreviewResult | null;
  agents: AgentWithStatus[];
  isPreviewLoading: boolean;
  isDeleting: boolean;
  error: string | null;
  title?: string;
  description?: string;
  dangerTitle?: string;
  confirmLabel?: string;
  confirmTestId?: string;
  onConfirm: (
    requests: BatchDeleteCentralSkillRequest[],
  ) => Promise<BatchDeleteCentralSkillResult>;
}

function uniqueIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

function copyPathsForAgents(
  preview: BatchDeleteCentralSkillPreviewResult | null,
  agentIds: string[],
): string {
  const agentSet = new Set(agentIds);
  return uniqueIds(
    (preview?.previews ?? []).flatMap((item) =>
      item.copy_installations
        .filter((installation) => agentSet.has(installation.agent_id))
        .map((installation) => installation.installed_path),
    ),
  ).join(", ");
}

export function BatchDeleteCentralSkillsDialog({
  open,
  onOpenChange,
  skillIds,
  preview,
  agents,
  isPreviewLoading,
  isDeleting,
  error,
  title,
  description,
  dangerTitle,
  confirmLabel,
  confirmTestId,
  onConfirm,
}: BatchDeleteCentralSkillsDialogProps) {
  const { t } = useTranslation();
  const [selectedCopyAgentIds, setSelectedCopyAgentIds] = useState<Set<string>>(
    new Set(),
  );
  const [forceArmed, setForceArmed] = useState(false);
  const skillIdsKey = useMemo(() => skillIds.join("\0"), [skillIds]);

  useEffect(() => {
    if (open) {
      setSelectedCopyAgentIds(new Set());
      setForceArmed(false);
    }
  }, [open, skillIdsKey]);

  const copyAgentIds = useMemo(() => {
    return uniqueIds(
      (preview?.previews ?? []).flatMap((item) =>
        item.copy_installations.map((installation) => installation.agent_id),
      ),
    );
  }, [preview]);

  const autoRemovedAgentIds = useMemo(() => {
    return uniqueIds(
      (preview?.previews ?? []).flatMap((item) => item.auto_removed_agent_ids),
    );
  }, [preview]);
  const copyAgentGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        copyAgentIds,
        t("platformTargets.universalLabel"),
      ),
    [agents, copyAgentIds, t],
  );
  const autoRemovedGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        autoRemovedAgentIds,
        t("platformTargets.universalLabel"),
      ),
    [agents, autoRemovedAgentIds, t],
  );

  function copyCountForAgents(agentIds: string[]): number {
    const agentSet = new Set(agentIds);
    return (preview?.previews ?? []).filter((item) =>
      item.copy_installations.some((installation) =>
        agentSet.has(installation.agent_id),
      ),
    ).length;
  }

  function handleToggleCopy(agentIds: string[], checked: boolean) {
    setSelectedCopyAgentIds((current) => {
      const next = new Set(current);
      if (checked) {
        agentIds.forEach((agentId) => next.add(agentId));
      } else {
        agentIds.forEach((agentId) => next.delete(agentId));
      }
      return next;
    });
  }

  const pendingRecoveries = preview?.previews.filter((item) => item.pending_recovery) ?? [];
  const forceEligibleIds = new Set(
    pendingRecoveries
      .filter((item) => item.pending_recovery?.force_delete_eligible)
      .map((item) => item.skill_id),
  );
  const forceBlocked = pendingRecoveries.filter(
    (item) => !item.pending_recovery?.force_delete_eligible,
  );

  function buildRequests(forceEligible: boolean): BatchDeleteCentralSkillRequest[] {
    if (!preview) return [];
    return preview.previews.map((item) => {
      const removableAgentIds = item.copy_installations
        .map((installation) => installation.agent_id)
        .filter((agentId) => selectedCopyAgentIds.has(agentId));
      return {
        skill_id: item.skill_id,
        remove_agent_ids: uniqueIds(removableAgentIds),
        force: forceEligible && forceEligibleIds.has(item.skill_id),
      };
    });
  }

  async function handleConfirm() {
    await onConfirm(buildRequests(false));
  }

  async function handleForceClick() {
    if (!forceArmed) {
      setForceArmed(true);
      return;
    }
    await onConfirm(buildRequests(true));
  }

  const canConfirm = Boolean(preview && preview.previews.length > 0);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {title ?? t("central.batchDeleteTitle", { count: skillIds.length })}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {description ??
              t("central.batchDeleteDesc", { count: skillIds.length })}
          </DialogDescription>

          <div className="rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-sm">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive-text" />
              <div className="min-w-0">
                <div className="font-medium text-foreground">
                  {dangerTitle ??
                    t("central.batchDeleteCentralRequired", {
                      count: preview?.previews.length ?? skillIds.length,
                    })}
                </div>
                {preview && preview.previews.length > 0 && (
                  <div className="mt-2 max-h-24 space-y-1 overflow-auto text-xs text-muted-foreground">
                    {preview.previews.map((item) => (
                      <div key={item.skill_id} className="truncate">
                        {item.skill_name} - {item.central_path}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>

          {isPreviewLoading ? (
            <div className="flex items-center gap-2 rounded-xl border border-border p-3 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              {t("central.batchDeletePreviewLoading")}
            </div>
          ) : (
            <>
              {autoRemovedGroups.length > 0 && (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.batchDeleteLinkedPlatformInstalls")}
                  </div>
                  <div className="space-y-2">
                    {autoRemovedGroups.map((group) => (
                      <div
                        key={group.id}
                        className="rounded-xl border border-border bg-muted/30 p-3"
                      >
                        <div className="text-sm font-medium text-foreground">
                          {group.label}
                        </div>
                        {group.detail && (
                          <div className="mt-1 truncate text-xs text-muted-foreground">
                            {group.detail}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {copyAgentGroups.length > 0 ? (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.batchDeletePlatformCopies")}
                  </div>
                  <div className="space-y-2">
                    {copyAgentGroups.map((group) => {
                      const checked = group.agentIds.every((agentId) =>
                        selectedCopyAgentIds.has(agentId),
                      );
                      const paths = copyPathsForAgents(preview, group.agentIds);
                      return (
                        <label
                          key={group.id}
                          className="flex cursor-pointer items-start gap-2 rounded-xl border border-border p-3"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(value) =>
                              handleToggleCopy(group.agentIds, !!value)
                            }
                            aria-label={t(
                              "central.batchDeletePlatformCopyLabel",
                              {
                                platform: group.label,
                              },
                            )}
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium text-foreground">
                              {group.label}
                            </span>
                            <span className="mt-1 block text-xs text-muted-foreground">
                              {t("central.batchDeletePlatformCopyCount", {
                                count: copyCountForAgents(group.agentIds),
                              })}
                            </span>
                            {(paths || group.detail) && (
                              <span className="mt-1 block truncate text-xs text-muted-foreground">
                                {paths || group.detail}
                              </span>
                            )}
                          </span>
                        </label>
                      );
                    })}
                  </div>
                </div>
              ) : autoRemovedGroups.length === 0 ? (
                <div className="rounded-xl border border-border p-3 text-sm text-muted-foreground">
                  {t("central.batchDeleteNoPlatformCopies")}
                </div>
              ) : null}

              {pendingRecoveries.length > 0 && (
                <div className="rounded-xl border border-warning/30 bg-warning/10 p-3 text-sm text-warning-foreground">
                  <div className="font-medium">
                    {formatBackendError(
                      "central_operation.delete_restore_collision:",
                      t,
                    )}
                  </div>
                  {forceEligibleIds.size > 0 && forceArmed && (
                    <div className="mt-2 text-xs">{t("central.forceDeleteHint")}</div>
                  )}
                  {forceBlocked.length > 0 && (
                    <div className="mt-2 space-y-1 text-xs">
                      {forceBlocked.map((item) => (
                        <div key={item.skill_id}>
                          {item.skill_name}: {t("central.forceDeleteBlocked")}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {preview && preview.failed.length > 0 && (
                <div className="rounded-xl border border-warning/30 bg-warning/10 p-3 text-xs text-warning-foreground">
                  <div className="font-medium">
                    {t("central.batchDeletePreviewFailures")}
                  </div>
                  <div className="mt-2 space-y-1">
                    {preview.failed.map((item) => (
                      <div key={item.skill_id}>
                        {item.skill_id}:{" "}
                        {formatBackendError(
                          item.error_code
                            ? `${item.error_code}:${item.error}`
                            : item.error,
                          t,
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
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
            disabled={isDeleting}
          >
            {t("common.cancel")}
          </Button>
          {forceEligibleIds.size > 0 && (
            <Button
              variant="destructive"
              onClick={handleForceClick}
              disabled={isPreviewLoading || isDeleting || !canConfirm}
              data-testid="force-delete-batch-central-skills"
            >
              {isDeleting ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  {t("central.deletingSkill")}
                </>
              ) : (
                <>
                  <Trash2 className="size-3.5" />
                  {t("central.forceDeleteSkill")}
                </>
              )}
            </Button>
          )}
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={isPreviewLoading || isDeleting || !canConfirm}
            data-testid={confirmTestId ?? "confirm-batch-delete-central-skills"}
          >
            {isDeleting ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.deletingSkill")}
              </>
            ) : (
              <>
                <Trash2 className="size-3.5" />
                {confirmLabel ?? t("central.confirmBatchDelete")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
