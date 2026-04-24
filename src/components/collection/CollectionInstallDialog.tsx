import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { AgentWithStatus, CollectionBatchInstallResult } from "@/types";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

// ─── Props ────────────────────────────────────────────────────────────────────

interface CollectionInstallDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  collectionName: string;
  skillCount: number;
  agents: AgentWithStatus[];
  onInstall: (agentIds: string[]) => Promise<CollectionBatchInstallResult>;
}

// ─── CollectionInstallDialog ──────────────────────────────────────────────────

export function CollectionInstallDialog({
  open,
  onOpenChange,
  collectionName,
  skillCount,
  agents,
  onInstall,
}: CollectionInstallDialogProps) {
  const { t } = useTranslation();
  const targetAgents = agents.filter((a) => a.id !== "central");
  const isLockedTarget = (agent: AgentWithStatus) => isUniversalPlatformTarget(agent);
  const selectedInstallAgentIds = () =>
    Array.from(
      new Set(
        targetAgents
          .filter((agent) => selectedAgentIds.has(agent.id))
          .filter((agent) => !isLockedTarget(agent))
          .flatMap((agent) => getPlatformTargetInstallAgentIds(agent))
      )
    );

  const [selectedAgentIds, setSelectedAgentIds] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CollectionBatchInstallResult | null>(null);

  // Reset when dialog opens.
  useEffect(() => {
    if (open) {
      // Default: select all detected agents.
      const initial = new Set<string>(
        targetAgents
          .filter((agent) => isLockedTarget(agent) || agent.is_detected)
          .map((agent) => agent.id)
      );
      setSelectedAgentIds(initial);
      setError(null);
      setResult(null);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  function handleToggle(agentId: string, checked: boolean) {
    const target = targetAgents.find((agent) => agent.id === agentId);
    if (target && isLockedTarget(target)) {
      return;
    }

    setSelectedAgentIds((prev) => {
      const next = new Set(prev);
      if (checked) next.add(agentId);
      else next.delete(agentId);
      return next;
    });
  }

  async function handleInstall() {
    const agentIds = selectedInstallAgentIds();
    if (agentIds.length === 0) {
      setError(t("batchInstall.selectPlatform"));
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const installResult = await onInstall(agentIds);
      setResult(installResult);
      if (installResult.failed.length === 0) {
        // All succeeded — close dialog.
        onOpenChange(false);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("batchInstall.title", { name: collectionName })}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {t("batchInstall.desc", { count: skillCount })}
          </DialogDescription>

          {/* Platform checkboxes */}
          <div className="grid grid-cols-2 gap-x-4 gap-y-2" role="group" aria-label={t("batchInstall.selectPlatforms")}>
            {targetAgents.length === 0 ? (
              <p className="col-span-2 text-sm text-muted-foreground">
                {t("batchInstall.noPlatforms")}
              </p>
            ) : (
              targetAgents.map((agent) => {
                const isUniversal = isUniversalPlatformTarget(agent);
                const isLocked = isLockedTarget(agent);
                const isChecked = selectedAgentIds.has(agent.id);
                const displayName = isUniversal
                  ? t("platformTargets.universalLabel")
                  : agent.display_name;
                const memberNames = getPlatformTargetMemberNames(agent).join(", ");
                return (
                  <div key={agent.id} className="flex items-center gap-2">
                    <Checkbox
                      checked={isChecked}
                      disabled={isLocked}
                      onCheckedChange={(checked) =>
                        handleToggle(agent.id, !!checked)
                      }
                      aria-label={displayName}
                    />
                    <div
                      className={`min-w-0 flex-1 select-none ${
                        isLocked
                          ? "text-muted-foreground cursor-default"
                          : "text-foreground cursor-pointer"
                      }`}
                      title={isUniversal ? memberNames : agent.global_skills_dir}
                      onClick={() => !isLocked && handleToggle(agent.id, !isChecked)}
                    >
                      <div className="truncate text-sm">{displayName}</div>
                      {isUniversal && (
                        <div className="truncate text-[11px] text-muted-foreground">
                          {memberNames}
                        </div>
                      )}
                    </div>
                    {isLocked && (
                      <span className="text-xs text-primary shrink-0">
                        {t("platformTargets.alwaysIncluded")}
                      </span>
                    )}
                    {!agent.is_detected && (
                      <span className="text-xs text-muted-foreground shrink-0">
                        {t("batchInstall.notDetected")}
                      </span>
                    )}
                  </div>
                );
              })
            )}
          </div>

          {/* Result summary if partial failure */}
          {result && result.failed.length > 0 && (
            <div className="space-y-1">
              <p className="text-xs text-amber-600 dark:text-amber-400 font-medium">
                {t("batchInstall.succeeded", {
                  succeeded: result.succeeded.length,
                  failed: result.failed.length,
                })}
              </p>
              <ul className="text-xs text-muted-foreground space-y-0.5">
                {result.failed.map((f) => (
                  <li key={f.agent_id} className="text-destructive">
                    {f.agent_id}: {f.error}
                  </li>
                ))}
              </ul>
              <Button
                variant="outline"
                size="sm"
                onClick={() => onOpenChange(false)}
                className="mt-2"
              >
                {t("batchInstall.close")}
              </Button>
            </div>
          )}

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        {!result && (
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isLoading}
            >
              {t("batchInstall.cancel")}
            </Button>
            <Button
              onClick={handleInstall}
              disabled={isLoading || selectedInstallAgentIds().length === 0}
            >
              {isLoading ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  {t("batchInstall.installing")}
                </>
              ) : (
                t("batchInstall.install", { count: selectedInstallAgentIds().length })
              )}
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}
