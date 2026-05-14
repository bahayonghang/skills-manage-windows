import { useEffect, useMemo, useState } from "react";
import { Loader2, Search, X } from "lucide-react";
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
import { Input } from "@/components/ui/input";
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { formatPathForDisplay } from "@/lib/path";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
  hasProjectSkillPattern,
  isUniversalPlatformTarget,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import type { Project, ProjectSkill, SkillWithLinks } from "@/types";

type InstallMethod = "symlink" | "copy";

interface ProjectInstallDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  project: Project | null;
  centralSkills: SkillWithLinks[];
  platformTargets: PlatformTarget[];
  existingSkills: ProjectSkill[];
  isInstalling: boolean;
  onConfirm: (
    skillId: string,
    agentIds: string[],
    method: InstallMethod
  ) => Promise<void>;
}

export function ProjectInstallDialog({
  open,
  onOpenChange,
  project,
  centralSkills,
  platformTargets,
  existingSkills,
  isInstalling,
  onConfirm,
}: ProjectInstallDialogProps) {
  const { t } = useTranslation();
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null);
  const [selectedTargetIds, setSelectedTargetIds] = useState<Set<string>>(
    new Set()
  );
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [skillSearch, setSkillSearch] = useState("");
  const [error, setError] = useState<string | null>(null);

  const eligibleTargets = useMemo(
    () =>
      platformTargets.filter(
        (target) =>
          target.id !== "central" &&
          target.is_enabled &&
          hasProjectSkillPattern(target)
      ),
    [platformTargets]
  );

  const selectedTargets = useMemo(
    () => eligibleTargets.filter((target) => selectedTargetIds.has(target.id)),
    [eligibleTargets, selectedTargetIds]
  );

  const selectedInstallAgentIds = useMemo(
    () =>
      Array.from(
        new Set(selectedTargets.flatMap((target) => getPlatformTargetInstallAgentIds(target)))
      ),
    [selectedTargets]
  );

  const filteredSkills = useMemo(() => {
    const q = skillSearch.trim().toLowerCase();
    if (!q) return centralSkills;
    return centralSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description ?? "").toLowerCase().includes(q)
    );
  }, [centralSkills, skillSearch]);

  useEffect(() => {
    if (!open) return;
    setSelectedSkillId(null);
    setSelectedTargetIds(new Set(eligibleTargets.map((target) => target.id)));
    setInstallMethod("symlink");
    setSkillSearch("");
    setError(null);
  }, [open, eligibleTargets]);

  function getTargetDisplayName(target: PlatformTarget): string {
    return isUniversalPlatformTarget(target)
      ? t("platformTargets.universalShortLabel")
      : target.display_name;
  }

  function getTargetTitle(target: PlatformTarget): string {
    return isUniversalPlatformTarget(target)
      ? getPlatformTargetMemberNames(target).join(", ")
      : target.global_skills_dir;
  }

  function existingForTarget(
    skillId: string,
    target: PlatformTarget
  ): ProjectSkill | undefined {
    const memberIds = new Set(getPlatformTargetMemberIds(target));
    return existingSkills.find(
      (s) => s.skillId === skillId && memberIds.has(s.agentId)
    );
  }

  function toggleTarget(id: string) {
    setSelectedTargetIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function handleConfirm() {
    if (!selectedSkillId) {
      setError(t("projectInstall.noSkillSelected"));
      return;
    }
    if (selectedInstallAgentIds.length === 0) {
      setError(t("projectInstall.noAgentSelected"));
      return;
    }
    setError(null);
    try {
      await onConfirm(
        selectedSkillId,
        selectedInstallAgentIds,
        installMethod
      );
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
    }
  }

  if (!project) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("projectInstall.title", { name: project.name })}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-5">
          <DialogDescription>
            {t("projectInstall.description", {
              path: formatPathForDisplay(project.path),
            })}
          </DialogDescription>

          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {t("projectInstall.pickSkill")}
            </p>
            <div className="relative">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
              <Input
                placeholder={t("projectInstall.searchSkills")}
                value={skillSearch}
                onChange={(e) => setSkillSearch(e.target.value)}
                className="pl-7 pr-7 h-8 text-sm"
              />
              {skillSearch && (
                <button
                  type="button"
                  onClick={() => setSkillSearch("")}
                  className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-muted"
                  aria-label={t("projects.clearSearch")}
                >
                  <X className="size-3" />
                </button>
              )}
            </div>
            <div className="max-h-48 overflow-y-auto rounded-md border border-border">
              {filteredSkills.length === 0 ? (
                <p className="text-xs text-muted-foreground px-3 py-4 text-center">
                  {t("projectInstall.noSkills")}
                </p>
              ) : (
                filteredSkills.map((skill) => {
                  const isSelected = selectedSkillId === skill.id;
                  return (
                    <button
                      key={skill.id}
                      type="button"
                      onClick={() => setSelectedSkillId(skill.id)}
                      className={cn(
                        "w-full text-left px-3 py-2 text-sm border-b border-border last:border-0 cursor-pointer transition-colors",
                        isSelected
                          ? "bg-primary/10 text-foreground"
                          : "hover:bg-muted/40"
                      )}
                    >
                      <div className="font-medium truncate">{skill.name}</div>
                      {skill.description && (
                        <div className="text-xs text-muted-foreground truncate">
                          {skill.description}
                        </div>
                      )}
                    </button>
                  );
                })
              )}
            </div>
          </div>

          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {t("projectInstall.pickAgents")}
            </p>
            {eligibleTargets.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                {t("projectInstall.noEligibleAgents")}
              </p>
            ) : (
              <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                {eligibleTargets.map((target) => {
                  const exists = selectedSkillId
                    ? existingForTarget(selectedSkillId, target)
                    : undefined;
                  const isChecked = selectedTargetIds.has(target.id);
                  const displayName = getTargetDisplayName(target);
                  return (
                    <label
                      key={target.id}
                      title={getTargetTitle(target)}
                      className="flex items-center gap-2 cursor-pointer text-sm"
                    >
                      <Checkbox
                        checked={isChecked}
                        onCheckedChange={() => toggleTarget(target.id)}
                      />
                      <PlatformIcon
                        agentId={target.id}
                        className="size-3.5 shrink-0"
                      />
                      <span className="flex-1 truncate">
                        {displayName}
                      </span>
                      {exists ? (
                        <span className="text-[10px] text-amber-600 dark:text-amber-400 shrink-0">
                          {t("projectInstall.willReplace", {
                            type: exists.linkType,
                          })}
                        </span>
                      ) : (
                        <span className="text-[10px] text-muted-foreground shrink-0">
                          {t("projectInstall.willCreate")}
                        </span>
                      )}
                    </label>
                  );
                })}
              </div>
            )}
          </div>

          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {t("installDialog.installMethod")}
            </p>
            <RadioGroup
              value={installMethod}
              onValueChange={(v) => setInstallMethod(v as InstallMethod)}
            >
              <label className="flex items-center gap-2.5 cursor-pointer">
                <RadioItem value="symlink" />
                <span className="text-sm">{t("installDialog.symlink")}</span>
                <span className="text-xs text-muted-foreground">
                  {t("installDialog.symlinkDesc")}
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
            {t("installDialog.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={
              isInstalling || !selectedSkillId || selectedInstallAgentIds.length === 0
            }
          >
            {isInstalling ? (
              <>
                <Loader2 className="size-3.5 mr-1 animate-spin" />
                {t("installDialog.installing")}
              </>
            ) : (
              t("projectInstall.confirm", { count: selectedTargets.length })
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
