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
import { formatBackendError } from "@/lib/backendError";
import { groupPlatformAgentIds } from "@/lib/platformCleanupGroups";
import type {
  AgentWithStatus,
  DeleteCentralSkillPreview,
  SkillInstallation,
  SkillWithLinks,
} from "@/types";

interface DeleteCentralSkillDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skill: SkillWithLinks | null;
  preview: DeleteCentralSkillPreview | null;
  agents: AgentWithStatus[];
  isPreviewLoading: boolean;
  isDeleting: boolean;
  error: string | null;
  onConfirm: (skillId: string, removeAgentIds: string[], force: boolean) => Promise<void>;
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
  preview,
  agents,
  isPreviewLoading,
  isDeleting,
  error,
  onConfirm,
}: DeleteCentralSkillDialogProps) {
  const { t } = useTranslation();
  const [selectedCopyAgentIds, setSelectedCopyAgentIds] = useState<Set<string>>(new Set());
  const [forceArmed, setForceArmed] = useState(false);

  useEffect(() => {
    if (open) {
      setSelectedCopyAgentIds(new Set());
      setForceArmed(false);
    }
  }, [open, skill?.id]);

  const copyInstallations = useMemo(
    () => preview?.copy_installations ?? [],
    [preview?.copy_installations]
  );
  const autoRemovedAgentIds = useMemo(
    () => preview?.auto_removed_agent_ids ?? [],
    [preview?.auto_removed_agent_ids]
  );
  const pendingRecovery = preview?.pending_recovery ?? null;
  const forceDeleteEligible = Boolean(pendingRecovery?.force_delete_eligible);
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

  async function handleConfirm(force: boolean) {
    if (!skill) return;
    await onConfirm(skill.id, Array.from(selectedCopyAgentIds), force);
  }

  async function handleForceClick() {
    if (!forceArmed) {
      setForceArmed(true);
      return;
    }
    await handleConfirm(true);
  }

  if (!skill) return null;

  const recoveryMessage = pendingRecovery
    ? formatBackendError(
        `${pendingRecovery.error_code || "central_operation.delete_restore_collision"}:`,
        t
      )
    : null;

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
                  {preview?.central_path ?? skill.canonical_path ?? skill.file_path}
                </div>
              </div>
            </div>
          </div>

          {pendingRecovery && recoveryMessage && (
            <div className="rounded-xl border border-warning/30 bg-warning/10 p-3 text-sm text-warning-foreground">
              <div className="font-medium">{recoveryMessage}</div>
              {forceDeleteEligible ? (
                forceArmed && (
                  <div className="mt-2 text-xs">{t("central.forceDeleteHint")}</div>
                )
              ) : (
                <div className="mt-2 text-xs">{t("central.forceDeleteBlocked")}</div>
              )}
            </div>
          )}

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
          {forceDeleteEligible && (
            <Button
              variant="destructive"
              onClick={handleForceClick}
              disabled={isPreviewLoading || isDeleting || !preview}
              data-testid="force-delete-central-skill"
            >
              {isDeleting ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  {t("central.deletingSkill")}
                </>
              ) : (
                <>
                  <Trash2 className="size-3.5" />
                  {t("central.forceDeleteSkill")}
                </>
              )}
            </Button>
          )}
          <Button variant="destructive" onClick={() => handleConfirm(false)} disabled={isPreviewLoading || isDeleting || !preview}>
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
