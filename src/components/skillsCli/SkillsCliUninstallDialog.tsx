import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatBackendError } from "@/lib/backendError";
import {
  aggregateRemovalImpact,
  type PlacementMutationOutcome,
  type RemovalImpact,
} from "@/pages/skillsCliBatchModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import type { SkillsCliRemovePlan } from "@/types";

export interface SkillsCliUninstallDialogProps {
  open: boolean;
  skillNames: readonly string[];
  isMutating: boolean;
  runtimeBlocked?: boolean;
  returnFocusRef?: RefObject<HTMLElement | null>;
  onOpenChange: (open: boolean) => void;
  previewRemoveGlobal: (skillName: string) => Promise<SkillsCliRemovePlan | null>;
  removeGlobalBatch: (skillNames: string[]) => Promise<PlacementMutationOutcome>;
  onRemoved: (succeededNames: string[]) => void;
}

export function SkillsCliUninstallDialog({
  open,
  skillNames,
  isMutating,
  runtimeBlocked = false,
  returnFocusRef,
  onOpenChange,
  previewRemoveGlobal,
  removeGlobalBatch,
  onRemoved,
}: SkillsCliUninstallDialogProps) {
  const { t } = useTranslation();
  const titleRef = useRef<HTMLHeadingElement | null>(null);
  const [remainingNames, setRemainingNames] = useState<string[]>([...skillNames]);
  const [impact, setImpact] = useState<RemovalImpact | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [itemErrors, setItemErrors] = useState<Array<{ name: string; error: string }>>(
    [],
  );
  const [isPreviewing, setIsPreviewing] = useState(false);
  const incomingNamesKey = skillNames.join("\0");
  const namesKey = remainingNames.join("\0");

  useEffect(() => {
    if (!open) {
      return;
    }
    setRemainingNames(incomingNamesKey ? incomingNamesKey.split("\0") : []);
    setItemErrors([]);
    setPreviewError(null);
    setImpact(null);
  }, [open, incomingNamesKey]);

  useEffect(() => {
    if (!open || namesKey.length === 0) {
      return;
    }
    const names = namesKey.split("\0");
    let cancelled = false;
    setIsPreviewing(true);
    setPreviewError(null);
    void (async () => {
      try {
        const plans: SkillsCliRemovePlan[] = [];
        for (const name of names) {
          const plan = await previewRemoveGlobal(name);
          if (cancelled) {
            return;
          }
          if (!plan) {
            const latest =
              useSkillsCliStore.getState().actionError ?? "internal.unexpected:";
            setPreviewError(
              t("skillsCli.uninstallImpact.previewError", {
                error: formatBackendError(latest, t),
              }),
            );
            setImpact(null);
            setIsPreviewing(false);
            return;
          }
          plans.push(plan);
        }
        if (cancelled) {
          return;
        }
        setImpact(aggregateRemovalImpact(plans));
        setIsPreviewing(false);
      } catch (error) {
        if (cancelled) {
          return;
        }
        setPreviewError(
          t("skillsCli.uninstallImpact.previewError", {
            error: formatBackendError(error, t),
          }),
        );
        setImpact(null);
        setIsPreviewing(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // Locale translator is read for error text only; including it retriggers
    // preview in jsdom because the test mock returns a new `t` each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- see above
  }, [open, namesKey, previewRemoveGlobal]);

  useEffect(() => {
    if (open) {
      titleRef.current?.focus();
    }
  }, [open, namesKey]);

  const title =
    remainingNames.length === 1
      ? t("skillsCli.uninstallTitle", { name: remainingNames[0] })
      : t("skillsCli.uninstallImpact.titleMany", {
          count: remainingNames.length,
        });
  const confirmDisabled =
    isMutating ||
    runtimeBlocked ||
    isPreviewing ||
    previewError !== null ||
    impact == null ||
    !impact.confirmable ||
    impact.conflicts.length > 0;

  const confirmableNote = useMemo(() => {
    if (!impact) {
      return null;
    }
    if (impact.conflicts.length > 0 || !impact.confirmable) {
      return t("skillsCli.uninstallImpact.confirmBlocked");
    }
    return null;
  }, [impact, t]);

  async function handleConfirm() {
    if (confirmDisabled) {
      return;
    }
    setItemErrors([]);
    setPreviewError(null);
    try {
      const outcome = await removeGlobalBatch(remainingNames);
      const succeeded = outcome.succeeded.map((item) => item.skillName);
      if (succeeded.length > 0) {
        onRemoved(succeeded);
      }
      const failedNames = outcome.failed.map((item) => item.skillName);
      if (failedNames.length === 0) {
        if (succeeded.length === 1) {
          showSkillsCliActionToast({
            semantic: "destructiveSuccess",
            message: t("skillsCli.removeSuccess", { name: succeeded[0] }),
          });
        } else {
          showSkillsCliActionToast({
            semantic: "destructiveSuccess",
            message: t("skillsCli.batch.removeSuccessMany", {
              count: succeeded.length,
            }),
          });
        }
        onOpenChange(false);
        return;
      }
      setRemainingNames(failedNames);
      setItemErrors(
        outcome.failed.map((item) => ({
          name: item.skillName,
          error: formatBackendError(`${item.errorCode}:`, t),
        })),
      );
      showSkillsCliActionToast({
        semantic: "destructiveError",
        message: t("skillsCli.batch.removePartial", {
          succeeded: succeeded.length,
          failed: failedNames.length,
        }),
      });
    } catch (error) {
      const message = t("skillsCli.removeError", {
        error: formatBackendError(error, t),
      });
      setPreviewError(message);
      showSkillsCliActionToast({
        semantic: "destructiveError",
        message,
      });
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (isMutating) {
          return;
        }
        onOpenChange(next);
      }}
    >
      <DialogContent
        className="sm:max-w-md"
        initialFocus={titleRef}
        finalFocus={returnFocusRef}
        aria-busy={isPreviewing || isMutating}
      >
        <DialogHeader>
          <DialogTitle>
            <span ref={titleRef} tabIndex={-1} className="outline-none">
              {title}
            </span>
          </DialogTitle>
          <DialogDescription>{t("skillsCli.uninstallDescription")}</DialogDescription>
        </DialogHeader>
        <DialogBody className="space-y-3">
          <ul className="flex flex-wrap gap-1">
            {remainingNames.map((name) => (
              <li
                key={name}
                className="rounded-full border border-border px-2 py-0.5 text-xs"
              >
                {name}
              </li>
            ))}
          </ul>
          {impact ? (
            <div className="space-y-2 text-sm">
              <p data-testid="skills-cli-uninstall-owned">
                {t("skillsCli.uninstallImpact.ownedContent", {
                  count: impact.ownedContentCount,
                })}
              </p>
              <p data-testid="skills-cli-uninstall-managed">
                {t("skillsCli.uninstallImpact.managedLinks", {
                  count: impact.managedLinkCount,
                })}
              </p>
              {impact.retainedDirectCopies.length > 0 ? (
                <div data-testid="skills-cli-uninstall-retained">
                  <p>{t("skillsCli.uninstallImpact.retainedCopies")}</p>
                  <ul className="mt-1 space-y-0.5 text-muted-foreground">
                    {impact.retainedDirectCopies.map((item) => (
                      <li key={`${item.skillName}:${item.agentId}`}>
                        {t("skillsCli.uninstallImpact.retainedCopyItem", {
                          name: item.skillName,
                          platform: item.displayName,
                        })}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
              {impact.conflicts.length > 0 ? (
                <div
                  role="alert"
                  data-testid="skills-cli-uninstall-conflicts"
                  className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-destructive-text"
                >
                  <p>{t("skillsCli.uninstallImpact.conflicts")}</p>
                  <ul className="mt-1 space-y-0.5">
                    {impact.conflicts.map((item) => (
                      <li key={`${item.skillName}:${item.agentId}`}>
                        {t("skillsCli.uninstallImpact.conflictItem", {
                          name: item.skillName,
                          platform: item.displayName,
                          reason: formatBackendError(`${item.reasonCode}:`, t),
                        })}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </div>
          ) : null}
          {confirmableNote ? (
            <p className="text-sm text-destructive-text">{confirmableNote}</p>
          ) : null}
          {previewError ? (
            <p role="alert" className="text-sm text-destructive-text">
              {previewError}
            </p>
          ) : null}
          {itemErrors.length > 0 ? (
            <ul role="alert" className="space-y-1 text-sm text-destructive-text">
              {itemErrors.map((item) => (
                <li key={item.name}>
                  {t("skillsCli.uninstallImpact.failedItem", {
                    name: item.name,
                    error: item.error,
                  })}
                </li>
              ))}
            </ul>
          ) : null}
        </DialogBody>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            disabled={isMutating}
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={confirmDisabled}
            onClick={() => void handleConfirm()}
          >
            {isMutating
              ? t("skillsCli.uninstalling")
              : t("skillsCli.uninstallConfirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
