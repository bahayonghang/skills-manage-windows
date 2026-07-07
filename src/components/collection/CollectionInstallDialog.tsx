import { useState, useEffect, useRef } from "react";
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
import {
  InstallFailureList,
  PlatformMultiSelectGrid,
  usePlatformTargetSelection,
} from "@/components/platform/PlatformMultiSelect";
import { AgentWithStatus, CollectionBatchInstallResult } from "@/types";
import { isUniversalPlatformTarget } from "@/lib/platformTargetGroups";

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
  const isLockedTarget = (agent: AgentWithStatus) =>
    isUniversalPlatformTarget(agent);

  const selection = usePlatformTargetSelection({
    targets: targetAgents,
    isTargetDisabled: isLockedTarget,
    // Default: select all enabled and visible platform targets (locked ones stay checked).
    isTargetDefaultSelected: () => true,
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CollectionBatchInstallResult | null>(
    null,
  );
  const wasOpenRef = useRef(false);
  const { reset: resetSelection } = selection;

  // Reset when dialog opens.
  useEffect(() => {
    const didOpen = open && !wasOpenRef.current;
    wasOpenRef.current = open;
    if (didOpen) {
      resetSelection();
      setError(null);
      setResult(null);
    }
  }, [open, resetSelection]);

  async function handleInstall() {
    const agentIds = selection.selectedInstallAgentIds();
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
          <DialogTitle>
            {t("batchInstall.title", { name: collectionName })}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {t("batchInstall.desc", { count: skillCount })}
          </DialogDescription>

          {/* Platform checkboxes */}
          <PlatformMultiSelectGrid
            targets={targetAgents}
            isSelected={selection.isSelected}
            isDisabled={isLockedTarget}
            onToggle={selection.toggle}
            renderBadges={(agent) => (
              <>
                {isLockedTarget(agent) && (
                  <span className="text-xs text-primary shrink-0">
                    {t("platformTargets.alwaysIncluded")}
                  </span>
                )}
                {!agent.is_detected && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {t("batchInstall.notDetected")}
                  </span>
                )}
              </>
            )}
            emptyMessage={t("batchInstall.noPlatforms")}
            ariaLabel={t("batchInstall.selectPlatforms")}
          />

          {/* Result summary if partial failure */}
          {result && result.failed.length > 0 && (
            <div className="space-y-1">
              <p className="text-xs text-warning-foreground font-medium">
                {t("batchInstall.succeeded", {
                  succeeded: result.succeeded.length,
                  failed: result.failed.length,
                })}
              </p>
              <InstallFailureList
                failures={result.failed.map((f) => ({
                  key: f.agent_id,
                  label: `${f.agent_id}: ${f.error}`,
                }))}
              />
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
              disabled={
                isLoading || selection.selectedInstallAgentIds().length === 0
              }
            >
              {isLoading ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  {t("batchInstall.installing")}
                </>
              ) : (
                t("batchInstall.install", {
                  count: selection.selectedInstallAgentIds().length,
                })
              )}
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}
