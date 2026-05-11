import { useEffect, useMemo, useState } from "react";
import { Loader2 } from "lucide-react";
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
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import type { InstallMethod } from "@/components/central/InstallDialog";
import { ProjectPathPicker } from "@/components/central/ProjectPathPicker";
import { AgentWithStatus, CentralBatchInstallResult } from "@/types";
import { useTargetStore } from "@/stores/targetStore";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberNames,
  hasProjectSkillPattern,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

type TargetMode = "platform" | "project";

interface BatchInstallCentralSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillCount: number;
  agents: AgentWithStatus[];
  isInstalling: boolean;
  onInstall: (
    agentIds: string[],
    method: InstallMethod,
    projectPath?: string | null
  ) => Promise<CentralBatchInstallResult>;
}

export function BatchInstallCentralSkillsDialog({
  open,
  onOpenChange,
  skillCount,
  agents,
  isInstalling,
  onInstall,
}: BatchInstallCentralSkillsDialogProps) {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const isRemoteTarget = activeTarget.kind === "ssh";
  const canUseSymlink = !isRemoteTarget || activeTarget.symlinkEnabled === true;
  const targetAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents]
  );
  const [targetMode, setTargetMode] = useState<TargetMode>("platform");
  const [selectedAgentIds, setSelectedAgentIds] = useState<Set<string>>(new Set());
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [projectPath, setProjectPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CentralBatchInstallResult | null>(null);

  const isProjectTargetDisabled = (agent: AgentWithStatus) =>
    targetMode === "project" && !hasProjectSkillPattern(agent);

  const selectedInstallAgentIds = useMemo(
    () =>
      Array.from(
        new Set(
          targetAgents
            .filter((agent) => selectedAgentIds.has(agent.id))
            .filter((agent) => targetMode !== "project" || hasProjectSkillPattern(agent))
            .flatMap((agent) => getPlatformTargetInstallAgentIds(agent))
        )
      ),
    [selectedAgentIds, targetAgents, targetMode]
  );
  const platformCount = selectedInstallAgentIds.length;

  useEffect(() => {
    if (!open) return;

    const initialSelection = new Set(
      targetAgents
        .filter((agent) => targetMode === "platform" || hasProjectSkillPattern(agent))
        .map((agent) => agent.id)
    );
    setSelectedAgentIds(initialSelection);
    setError(null);
    setResult(null);
    if (isRemoteTarget) {
      setTargetMode("platform");
      setInstallMethod(canUseSymlink ? "symlink" : "copy");
      setProjectPath("");
    } else if (targetMode === "platform") {
      setInstallMethod(canUseSymlink ? "symlink" : "copy");
      setProjectPath("");
    } else {
      setInstallMethod("copy");
    }
  }, [open, targetAgents, targetMode, isRemoteTarget, canUseSymlink]);

  function handleModeChange(mode: TargetMode) {
    if (isRemoteTarget && mode === "project") {
      return;
    }
    setTargetMode(mode);
    setInstallMethod(mode === "project" || !canUseSymlink ? "copy" : "symlink");
    setError(null);
    setResult(null);
  }

  function handleToggle(agentId: string, checked: boolean) {
    const target = targetAgents.find((agent) => agent.id === agentId);
    if (target && isProjectTargetDisabled(target)) {
      return;
    }

    setSelectedAgentIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(agentId);
      } else {
        next.delete(agentId);
      }
      return next;
    });
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
        targetMode === "project" ? trimmedProjectPath : null
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
          <DialogTitle>{t("central.batchInstallTitle")}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {t("central.batchInstallDesc", { count: skillCount })}
          </DialogDescription>

          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("central.batchInstallTargetMode")}
            </p>
            <RadioGroup value={targetMode} onValueChange={(value) => handleModeChange(value as TargetMode)}>
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="platform" />
                <span className="text-sm">{t("central.batchInstallTargetPlatforms")}</span>
              </label>
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="project" disabled={isRemoteTarget} />
                <span className="text-sm">{t("central.batchInstallTargetProject")}</span>
                {isRemoteTarget && (
                  <span className="text-xs text-muted-foreground">
                    {t("targets.discoverUnsupported")}
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
              />
            </div>
          )}

          <div className="grid grid-cols-2 gap-x-4 gap-y-2" role="group" aria-label={t("central.batchInstallSelectPlatform")}>
            {targetAgents.length === 0 ? (
              <p className="col-span-2 text-sm text-muted-foreground">
                {t("central.batchInstallNoPlatforms")}
              </p>
            ) : (
              targetAgents.map((agent) => {
                const isUniversal = isUniversalPlatformTarget(agent);
                const isDisabled = isProjectTargetDisabled(agent);
                const isChecked = selectedAgentIds.has(agent.id) && !isDisabled;
                const displayName = isUniversal
                  ? t("platformTargets.universalLabel")
                  : agent.display_name;
                const memberNames = getPlatformTargetMemberNames(agent).join(", ");

                return (
                  <div key={agent.id} className="flex items-center gap-2">
                    <Checkbox
                      checked={isChecked}
                      disabled={isDisabled}
                      onCheckedChange={(checked) => handleToggle(agent.id, !!checked)}
                      aria-label={displayName}
                    />
                    <div
                      className={`min-w-0 flex-1 select-none ${
                        isDisabled
                          ? "cursor-default text-muted-foreground"
                          : "cursor-pointer text-foreground"
                      }`}
                      title={isUniversal ? memberNames : agent.global_skills_dir}
                      onClick={() => !isDisabled && handleToggle(agent.id, !isChecked)}
                    >
                      <div className="truncate text-sm">{displayName}</div>
                      {isUniversal && (
                        <div className="truncate text-[11px] text-muted-foreground">
                          {memberNames}
                        </div>
                      )}
                    </div>
                    {isDisabled && (
                      <span className="shrink-0 text-xs text-muted-foreground">
                        {t("central.batchInstallProjectUnsupported")}
                      </span>
                    )}
                  </div>
                );
              })
            )}
          </div>

          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("central.batchInstallInstallMethod")}
            </p>
            <RadioGroup
              value={installMethod}
              onValueChange={(value) => setInstallMethod(value as InstallMethod)}
            >
              <label className="flex cursor-pointer items-center gap-2.5">
                <RadioItem value="symlink" disabled={!canUseSymlink} />
                <span className="text-sm">{t("central.batchInstallSymlink")}</span>
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
              <p className="text-xs font-medium text-amber-600 dark:text-amber-400">
                {t("central.batchInstallPartial", {
                  skillCount,
                  platformCount,
                  failedCount: result.failed.length,
                })}
              </p>
              <ul className="max-h-32 space-y-0.5 overflow-auto text-xs text-destructive">
                {result.failed.map((failure) => (
                  <li key={`${failure.skill_id}:${failure.agent_id}`}>
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
            disabled={isInstalling}
          >
            {result?.failed.length ? t("central.batchInstallClose") : t("installDialog.cancel")}
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
