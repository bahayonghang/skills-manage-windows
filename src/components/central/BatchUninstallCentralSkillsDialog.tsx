import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Loader2, Unlink } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
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
import {
  groupPlatformAgentIds,
  type PlatformCleanupGroup,
} from "@/lib/platformCleanupGroups";
import type {
  CentralBatchUninstallApplyResult,
  CentralBatchUninstallPreview,
  CentralBatchUninstallSkipReason,
} from "@/lib/centralBatchUninstall";
import type { AgentWithStatus } from "@/types";

interface BatchUninstallCentralSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preview: CentralBatchUninstallPreview;
  agents: AgentWithStatus[];
  isUninstalling: boolean;
  onConfirm: (
    preview: CentralBatchUninstallPreview,
  ) => Promise<CentralBatchUninstallApplyResult>;
}

function countRequestsForAgents(
  preview: CentralBatchUninstallPreview,
  agentIds: string[],
): number {
  const agentSet = new Set(agentIds);
  return preview.groups
    .filter((group) => agentSet.has(group.agentId))
    .reduce((count, group) => count + group.requests.length, 0);
}

function countSharedRootLinksForAgents(
  preview: CentralBatchUninstallPreview,
  agentIds: string[],
): number {
  const agentSet = new Set(agentIds);
  return preview.sharedRootLinks.filter((link) => agentSet.has(link.agentId))
    .length;
}

function summarizeSkippedReasons(
  preview: CentralBatchUninstallPreview,
): Array<{ reason: CentralBatchUninstallSkipReason; count: number }> {
  const counts = new Map<CentralBatchUninstallSkipReason, number>();
  for (const item of preview.skippedSkills) {
    counts.set(item.reason, (counts.get(item.reason) ?? 0) + 1);
  }
  return Array.from(counts, ([reason, count]) => ({ reason, count }));
}

function PlatformGroupList({
  groups,
  getCount,
}: {
  groups: PlatformCleanupGroup[];
  getCount: (group: PlatformCleanupGroup) => number;
}) {
  const { t } = useTranslation();

  return (
    <div className="space-y-2">
      {groups.map((group) => (
        <div
          key={group.id}
          className="rounded-xl border border-border bg-muted/30 p-3"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="text-sm font-medium text-foreground">
                {group.label}
              </div>
              {group.detail && (
                <div className="mt-1 truncate text-xs text-muted-foreground">
                  {group.detail}
                </div>
              )}
            </div>
            <span className="shrink-0 rounded-full bg-background px-2 py-0.5 text-xs text-muted-foreground ring-1 ring-border">
              {t("central.batchUninstallPlatformCount", {
                count: getCount(group),
              })}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}

export function BatchUninstallCentralSkillsDialog({
  open,
  onOpenChange,
  preview,
  agents,
  isUninstalling,
  onConfirm,
}: BatchUninstallCentralSkillsDialogProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] =
    useState<CentralBatchUninstallApplyResult | null>(null);
  useEffect(() => {
    if (!open) return;
    setError(null);
    setResult(null);
  }, [open]);

  const removableGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        preview.groups.map((group) => group.agentId),
        t("platformTargets.universalLabel"),
      ),
    [agents, preview.groups, t],
  );
  const sharedRootGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        preview.sharedRootLinks.map((link) => link.agentId),
        t("platformTargets.universalLabel"),
      ),
    [agents, preview.sharedRootLinks, t],
  );
  const skippedSummary = useMemo(
    () => summarizeSkippedReasons(preview),
    [preview],
  );
  const canConfirm = preview.totals.removableInstallCount > 0;

  async function handleConfirm() {
    if (!canConfirm) return;
    setError(null);
    try {
      const nextResult = await onConfirm(preview);
      setResult(nextResult);
      if (nextResult.failed.length === 0) {
        onOpenChange(false);
      }
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("central.batchUninstallTitle", {
              count: preview.totals.selectedSkillCount,
            })}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("central.batchUninstallDesc", {
              skillCount: preview.totals.selectedSkillCount,
              installCount: preview.totals.removableInstallCount,
              platformCount: preview.totals.removablePlatformCount,
            })}
          </DialogDescription>

          <div className="rounded-xl border border-warning/30 bg-warning/10 p-3 text-sm">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning-foreground" />
              <div className="space-y-1">
                <div className="font-medium text-foreground">
                  {t("central.batchUninstallSafetyTitle")}
                </div>
                <p className="text-muted-foreground">
                  {t("central.batchUninstallSafetyDesc")}
                </p>
              </div>
            </div>
          </div>

          {removableGroups.length > 0 ? (
            <div className="space-y-2">
              <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {t("central.batchUninstallPlatforms")}
              </div>
              <PlatformGroupList
                groups={removableGroups}
                getCount={(group) => countRequestsForAgents(preview, group.agentIds)}
              />
            </div>
          ) : (
            <div
              className="rounded-xl border border-border p-3 text-sm text-muted-foreground"
              data-testid="central-batch-uninstall-noop"
            >
              {t("central.batchUninstallNoRemovableInstalls")}
            </div>
          )}

          {preview.skippedSkills.length > 0 && (
            <div
              className="rounded-xl border border-border bg-muted/20 p-3 text-xs text-muted-foreground"
              data-testid="central-batch-uninstall-skipped-summary"
            >
              <div className="font-medium text-foreground">
                {t("central.batchUninstallSkippedTitle", {
                  count: preview.skippedSkills.length,
                })}
              </div>
              <ul className="mt-2 space-y-1">
                {skippedSummary.map(({ reason, count }) => (
                  <li key={reason}>
                    {t(`central.batchUninstallSkipReasons.${reason}`)}: {count}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {sharedRootGroups.length > 0 && (
            <div className="space-y-2">
              <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {t("central.batchUninstallSharedRoots")}
              </div>
              <PlatformGroupList
                groups={sharedRootGroups}
                getCount={(group) =>
                  countSharedRootLinksForAgents(preview, group.agentIds)
                }
              />
            </div>
          )}

          {result && result.failed.length > 0 && (
            <div
              className="rounded-xl border border-warning/30 bg-warning/10 p-3 text-xs text-warning-foreground"
              role="alert"
            >
              <div className="font-medium">
                {t("central.batchUninstallPartial", {
                  succeeded: result.succeeded.length,
                  failed: result.failed.length,
                })}
              </div>
              <ul className="mt-2 max-h-32 space-y-1 overflow-auto">
                {result.failed.map((failure) => (
                  <li key={`${failure.agent_id}:${failure.skill_id}`}>
                    {failure.skill_id} / {failure.agent_id}: {failure.error}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isUninstalling}
          >
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isUninstalling || !canConfirm}
            data-testid="confirm-batch-uninstall-central-skills"
          >
            {isUninstalling ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.batchUninstallUninstalling")}
              </>
            ) : (
              <>
                <Unlink className="size-3.5" />
                {t("central.confirmBatchUninstall", {
                  count: preview.totals.removableInstallCount,
                })}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
