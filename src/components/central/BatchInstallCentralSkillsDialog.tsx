import { useEffect, useMemo, useState } from "react";
import { Loader2, Settings2 } from "lucide-react";
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
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import type { InstallMethod } from "@/components/central/InstallDialog";
import { ProjectPathPicker } from "@/components/central/ProjectPathPicker";
import {
  InstallFailureList,
  PlatformMultiSelectGrid,
  usePlatformTargetSelection,
} from "@/components/platform/PlatformMultiSelect";
import { AgentWithStatus, CentralBatchInstallResult } from "@/types";
import { useTargetStore } from "@/stores/targetStore";
import { isRemoteLikeTarget } from "@/lib/targetKind";
import { hasProjectSkillPattern } from "@/lib/platformTargetGroups";

type TargetMode = "platform" | "project";

interface BatchInstallCentralSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillCount: number;
  agents: AgentWithStatus[];
  isInstalling: boolean;
  title?: string;
  description?: string;
  defaultExcludedAgentIds?: string[];
  onInstall: (
    agentIds: string[],
    method: InstallMethod,
    projectPath?: string | null,
  ) => Promise<CentralBatchInstallResult>;
  onManagePlatforms?: () => void;
}

export function BatchInstallCentralSkillsDialog({
  open,
  onOpenChange,
  skillCount,
  agents,
  isInstalling,
  title,
  description,
  defaultExcludedAgentIds = [],
  onInstall,
  onManagePlatforms,
}: BatchInstallCentralSkillsDialogProps) {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const isRemoteTarget = isRemoteLikeTarget(activeTarget);
  const canUseSymlink = !isRemoteTarget || activeTarget.symlinkEnabled === true;
  const targetAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents],
  );
  const [targetMode, setTargetMode] = useState<TargetMode>("platform");
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [projectPath, setProjectPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CentralBatchInstallResult | null>(null);
  const defaultExcludedKey = defaultExcludedAgentIds.join("\0");
  const defaultExcludedSet = useMemo(
    () => new Set(defaultExcludedKey ? defaultExcludedKey.split("\0") : []),
    [defaultExcludedKey],
  );
  const skipped = useMemo(() => result?.skipped ?? [], [result]);
  const skippedSummary = useMemo(
    () => summarizeSkippedReasons(skipped),
    [skipped],
  );

  const isProjectTargetDisabled = (agent: AgentWithStatus) =>
    targetMode === "project" && !hasProjectSkillPattern(agent);

  const selection = usePlatformTargetSelection({
    targets: targetAgents,
    isTargetDisabled: isProjectTargetDisabled,
    isTargetDefaultSelected: (agent) =>
      !isProjectTargetDisabled(agent) && !defaultExcludedSet.has(agent.id),
  });

  const selectedInstallAgentIds = selection.selectedInstallAgentIds();
  const platformCount = selectedInstallAgentIds.length;
  const { reset: resetSelection } = selection;

  useEffect(() => {
    if (!open) return;

    resetSelection();
    setError(null);
    setResult(null);
    if (targetMode === "platform") {
      setInstallMethod(canUseSymlink ? "symlink" : "copy");
      setProjectPath("");
    } else {
      setInstallMethod("copy");
    }
  }, [
    open,
    targetAgents,
    targetMode,
    isRemoteTarget,
    canUseSymlink,
    defaultExcludedSet,
    resetSelection,
  ]);

  function handleModeChange(mode: TargetMode) {
    setTargetMode(mode);
    setInstallMethod(mode === "project" || !canUseSymlink ? "copy" : "symlink");
    setError(null);
    setResult(null);
  }

  async function handleConfirm() {
    if (selectedInstallAgentIds.length === 0) {
      setError(t("central.batchInstallSelectPlatform"));
      return;
    }
    const trimmedProjectPath = projectPath.trim();
    if (targetMode === "project" && !trimmedProjectPath) {
      setError(t("central.batchInstallProjectPathRequired"));
      return;
    }

    setError(null);
    try {
      const installResult = await onInstall(
        selectedInstallAgentIds,
        installMethod,
        targetMode === "project" ? trimmedProjectPath : null,
      );
      setResult(installResult);
      if (installResult.failed.length === 0) {
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
          <DialogTitle>{title ?? t("central.batchInstallTitle")}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {description ??
              t("central.batchInstallDesc", { count: skillCount })}
          </DialogDescription>

          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("central.batchInstallTargetMode")}
            </p>
            <RadioGroup
              value={targetMode}
              onValueChange={(value) => handleModeChange(value as TargetMode)}
            >
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="platform" />
                <span className="text-sm">
                  {t("central.batchInstallTargetPlatforms")}
                </span>
              </label>
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="project" />
                <span className="text-sm">
                  {t("central.batchInstallTargetProject")}
                </span>
                {isRemoteTarget && (
                  <span className="text-xs text-muted-foreground">
                    {t("targets.projectModeRemoteHint")}
                  </span>
                )}
              </label>
            </RadioGroup>
          </div>

          {targetMode === "project" && (
            <div className="space-y-2">
              <label className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {t("central.batchInstallProjectPath")}
              </label>
              <ProjectPathPicker
                value={projectPath}
                onChange={setProjectPath}
                onError={setError}
                disabled={isInstalling}
                browseEnabled={!isRemoteTarget}
                placeholder={
                  isRemoteTarget
                    ? t("central.batchInstallRemoteProjectPathPlaceholder")
                    : undefined
                }
                hint={
                  isRemoteTarget
                    ? t("central.batchInstallRemoteProjectPathHint")
                    : undefined
                }
              />
            </div>
          )}

          <div className="flex items-center justify-between gap-3">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("central.batchInstallPlatformsLabel")}
            </p>
            {onManagePlatforms && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={onManagePlatforms}
                data-testid="batch-install-manage-platforms"
              >
                <Settings2 className="size-3.5" />
                {t("central.batchInstallManagePlatforms")}
              </Button>
            )}
          </div>

          <PlatformMultiSelectGrid
            targets={targetAgents}
            isSelected={(agent) =>
              selection.isSelected(agent) && !isProjectTargetDisabled(agent)
            }
            isDisabled={isProjectTargetDisabled}
            onToggle={selection.toggle}
            renderBadges={(agent) =>
              isProjectTargetDisabled(agent) ? (
                <span className="shrink-0 text-xs text-muted-foreground">
                  {t("central.batchInstallProjectUnsupported")}
                </span>
              ) : null
            }
            emptyMessage={t("central.batchInstallNoPlatforms")}
            ariaLabel={t("central.batchInstallSelectPlatform")}
          />

          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("central.batchInstallInstallMethod")}
            </p>
            <RadioGroup
              value={installMethod}
              onValueChange={(value) =>
                setInstallMethod(value as InstallMethod)
              }
            >
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="symlink" disabled={!canUseSymlink} />
                <span className="text-sm">
                  {t("central.batchInstallSymlink")}
                </span>
                <span className="text-xs text-muted-foreground">
                  {!canUseSymlink && isRemoteTarget
                    ? t("targets.symlinkDisabled")
                    : t("central.batchInstallSymlinkDesc")}
                </span>
              </label>
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="copy" />
                <span className="text-sm">{t("central.batchInstallCopy")}</span>
                <span className="text-xs text-muted-foreground">
                  {t("central.batchInstallCopyDesc")}
                </span>
              </label>
            </RadioGroup>
          </div>

          {result && result.failed.length > 0 && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-warning-foreground">
                {t("central.batchInstallPartial", {
                  skillCount,
                  platformCount,
                  succeededCount: result.succeeded.length,
                  skippedCount: skipped.length,
                  failedCount: result.failed.length,
                })}
              </p>
              {skippedSummary.length > 0 && (
                <div
                  className="rounded-md border border-border/70 bg-muted/35 p-2 text-xs text-muted-foreground"
                  data-testid="batch-install-skipped-summary"
                >
                  <p className="font-medium text-foreground">
                    {t("central.batchInstallSkippedDetails")}
                  </p>
                  <ul className="mt-1 space-y-0.5">
                    {skippedSummary.map(({ reason, count }) => (
                      <li key={reason}>
                        {t(`central.batchInstallSkipReasons.${reason}`, {
                          defaultValue: reason,
                        })}
                        : {count}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              <InstallFailureList
                failures={result.failed.map((failure) => ({
                  key: `${failure.skill_id}:${failure.agent_id}`,
                  label: `${failure.skill_id} / ${failure.agent_id}: ${failure.error}`,
                }))}
              />
            </div>
          )}

          {result && result.failed.length === 0 && skipped.length > 0 && (
            <div className="space-y-2" role="status">
              <p className="text-xs font-medium text-muted-foreground">
                {t("central.batchInstallSkipped", {
                  skippedCount: skipped.length,
                  succeededCount: result.succeeded.length,
                })}
              </p>
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
            disabled={isInstalling}
          >
            {result?.failed.length
              ? t("central.batchInstallClose")
              : t("installDialog.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isInstalling || platformCount === 0}
          >
            {isInstalling ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.batchInstallInstalling")}
              </>
            ) : (
              t("central.batchInstallConfirm", { skillCount, platformCount })
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function summarizeSkippedReasons(
  skipped: NonNullable<CentralBatchInstallResult["skipped"]>,
): Array<{ reason: string; count: number }> {
  const counts = new Map<string, number>();
  for (const item of skipped) {
    counts.set(item.reason, (counts.get(item.reason) ?? 0) + 1);
  }
  return Array.from(counts, ([reason, count]) => ({ reason, count })).sort(
    (a, b) => a.reason.localeCompare(b.reason),
  );
}
