import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "lucide-react";

import { SkillsCliBatchBar } from "@/components/skillsCli/SkillsCliBatchBar";
import { SkillsCliGroupHeader } from "@/components/skillsCli/SkillsCliGroupHeader";
import { SkillsCliHeader } from "@/components/skillsCli/SkillsCliHeader";
import {
  SKILLS_CLI_INSTALL_SURFACE_AVAILABLE,
  SkillsCliInstallMount,
} from "@/components/skillsCli/SkillsCliInstallMount";
import { SkillsCliToolbar } from "@/components/skillsCli/SkillsCliToolbar";
import { SkillsCliUninstallDialog } from "@/components/skillsCli/SkillsCliUninstallDialog";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import { isEditableEventTarget } from "@/lib/keyboardShortcuts";
import { isLocalTarget } from "@/lib/targetKind";
import { cn } from "@/lib/utils";
import {
  reconcileSelectedNames,
  selectedHasManagedLink,
  summarizeLinkTargets,
  type PlacementMutationOutcome,
} from "@/pages/skillsCliBatchModel";
import { exportSkillsCliInventory } from "@/pages/skillsCliExport";
import {
  SKILLS_CLI_CONTENT_CONTAINER_CLASS,
  SKILLS_CLI_GRID_CLASS,
  SKILLS_CLI_SKELETON_COUNT,
  bucketSkillsCli,
  closeSkillsCliSurface,
  deriveSkillsCliCounts,
  deriveSkillsCliLayoutBands,
  enabledTargetIdSet,
  filterSkillsCli,
  openSkillsCliDetail,
  openSkillsCliInstall,
  openSkillsCliUninstall,
  type SkillsCliActiveSurface,
  type SkillsCliBucket,
  type SkillsCliGroupBy,
} from "@/pages/skillsCliViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";

const EMPTY_UNINSTALL_NAMES: readonly string[] = [];

function bucketLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  labelKey: string,
  labelValue?: string,
): string {
  return labelValue ? t(labelKey, { name: labelValue }) : t(labelKey);
}

export function SkillsCliView() {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isLocal = isLocalTarget(activeTarget);

  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const doctor = useSkillsCliStore((state) => state.doctor);
  const canonicalRoot = useSkillsCliStore((state) => state.canonicalRoot);
  const lockPath = useSkillsCliStore((state) => state.lockPath);
  const isLoading = useSkillsCliStore((state) => state.isLoading);
  const isRefreshing = useSkillsCliStore((state) => state.isRefreshing);
  const isMutating = useSkillsCliStore((state) => state.isMutating);
  const runtimeError = useSkillsCliStore((state) => state.runtimeError);
  const inventoryError = useSkillsCliStore((state) => state.inventoryError);
  const loadAll = useSkillsCliStore((state) => state.loadAll);
  const previewRemoveGlobal = useSkillsCliStore((state) => state.previewRemoveGlobal);
  const removeGlobalBatch = useSkillsCliStore((state) => state.removeGlobalBatch);

  const [query, setQuery] = useState("");
  const [groupBy, setGroupBy] = useState<SkillsCliGroupBy>("repo");
  const [platformFilter, setPlatformFilter] = useState<string | null>(null);
  const [unlinkedOnly, setUnlinkedOnly] = useState(false);
  const [selectMode, setSelectMode] = useState(false);
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [selectedCardNames, setSelectedCardNames] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [activeSurface, setActiveSurface] = useState<SkillsCliActiveSurface>(null);
  const [contentWidthPx, setContentWidthPx] = useState<number | null>(null);
  const [linkMenuOpen, setLinkMenuOpen] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const installButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!isLocal) {
      return;
    }
    void loadAll();
  }, [isLocal, loadAll]);

  useEffect(() => {
    const node = contentRef.current;
    if (!node || typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (typeof width === "number") {
        setContentWidthPx(width);
      }
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, [isLocal]);

  useEffect(() => {
    setSelectedCardNames((current) => {
      const next = reconcileSelectedNames(
        current,
        skills.map((skill) => skill.name),
      );
      if (next.size === current.size) {
        for (const name of next) {
          if (!current.has(name)) {
            return next;
          }
        }
        return current;
      }
      return next;
    });
  }, [skills]);

  const runtimeBlocked = runtimeError !== null;
  const showInventoryEmpty = skills.length === 0 && !isLoading && !inventoryError;
  const showLoading = isLoading && skills.length === 0 && !inventoryError;
  const enabledIds = useMemo(() => enabledTargetIdSet(targets), [targets]);
  const counts = useMemo(
    () => deriveSkillsCliCounts(skills, enabledIds),
    [skills, enabledIds],
  );
  const filtered = useMemo(
    () =>
      filterSkillsCli(
        skills,
        { query, platformFilter, unlinkedOnly },
        enabledIds,
      ),
    [skills, query, platformFilter, unlinkedOnly, enabledIds],
  );
  const buckets = useMemo(
    () => bucketSkillsCli(filtered, groupBy, targets, enabledIds),
    [filtered, groupBy, targets, enabledIds],
  );
  const showFilteredEmpty =
    skills.length > 0 && filtered.length === 0 && !showLoading;
  const layoutBands = deriveSkillsCliLayoutBands(contentWidthPx);
  const uninstallNames =
    activeSurface?.kind === "uninstall"
      ? activeSurface.skillNames
      : EMPTY_UNINSTALL_NAMES;
  const linkSummaries = useMemo(
    () => summarizeLinkTargets(skills, selectedCardNames, targets),
    [skills, selectedCardNames, targets],
  );
  const unlinkEnabled = selectedHasManagedLink(skills, selectedCardNames);

  if (!isLocal) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8">
        <Terminal className="size-10 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{t("skillsCli.localOnly")}</p>
      </div>
    );
  }

  function handlePageKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Escape") {
      return;
    }
    if (
      activeSurface !== null ||
      linkMenuOpen ||
      event.defaultPrevented ||
      isEditableEventTarget(event.target)
    ) {
      return;
    }
    setSelectMode(false);
    setSelectedCardNames(new Set());
  }

  function handleSelectModeChange(next: boolean) {
    setSelectMode(next);
    if (!next) {
      setSelectedCardNames(new Set());
    }
  }

  function handleSelectAll(bucket: SkillsCliBucket) {
    setSelectMode(true);
    setSelectedCardNames((current) => {
      const next = new Set(current);
      for (const skill of bucket.skills) {
        next.add(skill.name);
      }
      return next;
    });
  }

  function toastPlacementOutcome(
    outcome: PlacementMutationOutcome,
    kind: "link" | "unlink",
  ) {
    const allOk = outcome.failed.length === 0 && outcome.skipped.length === 0;
    const failedMessages = [
      ...new Set(outcome.failed.map((item) => item.errorCode)),
    ].map((code) => formatBackendError(`${code}:`, t));
    showSkillsCliActionToast({
      semantic: allOk ? "success" : "error",
      message: allOk
        ? t(
            kind === "link"
              ? "skillsCli.batch.linkSuccess"
              : "skillsCli.batch.unlinkSuccess",
            { succeeded: outcome.succeeded.length },
          )
        : [
            t(
              kind === "link"
                ? "skillsCli.batch.linkPartial"
                : "skillsCli.batch.unlinkPartial",
              {
                succeeded: outcome.succeeded.length,
                failed: outcome.failed.length,
                skipped: outcome.skipped.length,
              },
            ),
            ...failedMessages,
          ]
            .join(" ")
            .trim(),
    });
  }

  async function handleExport(scope: "all" | "selected") {
    setIsExporting(true);
    try {
      const result = await exportSkillsCliInventory({
        scope,
        skills: useSkillsCliStore.getState().skills,
        selectedNames: selectedCardNames,
        targets,
        exportInventory: (input) =>
          useSkillsCliStore.getState().exportInventory(input),
      });
      if (result === "cancelled") {
        return;
      }
      showSkillsCliActionToast({
        semantic: "success",
        message: t(
          scope === "all"
            ? "skillsCli.export.successAll"
            : "skillsCli.export.successSelected",
        ),
      });
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.export.error", {
          error: formatBackendError(error, t),
        }),
      });
    } finally {
      setIsExporting(false);
    }
  }

  async function handleLink(agentId: string) {
    setLinkMenuOpen(false);
    try {
      const outcome = await useSkillsCliStore
        .getState()
        .linkPlatformBatch([...selectedCardNames], agentId);
      toastPlacementOutcome(outcome, "link");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  async function handleUnlink() {
    try {
      const outcome = await useSkillsCliStore
        .getState()
        .unlinkManagedBatch([...selectedCardNames]);
      toastPlacementOutcome(outcome, "unlink");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  function handleUninstalled(names: string[]) {
    setSelectedCardNames((current) => {
      const next = new Set(current);
      for (const name of names) {
        next.delete(name);
      }
      return next;
    });
  }

  function toggleCollapsed(id: string) {
    setCollapsedGroupIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function toggleCardSelected(name: string) {
    setSelectedCardNames((current) => {
      const next = new Set(current);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  return (
    <div
      className="flex h-full min-w-0 flex-col overflow-hidden overflow-x-hidden"
      data-testid="skills-cli-page"
      onKeyDown={handlePageKeyDown}
    >
      <div
        hidden
        data-testid="skills-cli-active-surface"
        data-kind={activeSurface?.kind ?? "none"}
        data-focus={
          activeSurface?.kind === "detail" ? String(activeSurface.focus) : ""
        }
        data-uninstall={uninstallNames.join(",")}
      />
      <div
        hidden
        data-testid="skills-cli-layout-bands"
        data-grid={layoutBands.grid}
        data-drawer={layoutBands.drawer}
        data-width={contentWidthPx == null ? "" : String(contentWidthPx)}
      />
      <SkillsCliHeader
        counts={counts}
        doctor={doctor}
        runtimeError={runtimeError}
        isLoading={isLoading}
        isRefreshing={isRefreshing}
        installAvailable={SKILLS_CLI_INSTALL_SURFACE_AVAILABLE}
        onRefresh={() => void loadAll()}
        onOpenInstall={() => setActiveSurface(openSkillsCliInstall())}
        installButtonRef={installButtonRef}
      />

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto overflow-x-hidden px-6 py-4">
        <div
          ref={contentRef}
          data-testid="skills-cli-content"
          className={cn(SKILLS_CLI_CONTENT_CONTAINER_CLASS, "space-y-4")}
        >
          <SkillsCliToolbar
            query={query}
            onQueryChange={setQuery}
            groupBy={groupBy}
            onGroupByChange={setGroupBy}
            platformFilter={platformFilter}
            onPlatformFilterChange={setPlatformFilter}
            unlinkedOnly={unlinkedOnly}
            onUnlinkedOnlyChange={setUnlinkedOnly}
            selectMode={selectMode}
            onSelectModeChange={handleSelectModeChange}
            targets={targets}
            onExportAll={
              skills.length > 0 ? () => void handleExport("all") : undefined
            }
            isExporting={isExporting}
          />

          {inventoryError && (
            <div
              role="alert"
              data-testid="skills-cli-inventory-error"
              className="flex flex-wrap items-center gap-2 text-sm text-destructive-text"
            >
              <span>{formatBackendError(inventoryError, t)}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => void loadAll()}
              >
                {t("common.retry")}
              </Button>
            </div>
          )}

          <section data-testid="skills-cli-inventory">
            {showLoading ? (
              <div
                className={SKILLS_CLI_GRID_CLASS}
                aria-busy="true"
                data-testid="skills-cli-skeleton"
              >
                {Array.from({ length: SKILLS_CLI_SKELETON_COUNT }, (_, index) => (
                  <div
                    key={index}
                    aria-hidden
                    className="min-h-[76px] animate-pulse rounded-xl bg-muted/60"
                  />
                ))}
              </div>
            ) : showInventoryEmpty ? (
              <p className="text-sm text-muted-foreground">{t("skillsCli.empty")}</p>
            ) : showFilteredEmpty ? (
              <p className="text-sm text-muted-foreground">
                {t("skillsCli.filteredEmpty", { query })}
              </p>
            ) : (
              <div className="space-y-4">
                {buckets.map((bucket) => {
                  const panelId = `skills-cli-group-panel-${bucket.id}`;
                  const expanded = !collapsedGroupIds.has(bucket.id);
                  return (
                    <section key={bucket.id}>
                      <SkillsCliGroupHeader
                        bucket={bucket}
                        label={bucketLabel(t, bucket.labelKey, bucket.labelValue)}
                        expanded={expanded}
                        panelId={panelId}
                        onToggle={() => toggleCollapsed(bucket.id)}
                        onSelectAll={() => handleSelectAll(bucket)}
                      />
                      {expanded ? (
                        <div id={panelId} className={SKILLS_CLI_GRID_CLASS}>
                          {bucket.skills.map((skill) => (
                            <UnifiedSkillCard
                              key={skill.name}
                              variant="skillsCli"
                              layout="denseRow"
                              name={skill.name}
                              path={skill.canonicalPath ?? skill.path}
                              placements={skill.placements}
                              checkbox={
                                selectMode
                                  ? {
                                      checked: selectedCardNames.has(skill.name),
                                      onChange: () => toggleCardSelected(skill.name),
                                    }
                                  : undefined
                              }
                              onDetail={() =>
                                setActiveSurface(openSkillsCliDetail(skill.name, null))
                              }
                              onManageLinks={() =>
                                setActiveSurface(
                                  openSkillsCliDetail(skill.name, "links"),
                                )
                              }
                              onUninstall={() =>
                                setActiveSurface(openSkillsCliUninstall([skill.name]))
                              }
                              isLoading={isMutating || runtimeBlocked}
                            />
                          ))}
                        </div>
                      ) : null}
                    </section>
                  );
                })}
              </div>
            )}
          </section>

          {canonicalRoot && lockPath && (
            <div
              data-testid="skills-cli-paths"
              className="space-y-0.5 text-ui-meta text-muted-foreground"
            >
              <p>{t("skillsCli.pathsCanonical", { root: canonicalRoot })}</p>
              <p>{t("skillsCli.pathsLock", { lock: lockPath })}</p>
              <p>{t("skillsCli.ownershipNote")}</p>
            </div>
          )}
        </div>
      </div>

      {selectedCardNames.size > 0 ? (
        <SkillsCliBatchBar
          selectedCount={selectedCardNames.size}
          summaries={linkSummaries}
          unlinkEnabled={unlinkEnabled}
          busy={isMutating}
          runtimeBlocked={runtimeBlocked}
          exporting={isExporting}
          linkMenuOpen={linkMenuOpen}
          onLinkMenuOpenChange={setLinkMenuOpen}
          onLink={(agentId) => void handleLink(agentId)}
          onUnlink={() => void handleUnlink()}
          onExportSelected={() => void handleExport("selected")}
          onUninstall={() =>
            setActiveSurface(openSkillsCliUninstall([...selectedCardNames]))
          }
          onClear={() => {
            setSelectMode(false);
            setSelectedCardNames(new Set());
          }}
        />
      ) : null}

      <SkillsCliInstallMount
        open={activeSurface?.kind === "install"}
        onOpenChange={(open) => {
          setActiveSurface(
            open ? openSkillsCliInstall() : closeSkillsCliSurface(),
          );
        }}
        returnFocusRef={installButtonRef}
        contentWidthPx={contentWidthPx}
      />

      <SkillsCliUninstallDialog
        open={activeSurface?.kind === "uninstall"}
        skillNames={uninstallNames}
        isMutating={isMutating}
        runtimeBlocked={runtimeBlocked}
        onOpenChange={(open) => {
          if (open && runtimeBlocked) {
            return;
          }
          if (!open) {
            setActiveSurface(closeSkillsCliSurface());
          }
        }}
        previewRemoveGlobal={previewRemoveGlobal}
        removeGlobalBatch={removeGlobalBatch}
        onRemoved={handleUninstalled}
      />
    </div>
  );
}
