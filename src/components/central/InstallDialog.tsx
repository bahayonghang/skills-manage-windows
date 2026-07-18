import { useCallback, useEffect, useMemo, useState } from "react";
import { Link2, Loader2 } from "lucide-react";
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
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import { ProjectPathPicker } from "@/components/central/ProjectPathPicker";
import {
  InstallFailureList,
  PlatformMultiSelectGrid,
  usePlatformTargetSelection,
} from "@/components/platform/PlatformMultiSelect";
import { AgentWithStatus, BatchInstallResult, SkillWithLinks } from "@/types";
import { useTargetStore } from "@/stores/targetStore";
import { isRemoteLikeTarget } from "@/lib/targetKind";
import {
  getPlatformTargetLabel,
  getPlatformTargetMemberIds,
  hasProjectSkillPattern,
} from "@/lib/platformTargetGroups";

// ─── Types ────────────────────────────────────────────────────────────────────

export type InstallMethod = "symlink" | "copy";
type TargetMode = "platform" | "project";

interface InstallDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skill: SkillWithLinks | null;
  /** All agents (the 'central' agent will be filtered out). */
  agents: AgentWithStatus[];
  onInstall: (
    skillId: string,
    agentIds: string[],
    method: InstallMethod,
    projectPath?: string | null,
  ) => Promise<BatchInstallResult>;
}

// ─── InstallDialog ────────────────────────────────────────────────────────────

export function InstallDialog({
  open,
  onOpenChange,
  skill,
  agents,
  onInstall,
}: InstallDialogProps) {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const isRemoteTarget = isRemoteLikeTarget(activeTarget);
  const canUseSymlink = !isRemoteTarget || activeTarget.symlinkEnabled === true;
  const canInstallToProject = Boolean(skill?.is_central);
  // Only show non-central agents in the install dialog.
  const targetAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents],
  );
  const sharedRootAgentIds = useMemo(
    () => new Set(skill?.shared_root_agents ?? []),
    [skill?.shared_root_agents],
  );

  const isSharedRootTarget = useCallback(
    (agent: AgentWithStatus) =>
      getPlatformTargetMemberIds(agent).some((agentId) =>
        sharedRootAgentIds.has(agentId),
      ),
    [sharedRootAgentIds],
  );

  const [targetMode, setTargetMode] = useState<TargetMode>("platform");
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [projectPath, setProjectPath] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BatchInstallResult | null>(null);
  const skipped = result?.skipped ?? [];

  const isProjectTargetDisabled = (agent: AgentWithStatus) =>
    targetMode === "project" && !hasProjectSkillPattern(agent);

  const isTargetDisabled = (agent: AgentWithStatus) =>
    (targetMode === "platform" && isSharedRootTarget(agent)) ||
    isProjectTargetDisabled(agent);

  // Selection default: check all enabled and visible platform targets
  // (shared-root targets in platform mode and pattern-less targets in project
  // mode are disabled and therefore excluded).
  const selection = usePlatformTargetSelection({
    targets: targetAgents,
    isTargetDisabled,
  });
  const selectedInstallAgentIds = selection.selectedInstallAgentIds;
  const { reset: resetSelection } = selection;

  // When the dialog opens for a skill, pre-select currently unlinked agents.
  // Agents that already have this skill are checked by default too so the
  // user can see the full picture, but they can deselect any.
  useEffect(() => {
    if (open && skill) {
      const effectiveTargetMode = canInstallToProject ? targetMode : "platform";
      if (effectiveTargetMode !== targetMode) {
        setTargetMode(effectiveTargetMode);
        // The effect re-runs with the corrected mode and resets then.
        return;
      }
      resetSelection();
      setInstallMethod(
        effectiveTargetMode === "project" || !canUseSymlink
          ? "copy"
          : "symlink",
      );
      if (effectiveTargetMode === "platform") {
        setProjectPath("");
      }
      setError(null);
      setResult(null);
    }
  }, [
    canInstallToProject,
    canUseSymlink,
    isSharedRootTarget,
    open,
    skill,
    targetAgents,
    targetMode,
    resetSelection,
  ]);

  function handleModeChange(mode: TargetMode) {
    if (mode === "project" && !canInstallToProject) {
      return;
    }
    setTargetMode(mode);
    setInstallMethod(mode === "project" || !canUseSymlink ? "copy" : "symlink");
    if (mode === "platform") {
      setProjectPath("");
    }
    setError(null);
    setResult(null);
  }

  async function handleConfirm() {
    if (!skill) return;

    const agentIds = selectedInstallAgentIds();
    if (agentIds.length === 0) {
      setError(t("installDialog.selectPlatform"));
      return;
    }
    const trimmedProjectPath = projectPath.trim();
    if (targetMode === "project" && !trimmedProjectPath) {
      setError(t("central.batchInstallProjectPathRequired"));
      return;
    }

    setIsLoading(true);
    setError(null);
    setResult(null);
    try {
      const installResult = await onInstall(
        skill.id,
        agentIds,
        installMethod,
        targetMode === "project" ? trimmedProjectPath : null,
      );
      setResult(installResult);
      if (installResult.failed.length === 0) {
        onOpenChange(false);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }

  if (!skill) return null;
  const selectedInstallableCount = selectedInstallAgentIds().length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("installDialog.title", { name: skill.name })}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {canInstallToProject
              ? t("installDialog.chooseTarget")
              : t("installDialog.choosePlatforms")}
          </DialogDescription>

          {canInstallToProject && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {t("central.batchInstallTargetMode")}
              </p>
              <RadioGroup
                value={targetMode}
                onValueChange={(value) => handleModeChange(value as TargetMode)}
              >
                <label className="flex items-center gap-2.5 cursor-pointer">
                  <RadioItem value="platform" />
                  <span className="text-sm">
                    {t("central.batchInstallTargetPlatforms")}
                  </span>
                </label>
                <label className="flex items-center gap-2.5 cursor-pointer">
                  <RadioItem value="project" />
                  <span className="text-sm">
                    {t("central.batchInstallTargetProject")}
                  </span>
                </label>
              </RadioGroup>
            </div>
          )}

          {targetMode === "project" && canInstallToProject && (
            <div className="space-y-2">
              <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {t("central.batchInstallProjectPath")}
              </label>
              <ProjectPathPicker
                value={projectPath}
                onChange={setProjectPath}
                onError={setError}
                disabled={isLoading}
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

          {/* Platform checkboxes */}
          <PlatformMultiSelectGrid
            targets={targetAgents}
            isSelected={(agent) =>
              ((targetMode === "platform" && isSharedRootTarget(agent)) ||
                selection.isSelected(agent)) &&
              !isProjectTargetDisabled(agent)
            }
            isDisabled={isTargetDisabled}
            onToggle={selection.toggle}
            renderBadges={(agent) => {
              const isSharedRoot =
                targetMode === "platform" && isSharedRootTarget(agent);
              const isLinked =
                targetMode === "platform" &&
                getPlatformTargetMemberIds(agent).some((agentId) =>
                  skill.linked_agents.includes(agentId),
                );
              const isProjectDisabled = isProjectTargetDisabled(agent);
              const displayName = getPlatformTargetLabel(agent, t, "full");

              return (
                <>
                  {isSharedRoot && (
                    <span className="text-xs text-primary shrink-0">
                      {t("platformTargets.alwaysIncluded")}
                    </span>
                  )}
                  {isLinked && !isSharedRoot && (
                    <span
                      className="inline-flex size-4 shrink-0 items-center justify-center rounded-full border border-primary/30 bg-primary/10 text-primary"
                      aria-label={t("installDialog.alreadyLinkedLabel", {
                        platform: displayName,
                      })}
                      title={t("installDialog.alreadyLinked")}
                    >
                      <Link2 className="size-3" aria-hidden="true" />
                    </span>
                  )}
                  {isProjectDisabled && (
                    <span className="text-xs text-muted-foreground shrink-0">
                      {t("central.batchInstallProjectUnsupported")}
                    </span>
                  )}
                  {!agent.is_detected && (
                    <span className="text-xs text-muted-foreground shrink-0">
                      {t("installDialog.notDetected")}
                    </span>
                  )}
                </>
              );
            }}
            emptyMessage={t("installDialog.noPlatforms")}
            ariaLabel={t("installDialog.selectPlatforms")}
          />

          {/* Install method selector */}
          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {t("installDialog.installMethod")}
            </p>
            <RadioGroup
              value={installMethod}
              onValueChange={(v) => setInstallMethod(v as InstallMethod)}
            >
              <label className="flex items-center gap-2.5 cursor-pointer">
                <RadioItem value="symlink" disabled={!canUseSymlink} />
                <span className="text-sm">{t("installDialog.symlink")}</span>
                <span className="text-xs text-muted-foreground">
                  {!canUseSymlink && isRemoteTarget
                    ? t("targets.symlinkDisabled")
                    : t("installDialog.symlinkDesc")}
                </span>
              </label>
              <label className="flex items-center gap-2.5 cursor-pointer">
                <RadioItem value="copy" />
                <span className="text-sm">{t("installDialog.copy")}</span>
                <span className="text-xs text-muted-foreground">
                  {t("installDialog.copyDesc")}
                </span>
              </label>
            </RadioGroup>
          </div>

          {result && result.failed.length > 0 && (
            <div className="space-y-2" role="alert">
              <p className="text-xs font-medium text-warning-foreground">
                {t("central.installPartialFail", {
                  platforms: result.failed
                    .map((failure) => failure.agent_id)
                    .join(", "),
                  succeededCount: result.succeeded.length,
                  skippedCount: skipped.length,
                  failedCount: result.failed.length,
                })}
              </p>
              <InstallFailureList
                failures={result.failed.map((failure) => ({
                  key: failure.agent_id,
                  label: `${failure.agent_id}: ${failure.error}`,
                }))}
              />
            </div>
          )}

          {result && result.failed.length === 0 && skipped.length > 0 && (
            <div className="space-y-2" role="status">
              <p className="text-xs font-medium text-muted-foreground">
                {t("central.installSkipped", {
                  skippedCount: skipped.length,
                  succeededCount: result.succeeded.length,
                })}
              </p>
            </div>
          )}

          {/* Error message */}
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
            disabled={isLoading}
          >
            {t("installDialog.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isLoading || selectedInstallableCount === 0}
          >
            {isLoading ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("installDialog.installing")}
              </>
            ) : (
              t("installDialog.confirmInstall", {
                count: selectedInstallableCount,
              })
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
