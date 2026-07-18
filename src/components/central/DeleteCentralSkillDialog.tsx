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
import { groupPlatformAgentIds } from "@/lib/platformCleanupGroups";
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

function uniqueAgentIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

function installationPaths(installations: SkillInstallation[]): string {
  return uniqueAgentIds(installations.map((installation) => installation.installed_path)).join(", ");
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
  const copyInstallationGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        copyInstallations.map((installation) => installation.agent_id),
        t("platformTargets.universalLabel")
      ).map((group) => ({
        ...group,
        installations: copyInstallations.filter((installation) =>
          group.agentIds.includes(installation.agent_id)
        ),
      })),
    [agents, copyInstallations, t]
  );
  const autoRemovedGroups = useMemo(
    () =>
      groupPlatformAgentIds(
        agents,
        autoRemovedAgentIds,
        t("platformTargets.universalLabel")
      ),
    [agents, autoRemovedAgentIds, t]
  );

  function handleToggleCopy(agentIds: string[], checked: boolean) {
    setSelectedCopyAgentIds((current) => {
      const next = new Set(current);
      if (checked) {
        agentIds.forEach((agentId) => next.add(agentId));
      } else {
        agentIds.forEach((agentId) => next.delete(agentId));
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
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive-text" />
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
              {autoRemovedGroups.length > 0 && (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.deleteLinkedPlatformInstalls")}
                  </div>
                  <div className="space-y-2">
                    {autoRemovedGroups.map((group) => (
                      <div key={group.id} className="rounded-xl border border-border bg-muted/30 p-3">
                        <div className="text-sm font-medium text-foreground">{group.label}</div>
                        {group.detail && (
                          <div className="mt-1 truncate text-xs text-muted-foreground">
                            {group.detail}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {copyInstallationGroups.length > 0 ? (
                <div className="space-y-2">
                  <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("central.deletePlatformCopies")}
                  </div>
                  <div className="space-y-2">
                    {copyInstallationGroups.map((group) => {
                      const checked = group.agentIds.every((agentId) =>
                        selectedCopyAgentIds.has(agentId)
                      );
                      const paths = installationPaths(group.installations);
                      return (
                        <label
                          key={group.id}
                          className="flex cursor-pointer items-start gap-2 rounded-xl border border-border p-3"
                        >
                          <Checkbox
                            checked={checked}
                            onCheckedChange={(value) => handleToggleCopy(group.agentIds, !!value)}
                            aria-label={t("central.deletePlatformCopyLabel", {
                              platform: group.label,
                              skill: skill.name,
                            })}
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium text-foreground">{group.label}</span>
                            <span className="mt-1 block truncate text-xs text-muted-foreground">
                              {paths || group.detail}
                            </span>
                          </span>
                        </label>
                      );
                    })}
                  </div>
                </div>
              ) : autoRemovedGroups.length === 0 ? (
                <div className="rounded-xl border border-border p-3 text-sm text-muted-foreground">
                  {t("central.deleteNoPlatformCopies")}
                </div>
              ) : null}
            </>
          )}

          {error && (
            <p className="text-xs text-destructive-text" role="alert">
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
