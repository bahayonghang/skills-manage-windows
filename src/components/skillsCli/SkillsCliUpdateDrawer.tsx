import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import type { TFunction } from "i18next";
import { XIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { formatBackendError } from "@/lib/backendError";
import { cn } from "@/lib/utils";
import { skillsCliDrawerPanelWidth } from "@/pages/skillsCliDetailModel";
import {
  applySelectionsForNames,
  argvPreviewForSelection,
  buildUpdateDrawerRows,
  shortRevisionIdentity,
  updateRowForSkill,
  type SkillsCliUpdateDrawerRow,
} from "@/pages/skillsCliViewModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliUpdateInventory,
  SkillsCliUpdateJobPhase,
  SkillsCliUpdateProgress,
  SkillsCliUpdateStatus,
} from "@/types";

const ICON_HIT_AREA =
  "relative after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

export interface SkillsCliUpdateDrawerProps {
  open: boolean;
  repositoryKey: string;
  skillNames: readonly string[];
  skills: readonly SkillsCliGlobalSkill[];
  inventory: SkillsCliUpdateInventory | null;
  contentWidth: number | null;
  updateError: string | null;
  updateJobPhase: SkillsCliUpdateJobPhase;
  updateProgress: SkillsCliUpdateProgress | null;
  returnFocusRef?: RefObject<HTMLElement | null>;
  onClose: () => void;
  onApply: (input: {
    repositoryKey: string;
    skillNames: string[];
  }) => Promise<unknown>;
  onVerifyBaseline: (skillNames: string[]) => Promise<unknown>;
  onRetryRecovery?: (operationId: string) => Promise<unknown>;
}

function statusLabel(status: SkillsCliUpdateStatus, t: TFunction): string {
  switch (status) {
    case "not_checked":
      return t("skillsCli.updates.status.not_checked");
    case "checking":
      return t("skillsCli.updates.status.checking");
    case "current":
      return t("skillsCli.updates.status.current");
    case "update_available":
      return t("skillsCli.updates.status.update_available");
    case "local_modified":
      return t("skillsCli.updates.status.local_modified");
    case "baseline_required":
      return t("skillsCli.updates.status.baseline_required");
    case "unsupported":
      return t("skillsCli.updates.status.unsupported");
    case "rate_limited":
      return t("skillsCli.updates.status.rate_limited");
    case "failed":
      return t("skillsCli.updates.status.failed");
    default: {
      const _exhaustive: never = status;
      return _exhaustive;
    }
  }
}

export function SkillsCliUpdateDrawer({
  open,
  repositoryKey,
  skillNames,
  skills,
  inventory,
  contentWidth,
  updateError,
  updateJobPhase,
  updateProgress,
  returnFocusRef,
  onClose,
  onApply,
  onVerifyBaseline,
  onRetryRecovery,
}: SkillsCliUpdateDrawerProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const titleRef = useRef<HTMLHeadingElement>(null);
  const [selectedNames, setSelectedNames] = useState<ReadonlySet<string>>(
    () => new Set(skillNames),
  );
  const [inlineError, setInlineError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setSelectedNames(new Set(skillNames));
      setInlineError(null);
    }
  }, [open, skillNames, repositoryKey]);

  useEffect(() => {
    if (open) {
      return;
    }
    returnFocusRef?.current?.focus?.();
  }, [open, returnFocusRef]);

  const hasRecovery = inventory?.pendingRecovery != null;
  const rows = useMemo(
    () =>
      buildUpdateDrawerRows(
        skills,
        inventory,
        repositoryKey,
        selectedNames,
        hasRecovery,
      ),
    [skills, inventory, repositoryKey, selectedNames, hasRecovery],
  );
  const selectedRows = rows.filter((row) => row.selected);
  const actionableNames = selectedRows
    .filter((row) => row.applyEnabled)
    .map((row) => row.skillName);
  const reinstallNames = selectedRows
    .filter((row) => row.reinstallEnabled)
    .map((row) => row.skillName);
  const preview = argvPreviewForSelection(
    inventory,
    actionableNames.length > 0 ? actionableNames : reinstallNames,
  );
  const applying = updateJobPhase === "applying";
  const verifying = updateJobPhase === "verifying";
  const busy = updateJobPhase != null;
  const width = skillsCliDrawerPanelWidth(contentWidth);
  const repoRow = inventory?.repositories.find(
    (row) => row.repositoryKey === repositoryKey,
  );
  const needsVerify = selectedRows.some((row) => row.status === "baseline_required");
  const hasStale = selectedRows.some((row) => {
    const cached = updateRowForSkill(inventory, row.skillName);
    return Boolean(cached?.isStale);
  });
  const hasLocalModified = selectedRows.some(
    (row) => row.status === "local_modified",
  );
  const hasUnsupported = selectedRows.some((row) => row.status === "unsupported");

  function toggleName(name: string, checked: boolean) {
    setSelectedNames((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(name);
      } else {
        next.delete(name);
      }
      return next;
    });
  }

  async function runApply() {
    setInlineError(null);
    if (actionableNames.length === 0) {
      setInlineError(t("skillsCli.updates.noActionable"));
      return;
    }
    if (applySelectionsForNames(inventory, actionableNames).length === 0) {
      setInlineError(t("skillsCli.updates.checkFirst"));
      return;
    }
    try {
      await onApply({ repositoryKey, skillNames: actionableNames });
      showSkillsCliActionToast({
        semantic: "success",
        message: t("skillsCli.updates.applySuccess", {
          count: actionableNames.length,
        }),
      });
      onClose();
    } catch (error) {
      const message = formatBackendError(error, t);
      setInlineError(t("skillsCli.updates.applyError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    }
  }

  async function runReinstall() {
    setInlineError(null);
    if (reinstallNames.length === 0) {
      setInlineError(t("skillsCli.updates.noActionable"));
      return;
    }
    if (applySelectionsForNames(inventory, reinstallNames).length === 0) {
      setInlineError(t("skillsCli.updates.checkFirst"));
      return;
    }
    try {
      await onApply({ repositoryKey, skillNames: reinstallNames });
      showSkillsCliActionToast({
        semantic: "success",
        message: t("skillsCli.updates.applySuccess", {
          count: reinstallNames.length,
        }),
      });
      onClose();
    } catch (error) {
      const message = formatBackendError(error, t);
      setInlineError(t("skillsCli.updates.applyError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    }
  }

  async function runVerify() {
    setInlineError(null);
    try {
      await onVerifyBaseline(selectedRows.map((row) => row.skillName));
    } catch (error) {
      const message = formatBackendError(error, t);
      setInlineError(t("skillsCli.updates.applyError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    }
  }

  async function runRecovery() {
    const operationId = inventory?.pendingRecovery?.operationId;
    if (!operationId || !onRetryRecovery) {
      return;
    }
    setInlineError(null);
    try {
      await onRetryRecovery(operationId);
    } catch (error) {
      const message = formatBackendError(error, t);
      setInlineError(t("skillsCli.updates.applyError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          setInlineError(null);
          onClose();
        }
      }}
    >
      <DialogPortal keepMounted={false}>
        <DialogOverlay
          data-testid="skills-cli-update-overlay"
          className="bg-foreground/30"
        />
        <DialogPrimitive.Popup
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-busy={busy}
          data-testid="skills-cli-update-drawer"
          data-width-mode={width.mode}
          initialFocus={titleRef}
          finalFocus={returnFocusRef}
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex h-full max-w-full flex-col bg-background shadow-2xl ring-1 ring-border outline-none",
            "will-change-transform",
            "data-[starting-style]:animate-in data-[starting-style]:slide-in-from-right",
            "data-[ending-style]:animate-out data-[ending-style]:slide-out-to-right",
            "animation-duration-150",
          )}
          style={{ width: width.cssWidth }}
        >
          <div className="flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border px-3">
            <h2
              id={titleId}
              ref={titleRef}
              tabIndex={-1}
              className="min-w-0 truncate text-sm font-semibold outline-none"
            >
              {t("skillsCli.updates.drawerTitle", { repository: repositoryKey })}
            </h2>
            <DialogClose
              render={
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("common.close")}
                  data-testid="skills-cli-update-close"
                  className={ICON_HIT_AREA}
                />
              }
            >
              <XIcon />
            </DialogClose>
          </div>

          <div className="flex min-h-0 flex-1 flex-col">
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
              <p className="text-sm text-muted-foreground">
                {t("skillsCli.updates.selectedCount", {
                  count: selectedRows.length,
                })}
              </p>
              {hasRecovery ? (
                <Warning
                  testId="skills-cli-update-recovery"
                  text={t("skillsCli.updates.recoveryRequired")}
                />
              ) : null}
              {hasStale ? (
                <Warning
                  testId="skills-cli-update-stale"
                  text={t("skillsCli.updates.stalePending")}
                />
              ) : null}
              {hasLocalModified ? (
                <Warning
                  testId="skills-cli-update-local-modified"
                  text={t("skillsCli.updates.localModified")}
                />
              ) : null}
              {needsVerify ? (
                <Warning
                  testId="skills-cli-update-baseline"
                  text={t("skillsCli.updates.baselineRequired")}
                />
              ) : null}
              {needsVerify ? (
                <p className="text-xs text-muted-foreground">
                  {t("skillsCli.updates.reinstallHint")}
                </p>
              ) : null}
              {reinstallNames.length > 0 ? (
                <Warning
                  testId="skills-cli-update-reinstall-warning"
                  text={t("skillsCli.updates.reinstallOverwrite")}
                />
              ) : null}
              {hasUnsupported ? (
                <Warning
                  testId="skills-cli-update-unsupported"
                  text={t("skillsCli.updates.unsupported")}
                />
              ) : null}
              {repoRow?.rateLimitResetAt ? (
                <Warning
                  testId="skills-cli-update-rate"
                  text={t("skillsCli.updates.rateReset", {
                    time: repoRow.rateLimitResetAt,
                  })}
                />
              ) : null}
              {repoRow?.lastErrorCode ? (
                <Warning
                  testId="skills-cli-update-repo-failed"
                  text={t("skillsCli.updates.repositoryFailed")}
                />
              ) : null}

              <ul className="space-y-2" data-testid="skills-cli-update-rows">
                {rows.map((row) => (
                  <UpdateRow
                    key={row.skillName}
                    row={row}
                    t={t}
                    disabled={busy}
                    onToggle={toggleName}
                  />
                ))}
              </ul>

              <section className="space-y-1">
                <h3 className="text-xs font-medium">
                  {t("skillsCli.updates.commandPreview")}
                </h3>
                <pre
                  data-testid="skills-cli-update-argv"
                  className="overflow-x-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-ui-meta"
                >
                  {preview.join(" ")}
                </pre>
              </section>

              {updateProgress ? (
                <p
                  data-testid="skills-cli-update-progress"
                  className="text-xs text-muted-foreground"
                >
                  {t("skillsCli.updates.progress", {
                    phase: updateProgress.phase,
                    completed: updateProgress.selectedCompleted,
                    total: updateProgress.selectedTotal,
                  })}
                </p>
              ) : null}

              {inlineError || updateError ? (
                <p
                  role="alert"
                  data-testid="skills-cli-update-error"
                  className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-sm text-destructive-text"
                >
                  {inlineError ??
                    formatBackendError(updateError ?? "", t)}
                </p>
              ) : null}
            </div>

            <div className="flex shrink-0 flex-wrap gap-2 border-t border-border p-3">
              {hasRecovery && onRetryRecovery ? (
                <Button
                  type="button"
                  variant="outline"
                  data-testid="skills-cli-update-retry-recovery"
                  disabled={busy}
                  onClick={() => void runRecovery()}
                >
                  {t("skillsCli.updates.retryRecovery")}
                </Button>
              ) : null}
              {needsVerify ? (
                <Button
                  type="button"
                  variant="outline"
                  data-testid="skills-cli-update-verify"
                  disabled={busy || selectedRows.length === 0}
                  onClick={() => void runVerify()}
                >
                  {verifying
                    ? t("skillsCli.updates.checking")
                    : t("skillsCli.updates.verifyBaseline")}
                </Button>
              ) : null}
              {reinstallNames.length > 0 ? (
                <Button
                  type="button"
                  variant="outline"
                  data-testid="skills-cli-update-reinstall"
                  disabled={busy}
                  onClick={() => void runReinstall()}
                >
                  {applying
                    ? t("skillsCli.updates.updating")
                    : t("skillsCli.updates.reinstallCurrent")}
                </Button>
              ) : null}
              <Button
                type="button"
                data-testid="skills-cli-update-apply"
                disabled={busy || actionableNames.length === 0}
                onClick={() => void runApply()}
              >
                {applying
                  ? t("skillsCli.updates.updating")
                  : t("skillsCli.updates.updateSelected")}
              </Button>
            </div>
          </div>
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}

function Warning({ testId, text }: { testId: string; text: string }) {
  return (
    <p
      data-testid={testId}
      className="rounded-md border border-warning/30 bg-warning/10 p-2 text-sm"
    >
      {text}
    </p>
  );
}

function UpdateRow({
  row,
  t,
  disabled,
  onToggle,
}: {
  row: SkillsCliUpdateDrawerRow;
  t: TFunction;
  disabled: boolean;
  onToggle: (name: string, checked: boolean) => void;
}) {
  const installed = shortRevisionIdentity(row.installedRevision) || "—";
  const observed = shortRevisionIdentity(row.observedRevision) || "—";
  return (
    <li
      data-testid={`skills-cli-update-row-${row.skillName}`}
      data-status={row.status}
      className="flex items-start gap-3 rounded-lg border border-border p-3"
    >
      <Checkbox
        checked={row.selected}
        disabled={disabled}
        aria-label={row.skillName}
        onCheckedChange={(checked) => onToggle(row.skillName, checked === true)}
      />
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex flex-wrap items-center gap-2">
          <p className="truncate text-sm font-medium">{row.skillName}</p>
          <span className="text-ui-meta text-muted-foreground">
            {statusLabel(row.status, t)}
          </span>
        </div>
        <p className="font-mono text-ui-meta text-muted-foreground">
          {t("skillsCli.updates.installedRevision", { sha: installed })}
          {" → "}
          {t("skillsCli.updates.observedRevision", { sha: observed })}
        </p>
        <p className="text-xs text-muted-foreground">
          {row.changeSummary.length > 0
            ? row.changeSummary.join(", ")
            : t("skillsCli.updates.emptySummary")}
        </p>
        {!row.applyEnabled && row.blockerCodes[0] ? (
          <p className="text-xs text-destructive-text">
            {formatBackendError(`${row.blockerCodes[0]}:`, t)}
          </p>
        ) : null}
      </div>
    </li>
  );
}
