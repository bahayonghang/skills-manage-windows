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
import {
  AlertTriangle,
  Ban,
  Copy,
  FolderOpen,
  Trash2,
  XIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { formatRelativeTime } from "@/components/logs/logsUtils";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { formatBackendError } from "@/lib/backendError";
import { statusChipClass } from "@/lib/statusTone";
import { cn } from "@/lib/utils";
import {
  buildSkillsCliDetailRows,
  folderHashPrefix,
  skillLocalTimestamp,
  skillsCliDrawerPanelWidth,
  summarizeDetailPlacements,
  visibleSkillDocState,
  type SkillsCliDetailPlacementRow,
  type SkillsCliDocState,
} from "@/pages/skillsCliDetailModel";
import type { SkillsCliGlobalSkill, SkillsCliInstallTarget } from "@/types";

const ICON_HIT_AREA =
  "relative after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

export interface SkillsCliDetailDrawerProps {
  skill: SkillsCliGlobalSkill | null;
  targets: readonly SkillsCliInstallTarget[];
  contentWidth: number | null;
  docState: SkillsCliDocState;
  updateAvailable: boolean;
  focusSection: "links" | null;
  busyPlacements?: ReadonlySet<string>;
  isMutating?: boolean;
  nowMs?: number;
  returnFocusRef?: RefObject<HTMLElement | null>;
  open?: boolean;
  onClose: () => void;
  onFocusConsumed?: () => void;
  onToggleLink: (agentId: string, next: boolean) => void | Promise<void>;
  onLinkAll: () => void | Promise<void>;
  onUnlinkAll: () => void | Promise<void>;
  onRetryDoc: () => void;
  onUpdate?: () => void;
  onRevealFolder: () => void | Promise<void>;
  onUninstall: () => void;
  mutationLockReason?: string;
}

function rowReason(
  row: SkillsCliDetailPlacementRow,
  t: TFunction,
): string | null {
  if (!row.switchDisabled) {
    return null;
  }
  if (row.reasonCode) {
    return formatBackendError(`${row.reasonCode}:`, t);
  }
  switch (row.reasonKind) {
    case "direct_copy":
      return t("skillsCli.detail.reasonDirectCopy");
    case "conflict":
      return t("skillsCli.detail.reasonConflict");
    case "unavailable":
      return t("skillsCli.detail.reasonUnavailable");
    case null:
      return null;
    default: {
      const _exhaustive: never = row.reasonKind;
      return _exhaustive;
    }
  }
}

function ReasonIcon({ kind }: { kind: SkillsCliDetailPlacementRow["reasonKind"] }) {
  switch (kind) {
    case "direct_copy":
      return <Copy className="size-3.5 shrink-0" aria-hidden />;
    case "conflict":
      return <AlertTriangle className="size-3.5 shrink-0" aria-hidden />;
    case "unavailable":
      return <Ban className="size-3.5 shrink-0" aria-hidden />;
    case null:
      return null;
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

export function SkillsCliDetailDrawer({
  skill,
  targets,
  contentWidth,
  docState,
  updateAvailable,
  focusSection,
  busyPlacements,
  isMutating = false,
  nowMs,
  returnFocusRef,
  open: openProp,
  onClose,
  onFocusConsumed,
  onToggleLink,
  onLinkAll,
  onUnlinkAll,
  onRetryDoc,
  onUpdate,
  onRevealFolder,
  onUninstall,
  mutationLockReason,
}: SkillsCliDetailDrawerProps) {
  const { t, i18n } = useTranslation();
  const titleId = useId();
  const pathId = useId();
  const titleRef = useRef<HTMLHeadingElement | null>(null);
  const linksHeadingRef = useRef<HTMLHeadingElement | null>(null);
  const [localBusy, setLocalBusy] = useState<ReadonlySet<string>>(() => new Set());
  const [aggregateBusy, setAggregateBusy] = useState(false);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [revealError, setRevealError] = useState<string | null>(null);

  const open = openProp ?? skill !== null;
  const width = skillsCliDrawerPanelWidth(contentWidth);
  const rows = useMemo(
    () => (skill ? buildSkillsCliDetailRows(skill, targets) : []),
    [skill, targets],
  );
  const summary = useMemo(
    () => summarizeDetailPlacements(rows, targets),
    [rows, targets],
  );
  const visibleDoc = useMemo(
    () => visibleSkillDocState(skill?.name, docState),
    [skill?.name, docState],
  );
  const busy = useMemo(() => {
    const next = new Set(busyPlacements ?? []);
    for (const id of localBusy) {
      next.add(id);
    }
    return next;
  }, [busyPlacements, localBusy]);

  useEffect(() => {
    if (open) {
      return;
    }
    returnFocusRef?.current?.focus?.();
  }, [open, returnFocusRef]);

  useEffect(() => {
    setMutationError(null);
    setRevealError(null);
    setLocalBusy(new Set());
    setAggregateBusy(false);
  }, [skill?.name]);

  useEffect(() => {
    if (!open || focusSection !== "links") {
      return;
    }
    let cancelled = false;
    let attempts = 0;
    let frame = 0;
    const tryScroll = () => {
      if (cancelled) {
        return;
      }
      const node = linksHeadingRef.current;
      if (node) {
        node.scrollIntoView({ block: "start" });
        onFocusConsumed?.();
        return;
      }
      attempts += 1;
      if (attempts < 30) {
        frame = window.requestAnimationFrame(tryScroll);
      }
    };
    frame = window.requestAnimationFrame(tryScroll);
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(frame);
    };
  }, [open, focusSection, skill?.name, onFocusConsumed]);

  const canonicalPath = skill?.canonicalPath ?? skill?.path ?? "";
  const sourceValue = skill?.source?.trim() || "";
  const hashValue = folderHashPrefix(skill?.folderHash);
  const timeValue = skill ? skillLocalTimestamp(skill) : null;
  const relativeTime =
    timeValue != null
      ? formatRelativeTime(timeValue, i18n.language, nowMs ?? Date.now())
      : null;
  const showUpdateButton = updateAvailable && onUpdate != null;
  const actionsLocked = isMutating || aggregateBusy || Boolean(mutationLockReason);
  const docBusy = visibleDoc.status === "loading";

  function markBusy(ids: readonly string[], next: boolean) {
    setLocalBusy((current) => {
      const copy = new Set(current);
      for (const id of ids) {
        if (next) {
          copy.add(id);
        } else {
          copy.delete(id);
        }
      }
      return copy;
    });
  }

  async function runToggle(agentId: string, next: boolean) {
    setMutationError(null);
    markBusy([agentId], true);
    try {
      await onToggleLink(agentId, next);
      showSkillsCliActionToast({
        semantic: "success",
        message: t(
          next ? "skillsCli.batch.linkSuccess" : "skillsCli.batch.unlinkSuccess",
          { succeeded: 1 },
        ),
      });
    } catch (error) {
      const message = formatBackendError(error, t);
      setMutationError(t("skillsCli.detail.mutationError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    } finally {
      markBusy([agentId], false);
    }
  }

  async function runAggregate(kind: "linkAll" | "unlinkAll") {
    setMutationError(null);
    setAggregateBusy(true);
    const ids =
      kind === "linkAll" ? summary.missingAgentIds : summary.managedLinkAgentIds;
    markBusy(ids, true);
    try {
      await (kind === "linkAll" ? onLinkAll() : onUnlinkAll());
    } catch (error) {
      const message = formatBackendError(error, t);
      setMutationError(t("skillsCli.detail.mutationError", { error: message }));
    } finally {
      markBusy(ids, false);
      setAggregateBusy(false);
    }
  }

  async function runReveal() {
    if (!skill) {
      return;
    }
    setRevealError(null);
    try {
      await onRevealFolder();
    } catch (error) {
      const message = formatBackendError(error, t);
      setRevealError(t("skillsCli.detail.revealError", { error: message }));
      showSkillsCliActionToast({ semantic: "error", message });
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          onClose();
        }
      }}
    >
      <DialogPortal keepMounted={false}>
        <DialogOverlay
          data-testid="skills-cli-detail-overlay"
          className="bg-foreground/30"
        />
        <DialogPrimitive.Popup
          role="dialog"
          aria-modal="true"
          aria-labelledby={skill ? titleId : undefined}
          aria-describedby={skill && canonicalPath ? pathId : undefined}
          aria-busy={docBusy || aggregateBusy || isMutating}
          data-testid="skills-cli-detail-drawer"
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
              {skill?.name ?? t("skillsCli.title")}
            </h2>
            <DialogClose
              render={
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("common.close")}
                  data-testid="skills-cli-detail-close"
                  className={ICON_HIT_AREA}
                />
              }
            >
              <XIcon />
            </DialogClose>
          </div>

          {skill ? (
            <div className="flex min-h-0 flex-1 flex-col">
              <div className="min-h-0 flex-1 space-y-5 overflow-y-auto p-4">
                <header className="space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="text-base font-semibold tracking-[-0.01em]">
                      {skill.name}
                    </p>
                    {updateAvailable ? (
                      <span
                        className={cn(
                          "rounded-full border px-1.5 py-0.5 text-xs font-medium",
                          statusChipClass.warning,
                        )}
                      >
                        {t("skillsCli.detail.updateBadge")}
                      </span>
                    ) : null}
                  </div>
                  {canonicalPath ? (
                    <p
                      id={pathId}
                      title={canonicalPath}
                      className="truncate font-mono text-ui-meta text-muted-foreground"
                    >
                      <span className="sr-only">
                        {t("skillsCli.detail.canonicalPath")}:{" "}
                      </span>
                      {canonicalPath}
                    </p>
                  ) : null}
                  <div
                    data-testid="skills-cli-detail-pills"
                    className="flex flex-wrap gap-1.5"
                  >
                    {sourceValue ? (
                      <span
                        title={sourceValue}
                        className="max-w-full truncate rounded-full border border-border px-2 py-0.5 text-xs"
                      >
                        {sourceValue}
                      </span>
                    ) : null}
                    {hashValue ? (
                      <span
                        title={t("skillsCli.detail.hashTooltip")}
                        className="rounded-full border border-border px-2 py-0.5 font-mono text-xs"
                      >
                        {hashValue}
                      </span>
                    ) : null}
                    {relativeTime && timeValue ? (
                      <span
                        title={timeValue}
                        className="rounded-full border border-border px-2 py-0.5 text-xs"
                      >
                        {relativeTime}
                      </span>
                    ) : null}
                  </div>
                </header>

                <section
                  data-testid="skills-cli-detail-links"
                  className="space-y-3"
                  aria-busy={aggregateBusy}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <h3
                        ref={linksHeadingRef}
                        className="text-xs font-medium text-foreground"
                      >
                        {t("skillsCli.detail.linksHeading")}
                      </h3>
                      <p className="mt-0.5 text-ui-meta text-muted-foreground">
                        {t("skillsCli.detail.linksSummary", {
                          associated: summary.associatedCount,
                          total: summary.enabledCount,
                        })}
                      </p>
                    </div>
                    {summary.aggregate === "linkAll" ? (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        data-testid="skills-cli-detail-link-all"
                        disabled={actionsLocked}
                        title={mutationLockReason}
                        onClick={() => void runAggregate("linkAll")}
                      >
                        {t("skillsCli.detail.linkAll")}
                      </Button>
                    ) : (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        data-testid="skills-cli-detail-unlink-all"
                        disabled={
                          actionsLocked || summary.aggregate === "disabled"
                        }
                        title={
                          mutationLockReason ??
                          (summary.aggregate === "disabled"
                            ? t("skillsCli.detail.aggregateDisabled")
                            : undefined)
                        }
                        onClick={() => void runAggregate("unlinkAll")}
                      >
                        {t("skillsCli.detail.unlinkAll")}
                      </Button>
                    )}
                  </div>
                  {mutationError ? (
                    <p
                      role="alert"
                      data-testid="skills-cli-detail-mutation-error"
                      className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-sm text-destructive-text"
                    >
                      {mutationError}
                    </p>
                  ) : null}
                  <ul className="space-y-2">
                    {rows.map((row) => {
                      const reason = rowReason(row, t);
                      const rowBusy = busy.has(row.agentId);
                      const switchDisabled =
                        row.switchDisabled || actionsLocked || rowBusy;
                      const switchLabel = switchDisabled
                        ? t("skillsCli.detail.switchDisabled", {
                            platform: row.displayName,
                            reason: mutationLockReason ?? reason ?? row.displayName,
                          })
                        : t(
                            row.action === "unlink"
                              ? "skillsCli.detail.switchUnlink"
                              : "skillsCli.detail.switchLink",
                            { name: skill.name, platform: row.displayName },
                          );
                      return (
                        <li
                          key={row.agentId}
                          data-testid={`skills-cli-placement-${row.agentId}`}
                          data-state={row.state}
                          aria-busy={rowBusy}
                          className="flex items-start gap-3 rounded-lg border border-border bg-muted/20 p-3"
                        >
                          <PlatformIcon
                            agentId={row.agentId}
                            size={16}
                            className="mt-0.5 shrink-0"
                          />
                          <div className="min-w-0 flex-1">
                            <p className="truncate text-sm font-medium">
                              {row.displayName}
                            </p>
                            <p
                              title={row.targetPath}
                              className="truncate font-mono text-ui-meta text-muted-foreground"
                            >
                              {row.targetPath}
                            </p>
                            {reason ? (
                              <p className="mt-1 flex items-start gap-1 text-xs text-muted-foreground">
                                <ReasonIcon kind={row.reasonKind} />
                                <span>{reason}</span>
                              </p>
                            ) : null}
                          </div>
                          <Switch
                            checked={row.switchChecked}
                            disabled={switchDisabled}
                            aria-label={switchLabel}
                            onCheckedChange={(checked) => {
                              if (row.action == null) {
                                return;
                              }
                              void runToggle(row.agentId, checked);
                            }}
                          />
                        </li>
                      );
                    })}
                  </ul>
                </section>

                <section data-testid="skills-cli-detail-doc" className="space-y-2">
                  <div className="flex items-baseline justify-between gap-2">
                    <h3 className="text-xs font-medium">
                      {t("skillsCli.detail.docHeading")}
                    </h3>
                    {visibleDoc.status === "ready" || visibleDoc.status === "empty" ? (
                      <span className="text-ui-meta text-muted-foreground">
                        {t("skillsCli.detail.docBytes", {
                          bytes: visibleDoc.byteSize,
                        })}
                      </span>
                    ) : null}
                  </div>
                  {visibleDoc.status === "loading" ? (
                    <div
                      data-testid="skills-cli-detail-doc-skeleton"
                      aria-busy="true"
                      aria-label={t("skillsCli.detail.docLoading")}
                      className="space-y-2"
                    >
                      <div className="h-3 animate-pulse rounded bg-muted/80" />
                      <div className="h-3 w-5/6 animate-pulse rounded bg-muted/80" />
                      <div className="h-3 w-2/3 animate-pulse rounded bg-muted/80" />
                    </div>
                  ) : null}
                  {visibleDoc.status === "empty" ? (
                    <p
                      data-testid="skills-cli-detail-doc-empty"
                      className="text-sm text-muted-foreground"
                    >
                      {t("skillsCli.detail.docEmpty")}
                    </p>
                  ) : null}
                  {visibleDoc.status === "ready" ? (
                    <pre
                      data-testid="skills-cli-detail-doc-pre"
                      className="max-h-[40vh] overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-muted/30 p-3 font-mono text-ui-meta leading-relaxed"
                    >
                      {visibleDoc.content}
                    </pre>
                  ) : null}
                  {visibleDoc.status === "error" ? (
                    <div
                      role="alert"
                      data-testid="skills-cli-detail-doc-error"
                      className="space-y-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive-text"
                    >
                      <p>
                        {t("skillsCli.detail.docError", {
                          error: formatBackendError(
                            `${visibleDoc.errorCode}:`,
                            t,
                          ),
                        })}
                      </p>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={onRetryDoc}
                      >
                        {t("common.retry")}
                      </Button>
                    </div>
                  ) : null}
                </section>
              </div>

              <div className="flex shrink-0 flex-wrap gap-2 border-t border-border p-3">
                {showUpdateButton ? (
                  <Button
                    type="button"
                    data-testid="skills-cli-detail-update"
                    disabled={isMutating || Boolean(mutationLockReason)}
                    title={mutationLockReason}
                    onClick={onUpdate}
                  >
                    {t("skillsCli.detail.update")}
                  </Button>
                ) : null}
                <Button
                  type="button"
                  variant="outline"
                  data-testid="skills-cli-detail-reveal"
                  disabled={isMutating || Boolean(mutationLockReason)}
                  title={mutationLockReason}
                  onClick={() => void runReveal()}
                >
                  <FolderOpen className="size-4" />
                  {t("skillsCli.detail.reveal")}
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  data-testid="skills-cli-detail-uninstall"
                  disabled={isMutating || Boolean(mutationLockReason)}
                  title={mutationLockReason}
                  onClick={onUninstall}
                >
                  <Trash2 className="size-4" />
                  {t("skillsCli.detail.uninstall")}
                </Button>
              </div>
              {revealError ? (
                <p
                  role="alert"
                  data-testid="skills-cli-detail-reveal-error"
                  className="border-t border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive-text"
                >
                  {revealError}
                </p>
              ) : null}
            </div>
          ) : null}
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}
