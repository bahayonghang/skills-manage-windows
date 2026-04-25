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
  onConfirm: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
}

function agentName(agents: AgentWithStatus[], agentId: string): string {
  return agents.find((agent) => agent.id === agentId)?.display_name ?? agentId;
}

function uniqueIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
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
  onConfirm,
}: BatchDeleteCentralSkillsDialogProps) {
  const { t } = useTranslation();
  const [selectedCopyAgentIds, setSelectedCopyAgentIds] = useState<Set<string>>(new Set());
  const skillIdsKey = useMemo(() => skillIds.join("\0"), [skillIds]);

  useEffect(() => {
    if (open) {
      setSelectedCopyAgentIds(new Set());
    }
  }, [open, skillIdsKey]);

  const copyAgentIds = useMemo(() => {
    return uniqueIds(
      (preview?.previews ?? []).flatMap((item) =>
        item.copy_installations.map((installation) => installation.agent_id)
      )
    );
  }, [preview]);

  const autoRemovedAgentIds = useMemo(() => {
    return uniqueIds(
      (preview?.previews ?? []).flatMap((item) => item.auto_removed_agent_ids)
    );
  }, [preview]);

  function copyCountForAgent(agentId: string): number {
    return (preview?.previews ?? []).filter((item) =>
      item.copy_installations.some((installation) => installation.agent_id === agentId)
    ).length;
  }

  function handleToggleCopy(agentId: string, checked: boolean) {
    setSelectedCopyAgentIds((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(agentId);
      } else {
        next.delete(agentId);
      }
      return next;
    });
  }

  async function handleConfirm() {
    if (!preview) return;
    const requests = preview.previews.map((item) => {
      const removableAgentIds = item.copy_installations
        .map((installation) => installation.agent_id)
        .filter((agentId) => selectedCopyAgentIds.has(agentId));
      return {
        skill_id: item.skill_id,
        remove_agent_ids: uniqueIds(removableAgentIds),
      };
    });

    await onConfirm(requests);
  }

  const canConfirm = Boolean(preview && preview.previews.length > 0);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("central.batchDeleteTitle", { count: skillIds.length })}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("central.batchDeleteDesc", { count: skillIds.length })}
          </DialogDescription>

          <div className="rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-sm">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <div className="min-w-0">
                <div className="font-medium text-foreground">
                  {t("central.batchDeleteCentralRequired", {
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
              {copyAgentIds.length > 0 ? (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.batchDeletePlatformCopies")}
                  </div>
                  <div className="space-y-2">
                    {copyAgentIds.map((agentId) => {
                      const checked = selectedCopyAgentIds.has(agentId);
                      const name = agentName(agents, agentId);
                      return (
                        <label
                          key={agentId}
                          className="flex cursor-pointer items-start gap-2 rounded-xl border border-border p-3"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(value) => handleToggleCopy(agentId, !!value)}
                            aria-label={t("central.batchDeletePlatformCopyLabel", {
                              platform: name,
                            })}
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium text-foreground">{name}</span>
                            <span className="mt-1 block text-xs text-muted-foreground">
                              {t("central.batchDeletePlatformCopyCount", {
                                count: copyCountForAgent(agentId),
                              })}
                            </span>
                          </span>
                        </label>
                      );
                    })}
                  </div>
                </div>
              ) : (
                <div className="rounded-xl border border-border p-3 text-sm text-muted-foreground">
                  {t("central.batchDeleteNoPlatformCopies")}
                </div>
              )}

              {autoRemovedAgentIds.length > 0 && (
                <div className="rounded-xl border border-border bg-muted/30 p-3 text-xs text-muted-foreground">
                  {t("central.batchDeleteAutoCleanup", {
                    platforms: autoRemovedAgentIds.map((agentId) => agentName(agents, agentId)).join(", "),
                  })}
                </div>
              )}

              {preview && preview.failed.length > 0 && (
                <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
                  <div className="font-medium">{t("central.batchDeletePreviewFailures")}</div>
                  <div className="mt-2 space-y-1">
                    {preview.failed.map((item) => (
                      <div key={item.skill_id}>
                        {item.skill_id}: {item.error}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isDeleting}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={isPreviewLoading || isDeleting || !canConfirm}
            data-testid="confirm-batch-delete-central-skills"
          >
            {isDeleting ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.deletingSkill")}
              </>
            ) : (
              <>
                <Trash2 className="size-3.5" />
                {t("central.confirmBatchDelete")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
