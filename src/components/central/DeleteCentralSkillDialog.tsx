import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Loader2, Trash2 } from "lucide-react";
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
import type { AgentWithStatus, SkillDetail, SkillInstallation, SkillWithLinks } from "@/types";

interface DeleteCentralSkillDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skill: SkillWithLinks | null;
  detail: SkillDetail | null;
  agents: AgentWithStatus[];
  isPreviewLoading: boolean;
  isDeleting: boolean;
  error: string | null;
  onConfirm: (skillId: string, removeAgentIds: string[]) => Promise<void>;
}

function agentName(agents: AgentWithStatus[], agentId: string): string {
  return agents.find((agent) => agent.id === agentId)?.display_name ?? agentId;
}

function uniqueAgentIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

export function DeleteCentralSkillDialog({
  open,
  onOpenChange,
  skill,
  detail,
  agents,
  isPreviewLoading,
  isDeleting,
  error,
  onConfirm,
}: DeleteCentralSkillDialogProps) {
  const { t } = useTranslation();
  const [selectedCopyAgentIds, setSelectedCopyAgentIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (open) {
      setSelectedCopyAgentIds(new Set());
    }
  }, [open, skill?.id]);

  const copyInstallations = useMemo(
    () => (detail?.installations ?? []).filter((item) => item.link_type === "copy"),
    [detail?.installations]
  );
  const autoRemovedAgentIds = useMemo(() => {
    const linkedAgentIds = (detail?.installations ?? [])
      .filter((item) => item.link_type !== "copy")
      .map((item) => item.agent_id);
    return uniqueAgentIds([...linkedAgentIds, ...(skill?.shared_root_agents ?? [])]);
  }, [detail?.installations, skill?.shared_root_agents]);

  function handleToggleCopy(agentId: string, checked: boolean) {
    setSelectedCopyAgentIds((current) => {
      const next = new Set(current);
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
    await onConfirm(skill.id, Array.from(selectedCopyAgentIds));
  }

  if (!skill) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("central.deleteDialogTitle", { name: skill.name })}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("central.deleteDialogDesc")}
          </DialogDescription>

          <div className="rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-sm">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
              <div className="min-w-0">
                <div className="font-medium text-foreground">{t("central.deleteCentralRequired")}</div>
                <div className="mt-1 truncate text-xs text-muted-foreground">
                  {skill.canonical_path ?? skill.file_path}
                </div>
              </div>
            </div>
          </div>

          {isPreviewLoading ? (
            <div className="flex items-center gap-2 rounded-xl border border-border p-3 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              {t("central.deletePreviewLoading")}
            </div>
          ) : (
            <>
              {copyInstallations.length > 0 ? (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.deletePlatformCopies")}
                  </div>
                  <div className="space-y-2">
                    {copyInstallations.map((installation: SkillInstallation) => {
                      const checked = selectedCopyAgentIds.has(installation.agent_id);
                      const name = agentName(agents, installation.agent_id);
                      return (
                        <label
                          key={installation.agent_id}
                          className="flex cursor-pointer items-start gap-2 rounded-xl border border-border p-3"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(value) => handleToggleCopy(installation.agent_id, !!value)}
                            aria-label={t("central.deletePlatformCopyLabel", {
                              platform: name,
                              skill: skill.name,
                            })}
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium text-foreground">{name}</span>
                            <span className="mt-1 block truncate text-xs text-muted-foreground">
                              {installation.installed_path}
                            </span>
                          </span>
                        </label>
                      );
                    })}
                  </div>
                </div>
              ) : (
                <div className="rounded-xl border border-border p-3 text-sm text-muted-foreground">
                  {t("central.deleteNoPlatformCopies")}
                </div>
              )}

              {autoRemovedAgentIds.length > 0 && (
                <div className="rounded-xl border border-border bg-muted/30 p-3 text-xs text-muted-foreground">
                  {t("central.deleteAutoCleanup", {
                    platforms: autoRemovedAgentIds.map((agentId) => agentName(agents, agentId)).join(", "),
                  })}
                </div>
              )}
            </>
          )}

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isDeleting}>
            {t("common.cancel")}
          </Button>
          <Button variant="destructive" onClick={handleConfirm} disabled={isPreviewLoading || isDeleting || !detail}>
            {isDeleting ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.deletingSkill")}
              </>
            ) : (
              <>
                <Trash2 className="size-3.5" />
                {t("central.confirmDeleteSkill")}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
