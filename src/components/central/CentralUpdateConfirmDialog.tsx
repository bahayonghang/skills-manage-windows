import { useEffect, useMemo, useState } from "react";
import { Download, Loader2 } from "lucide-react";
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
  CentralSkillUpdateState,
  SkillRepository,
  SkillWithLinks,
} from "@/types";

export interface CentralUpdateConfirmSkill {
  id: string;
  name: string;
  description?: string | null;
  repository?: SkillRepository;
  source_path?: string | null;
}

interface CentralUpdateConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  states: CentralSkillUpdateState[];
  skills: CentralUpdateConfirmSkill[] | SkillWithLinks[];
  isApplying: boolean;
  onConfirm: (skillIds: string[]) => Promise<void>;
}

function formatDate(value?: string | null): string | null {
  if (!value) return null;
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return value;
  return new Date(timestamp).toLocaleString();
}

/**
 * @deprecated 使用 `UpdateCenterDialog` 代替（plans/update-mechanism-overhaul-plan.md P5/P6）。
 * 本组件保留以兼容旧 workflow（单条更新入口、SkillDetailView 内的更新按钮），
 * 将在下个 minor release 删除。
 */
export function CentralUpdateConfirmDialog({
  open,
  onOpenChange,
  states,
  skills,
  isApplying,
  onConfirm,
}: CentralUpdateConfirmDialogProps) {
  const { t } = useTranslation();
  const [selectedSkillIds, setSelectedSkillIds] = useState<string[]>([]);
  const statesKey = useMemo(
    () => states.map((state) => state.skill_id).join("\0"),
    [states]
  );
  const skillById = useMemo(
    () => new Map(skills.map((skill) => [skill.id, skill])),
    [skills]
  );

  useEffect(() => {
    if (open) {
      setSelectedSkillIds(states.map((state) => state.skill_id));
    }
  }, [open, statesKey, states]);

  const selectedSet = useMemo(() => new Set(selectedSkillIds), [selectedSkillIds]);
  const allSelected = states.length > 0 && selectedSkillIds.length === states.length;
  const selectedCount = selectedSkillIds.length;

  function toggleSkill(skillId: string, checked: boolean) {
    setSelectedSkillIds((current) => {
      if (checked) {
        return current.includes(skillId) ? current : [...current, skillId];
      }
      return current.filter((id) => id !== skillId);
    });
  }

  function toggleAll(checked: boolean) {
    setSelectedSkillIds(checked ? states.map((state) => state.skill_id) : []);
  }

  async function handleConfirm() {
    if (selectedSkillIds.length === 0) return;
    await onConfirm(selectedSkillIds);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("central.updateConfirmTitle")}</DialogTitle>
          <DialogDescription>
            {t("central.updateConfirmDesc", { count: states.length })}
          </DialogDescription>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          {states.length > 0 ? (
            <>
              <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-muted/30 px-3 py-2">
                <label className="flex cursor-pointer items-center gap-2 text-sm font-medium">
                  <Checkbox
                    checked={allSelected}
                    onCheckedChange={(value) => toggleAll(!!value)}
                    aria-label={t("central.updateConfirmSelectAll")}
                  />
                  {t("central.updateConfirmSelectAll")}
                </label>
                <span className="text-xs text-muted-foreground">
                  {t("central.updateConfirmSelected", { count: selectedCount })}
                </span>
              </div>

              <div className="max-h-[26rem] space-y-3 overflow-auto pr-1">
                {states.map((state) => {
                  const skill = skillById.get(state.skill_id);
                  const displayName = skill?.name ?? state.skill_id;
                  const checked = selectedSet.has(state.skill_id);
                  const lastChecked = formatDate(state.last_checked_at);
                  return (
                    <section
                      key={state.skill_id}
                      className="rounded-xl border border-border bg-background p-3"
                    >
                      <label className="flex cursor-pointer items-start gap-3">
                        <Checkbox
                          checked={checked}
                          onCheckedChange={(value) => toggleSkill(state.skill_id, !!value)}
                          aria-label={t("central.updateConfirmSkillLabel", {
                            name: displayName,
                          })}
                        />
                        <span className="min-w-0 flex-1 space-y-2">
                          <span className="inline-flex max-w-full items-center rounded-md border border-primary/20 bg-primary/5 px-2 py-0.5 text-sm font-semibold text-primary">
                            <span className="min-w-0 truncate">{displayName}</span>
                          </span>
                          {skill?.description && (
                            <span className="block line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                              {skill.description}
                            </span>
                          )}
                          <span className="block space-y-1 rounded-lg border border-border/70 bg-muted/20 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground">
                            <span className="block break-all">
                              {state.source_path ?? skill?.source_path
                                ? t("central.updateConfirmSourcePath", {
                                    path: state.source_path ?? skill?.source_path,
                                  })
                                : t("central.updateConfirmSourceUnknown")}
                            </span>
                            {skill?.repository?.name && (
                              <span className="block break-words">
                                {t("central.updateConfirmRepository", {
                                  repo: skill.repository.name,
                                })}
                              </span>
                            )}
                            {lastChecked && (
                              <span className="block break-words">
                                {t("central.updateConfirmLastChecked", {
                                  date: lastChecked,
                                })}
                              </span>
                            )}
                          </span>
                        </span>
                      </label>
                    </section>
                  );
                })}
              </div>
            </>
          ) : (
            <div className="rounded-xl border border-border p-4 text-sm text-muted-foreground">
              {t("central.updateConfirmEmpty")}
            </div>
          )}
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApplying}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={selectedCount === 0 || isApplying}
            data-testid="confirm-central-skill-updates"
          >
            {isApplying ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.updateConfirmApplying")}
              </>
            ) : (
              <>
                <Download className="size-3.5" />
                {t("central.updateConfirmApply", { count: selectedCount })}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
