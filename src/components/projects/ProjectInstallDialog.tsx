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
import { Input } from "@/components/ui/input";
import { RadioGroup, RadioItem } from "@/components/ui/radio-group";
import {
  PlatformMultiSelectGrid,
  usePlatformTargetSelection,
} from "@/components/platform/PlatformMultiSelect";
import { formatPathForDisplay } from "@/lib/path";
import {
  getPlatformTargetMemberIds,
  hasProjectSkillPattern,
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
    method: InstallMethod,
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
  const [installMethod, setInstallMethod] = useState<InstallMethod>("symlink");
  const [skillSearch, setSkillSearch] = useState("");
  const [error, setError] = useState<string | null>(null);

  const eligibleTargets = useMemo(
    () =>
      platformTargets.filter(
        (target) =>
          target.id !== "central" &&
          target.is_enabled &&
          hasProjectSkillPattern(target),
      ),
    [platformTargets],
  );

  const selection = usePlatformTargetSelection({ targets: eligibleTargets });

  const selectedTargets = eligibleTargets.filter((target) =>
    selection.isSelected(target),
  );

  const selectedInstallAgentIds = selection.selectedInstallAgentIds();

  const filteredSkills = useMemo(() => {
    const q = skillSearch.trim().toLowerCase();
    if (!q) return centralSkills;
    return centralSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description ?? "").toLowerCase().includes(q),
    );
  }, [centralSkills, skillSearch]);

  const { reset: resetSelection } = selection;

  useEffect(() => {
    if (!open) return;
    setSelectedSkillId(null);
    resetSelection();
    setInstallMethod("symlink");
    setSkillSearch("");
    setError(null);
  }, [open, eligibleTargets, resetSelection]);

  function existingForTarget(
    skillId: string,
    target: PlatformTarget,
  ): ProjectSkill | undefined {
    const memberIds = new Set(getPlatformTargetMemberIds(target));
    return existingSkills.find(
      (s) => s.skillId === skillId && memberIds.has(s.agentId),
    );
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
      await onConfirm(selectedSkillId, selectedInstallAgentIds, installMethod);
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
                          : "hover:bg-muted/40",
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
            <PlatformMultiSelectGrid
              targets={eligibleTargets}
              isSelected={selection.isSelected}
              onToggle={selection.toggle}
              showIcon
              labelVariant="short"
              renderBadges={(target) => {
                const exists = selectedSkillId
                  ? existingForTarget(selectedSkillId, target)
                  : undefined;
                return exists ? (
                  <span className="text-[10px] text-warning-foreground shrink-0">
                    {t("projectInstall.willReplace", {
                      type: exists.linkType,
                    })}
                  </span>
                ) : (
                  <span className="text-[10px] text-muted-foreground shrink-0">
                    {t("projectInstall.willCreate")}
                  </span>
                );
              }}
              emptyMessage={t("projectInstall.noEligibleAgents")}
              ariaLabel={t("projectInstall.pickAgents")}
            />
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
              isInstalling ||
              !selectedSkillId ||
              selectedInstallAgentIds.length === 0
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
