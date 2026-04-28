import { useEffect, useMemo, useState } from "react";
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
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import { AgentWithStatus, BatchInstallResult, SkillWithLinks } from "@/types";
import { useTargetStore } from "@/stores/targetStore";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
  hasProjectSkillPattern,
  isUniversalPlatformTarget,
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
    projectPath?: string | null
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
  const isRemoteTarget = activeTarget.kind === "ssh";
  const canUseSymlink = !isRemoteTarget || activeTarget.symlinkEnabled === true;
  const canInstallToProject = Boolean(skill?.is_central) && !isRemoteTarget;
  // Only show non-central agents in the install dialog.
  const targetAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents]
  );
  const sharedRootAgentIds = new Set(skill?.shared_root_agents ?? []);

  const isSharedRootTarget = (agent: AgentWithStatus) =>
    getPlatformTargetMemberIds(agent).some((agentId) => sharedRootAgentIds.has(agentId));

  const selectedInstallAgentIds = () =>
    Array.from(
      new Set(
        targetAgents
          .filter((agent) => selectedAgentIds.has(agent.id))
          .filter((agent) => targetMode !== "platform" || !isSharedRootTarget(agent))
          .filter((agent) => targetMode !== "project" || hasProjectSkillPattern(agent))
          .flatMap((agent) => getPlatformTargetInstallAgentIds(agent))
      )
    );

  // Track which agents are selected for installation.
  const [selectedAgentIds, setSelectedAgentIds] = useState<Set<string>>(
    new Set()
  );
  const [targetMode, setTargetMode] = useState<TargetMode>("platform");
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [projectPath, setProjectPath] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BatchInstallResult | null>(null);

  // When the dialog opens for a skill, pre-select currently unlinked agents.
  // Agents that already have this skill are checked by default too so the
  // user can see the full picture, but they can deselect any.
  useEffect(() => {
    if (open && skill) {
      const effectiveTargetMode = canInstallToProject ? targetMode : "platform";
      if (effectiveTargetMode !== targetMode) {
        setTargetMode(effectiveTargetMode);
      }
      // Default: check all enabled and visible platform targets.
      const initialSelection = new Set<string>(
        targetAgents
          .filter((agent) => effectiveTargetMode !== "platform" || !isSharedRootTarget(agent))
          .filter((agent) => effectiveTargetMode !== "project" || hasProjectSkillPattern(agent))
          .map((agent) => agent.id)
      );
      setSelectedAgentIds(initialSelection);
      setInstallMethod(
        effectiveTargetMode === "project" || !canUseSymlink ? "copy" : "symlink"
      );
      if (effectiveTargetMode === "platform") {
        setProjectPath("");
      }
      setError(null);
      setResult(null);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, skill?.id, targetMode, targetAgents, canInstallToProject, canUseSymlink]);

  const isProjectTargetDisabled = (agent: AgentWithStatus) =>
    targetMode === "project" && !hasProjectSkillPattern(agent);

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

  function handleCheckboxChange(agentId: string, checked: boolean) {
    const target = targetAgents.find((agent) => agent.id === agentId);
    if (
      target &&
      ((targetMode === "platform" && isSharedRootTarget(target)) ||
        isProjectTargetDisabled(target))
    ) {
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
        targetMode === "project" ? trimmedProjectPath : null
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
          <DialogTitle>{t("installDialog.title", { name: skill.name })}</DialogTitle>
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
                  <span className="text-sm">{t("central.batchInstallTargetPlatforms")}</span>
                </label>
                <label className="flex items-center gap-2.5 cursor-pointer">
                  <RadioItem value="project" />
                  <span className="text-sm">{t("central.batchInstallTargetProject")}</span>
                </label>
              </RadioGroup>
            </div>
          )}

          {targetMode === "project" && canInstallToProject && (
            <div className="space-y-2">
              <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {t("central.batchInstallProjectPath")}
              </label>
              <Input
                value={projectPath}
                onChange={(event) => setProjectPath(event.target.value)}
                placeholder={t("central.batchInstallProjectPathPlaceholder")}
              />
            </div>
          )}

          {/* Platform checkboxes */}
          <div className="grid grid-cols-2 gap-x-4 gap-y-2" role="group" aria-label={t("installDialog.selectPlatforms")}>
            {targetAgents.length === 0 ? (
              <p className="col-span-2 text-sm text-muted-foreground">
                {t("installDialog.noPlatforms")}
              </p>
            ) : (
              targetAgents.map((agent) => {
                const memberIds = getPlatformTargetMemberIds(agent);
                const memberNames = getPlatformTargetMemberNames(agent).join(", ");
                const isUniversal = isUniversalPlatformTarget(agent);
                const displayName = isUniversal
                  ? t("platformTargets.universalLabel")
                  : agent.display_name;
                const isLinked =
                  targetMode === "platform" &&
                  memberIds.some((agentId) => skill.linked_agents.includes(agentId));
                const isSharedRoot =
                  targetMode === "platform" && isSharedRootTarget(agent);
                const isProjectDisabled = isProjectTargetDisabled(agent);
                const isDisabled = isSharedRoot || isProjectDisabled;
                const isChecked =
                  (isSharedRoot || selectedAgentIds.has(agent.id)) && !isProjectDisabled;

                return (
                  <div
                    key={agent.id}
                    className="flex items-center gap-2"
                  >
                    <Checkbox
                      checked={isChecked}
                      disabled={isDisabled}
                      onCheckedChange={(checked) =>
                        handleCheckboxChange(agent.id, !!checked)
                      }
                      aria-label={displayName}
                    />
                    <div
                      className={`min-w-0 flex-1 select-none ${
                        isDisabled
                          ? "text-muted-foreground cursor-default"
                          : "text-foreground cursor-pointer"
                      }`}
                      title={isUniversal ? memberNames : agent.global_skills_dir}
                      onClick={() =>
                        !isDisabled && handleCheckboxChange(agent.id, !isChecked)
                      }
                    >
                      <div className="truncate text-sm">{displayName}</div>
                      {isUniversal && (
                        <div className="truncate text-[11px] text-muted-foreground">
                          {memberNames}
                        </div>
                      )}
                    </div>
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
                  </div>
                );
              })
            )}
          </div>

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
              <p className="text-xs font-medium text-amber-600 dark:text-amber-400">
                {t("central.installPartialFail", {
                  platforms: result.failed.map((failure) => failure.agent_id).join(", "),
                })}
              </p>
              <ul className="max-h-32 space-y-0.5 overflow-auto text-xs text-destructive">
                {result.failed.map((failure) => (
                  <li key={failure.agent_id}>
                    {failure.agent_id}: {failure.error}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Error message */}
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
              t("installDialog.confirmInstall", { count: selectedInstallableCount })
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
