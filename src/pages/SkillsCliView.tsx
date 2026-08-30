import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { SkillsCliBatchBar } from "@/components/skillsCli/SkillsCliBatchBar";
import { SkillsCliCleanupDialog } from "@/components/skillsCli/SkillsCliCleanupDialog";
import { SkillsCliDetailDrawer } from "@/components/skillsCli/SkillsCliDetailDrawer";
import { SkillsCliGroupHeader } from "@/components/skillsCli/SkillsCliGroupHeader";
import { SkillsCliHeader } from "@/components/skillsCli/SkillsCliHeader";
import {
  SKILLS_CLI_INSTALL_SURFACE_AVAILABLE,
  SkillsCliInstallMount,
} from "@/components/skillsCli/SkillsCliInstallMount";
import { SkillsCliToolbar } from "@/components/skillsCli/SkillsCliToolbar";
import { SkillsCliUninstallDialog } from "@/components/skillsCli/SkillsCliUninstallDialog";
import { SkillsCliUpdateDrawer } from "@/components/skillsCli/SkillsCliUpdateDrawer";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import { isLocalTarget } from "@/lib/targetKind";
import { cn } from "@/lib/utils";
import {
  deriveCleanupCandidates,
  isGroupFullySelected,
  reconcileSelectedNames,
  selectedHasManagedLink,
  summarizeLinkTargets,
} from "@/pages/skillsCliBatchModel";
import { createSkillsCliPageHandlers } from "@/pages/skillsCliPageHandlers";
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
  openSkillsCliCleanup,
  openSkillsCliDetail,
  openSkillsCliInstall,
  openSkillsCliUninstall,
  actionableUpdateSkillNames,
  pendingUpdateCountForSkills,
  repositoryKeyForSkills,
  skillHasPendingUpdate,
  skillsCliRemoteMutationLockReason,
  updateRowForSkill,
  type SkillsCliActiveSurface,
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
  const revealLockReason = skillsCliRemoteMutationLockReason(isLocal, t);
  const installUpdateLockReason = undefined;

  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const doctor = useSkillsCliStore((state) => state.doctor);
  const canonicalRoot = useSkillsCliStore((state) => state.canonicalRoot);
  const lockPath = useSkillsCliStore((state) => state.lockPath);
  const isLoading = useSkillsCliStore((state) => state.isLoading);
  const isRefreshing = useSkillsCliStore((state) => state.isRefreshing);
  const isMutating = useSkillsCliStore((state) => state.isMutating);
  const batchProgress = useSkillsCliStore((state) => state.batchProgress);
  const runtimeError = useSkillsCliStore((state) => state.runtimeError);
  const inventoryError = useSkillsCliStore((state) => state.inventoryError);
  const loadAll = useSkillsCliStore((state) => state.loadAll);
  const previewRemoveGlobal = useSkillsCliStore((state) => state.previewRemoveGlobal);
  const removeGlobalBatch = useSkillsCliStore((state) => state.removeGlobalBatch);
  const docState = useSkillsCliStore((state) => state.docState);
  const updateInventory = useSkillsCliStore((state) => state.updateInventory);
  const updateJob = useSkillsCliStore((state) => state.updateJob);
  const updateError = useSkillsCliStore((state) => state.updateError);
  const updateProgress = useSkillsCliStore((state) => state.updateProgress);

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
  const [unlinkMenuOpen, setUnlinkMenuOpen] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const installButtonRef = useRef<HTMLButtonElement>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

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
  }, []);

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
  const cleanupCandidates = useMemo(
    () => deriveCleanupCandidates(skills),
    [skills],
  );
  const batchBusy = isMutating || batchProgress !== null;
  const detailName =
    activeSurface?.kind === "detail" ? activeSurface.skillName : null;
  const detailFocus =
    activeSurface?.kind === "detail" ? activeSurface.focus : null;
  const detailSkill =
    detailName == null
      ? null
      : (skills.find((skill) => skill.name === detailName) ?? null);

  useEffect(() => {
    if (detailName == null) {
      useSkillsCliStore.getState().clearSkillDoc();
      return;
    }
    void useSkillsCliStore.getState().readSkillDoc(detailName);
  }, [detailName]);

  const handleFocusConsumed = useCallback(() => {
    setActiveSurface((current) => {
      if (current?.kind !== "detail" || current.focus == null) {
        return current;
      }
      return { kind: "detail", skillName: current.skillName, focus: null };
    });
  }, []);

  const {
    captureReturnFocus,
    handlePageKeyDown,
    handleSelectModeChange,
    handleSelectAll,
    handleExport,
    handleLink,
    handleUnlink,
    handleUnlinkPlatform,
    handleBatchUpdate,
    handleUninstalled,
    handleDetailClose,
    openUpdateSurface,
    handleDetailToggle,
    handleDetailForceUnlink,
    handleDetailLinkAll,
    handleDetailUnlinkAll,
    toggleCollapsed,
    toggleCardSelected,
  } = createSkillsCliPageHandlers({
    t,
    activeSurface,
    linkMenuOpen,
    unlinkMenuOpen,
    selectedCardNames,
    targets,
    detailSkill,
    returnFocusRef,
    setSelectMode,
    setSelectedCardNames,
    setCollapsedGroupIds,
    setActiveSurface,
    setLinkMenuOpen,
    setUnlinkMenuOpen,
    setIsExporting,
  });

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
        data-update-repo={
          activeSurface?.kind === "update" ? activeSurface.repositoryKey : ""
        }
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
        isCheckingUpdates={updateJob.phase === "checking"}
        installAvailable={SKILLS_CLI_INSTALL_SURFACE_AVAILABLE}
        mutationLockReason={installUpdateLockReason}
        onRefresh={() => void loadAll()}
        onCheckUpdates={() => {
          void useSkillsCliStore
            .getState()
            .checkUpdates()
            .catch((error) => {
              showSkillsCliActionToast({
                semantic: "error",
                message: formatBackendError(error, t),
              });
            });
        }}
        onCancelUpdate={() => {
          void useSkillsCliStore.getState().cancelUpdateJob();
        }}
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
            onCleanupUnavailable={() => {
              if (cleanupCandidates.length === 0 || batchBusy) {
                return;
              }
              captureReturnFocus(document.activeElement);
              setActiveSurface(openSkillsCliCleanup());
            }}
            cleanupUnavailableCount={cleanupCandidates.length}
            cleanupDisabled={batchBusy}
          />

          {batchProgress ? (
            <p
              role="status"
              aria-live="polite"
              data-testid="skills-cli-batch-progress"
              aria-label={t("skillsCli.batch.progressAria", {
                completed: batchProgress.completed,
                total: batchProgress.total,
              })}
              className="text-xs tabular-nums text-muted-foreground"
            >
              {t("skillsCli.batch.progress", {
                completed: batchProgress.completed,
                total: batchProgress.total,
              })}
            </p>
          ) : null}

          {updateInventory.lastSuccessAt ? (
            <p
              data-testid="skills-cli-update-last-checked"
              className="text-xs text-muted-foreground"
            >
              {t("skillsCli.updates.lastChecked", {
                time: updateInventory.lastSuccessAt,
              })}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">
              {t("skillsCli.updates.notChecked")}
            </p>
          )}

          {updateProgress ? (
            <p
              data-testid="skills-cli-update-check-progress"
              className="text-xs text-muted-foreground"
            >
              {t("skillsCli.updates.progress", {
                phase: updateProgress.phase,
                completed: updateProgress.repositoryCompleted,
                total: updateProgress.repositoryTotal,
              })}
            </p>
          ) : null}

          {updateInventory.pendingRecovery ? (
            <div
              role="alert"
              data-testid="skills-cli-update-recovery-banner"
              className="flex flex-wrap items-center gap-2 text-sm text-destructive-text"
            >
              <span>{t("skillsCli.updates.recoveryRequired")}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  const operationId =
                    useSkillsCliStore.getState().updateInventory.pendingRecovery
                      ?.operationId;
                  if (!operationId) {
                    return;
                  }
                  void useSkillsCliStore
                    .getState()
                    .retryUpdateRecovery(operationId)
                    .catch((error) => {
                      showSkillsCliActionToast({
                        semantic: "error",
                        message: formatBackendError(error, t),
                      });
                    });
                }}
              >
                {t("skillsCli.updates.retryRecovery")}
              </Button>
            </div>
          ) : null}

          {updateError && (
            <div
              role="alert"
              data-testid="skills-cli-update-cache-error"
              className="flex flex-wrap items-center gap-2 text-sm text-destructive-text"
            >
              <span>{formatBackendError(updateError, t)}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => void useSkillsCliStore.getState().checkUpdates()}
              >
                {t("skillsCli.updates.retry")}
              </Button>
            </div>
          )}

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
                  const updateCount = pendingUpdateCountForSkills(
                    bucket.skills,
                    updateInventory,
                  );
                  const groupRepositoryKey = repositoryKeyForSkills(
                    bucket.skills,
                    updateInventory,
                  );
                  const actionableNames = actionableUpdateSkillNames(
                    bucket.skills,
                    updateInventory,
                    Boolean(updateInventory.pendingRecovery),
                  );
                  return (
                    <section key={bucket.id}>
                      <SkillsCliGroupHeader
                        bucket={bucket}
                        label={bucketLabel(t, bucket.labelKey, bucket.labelValue)}
                        expanded={expanded}
                        panelId={panelId}
                        onToggle={() => toggleCollapsed(bucket.id)}
                        onSelectAll={() => handleSelectAll(bucket)}
                        allSelected={isGroupFullySelected(
                          selectedCardNames,
                          bucket.skills.map((skill) => skill.name),
                        )}
                        updateCount={updateCount}
                        onUpdateAll={
                          installUpdateLockReason
                            ? undefined
                            : updateCount > 0 && groupRepositoryKey
                            ? () =>
                                openUpdateSurface({
                                  repositoryKey: groupRepositoryKey,
                                  skillNames: actionableNames,
                                  from: document.activeElement,
                                })
                            : undefined
                        }
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
                              updateAvailable={skillHasPendingUpdate(
                                updateRowForSkill(updateInventory, skill.name),
                              )}
                              checkbox={
                                selectMode
                                  ? {
                                      checked: selectedCardNames.has(skill.name),
                                      onChange: () => toggleCardSelected(skill.name),
                                    }
                                  : undefined
                              }
                              onDetail={(event) => {
                                captureReturnFocus(event.currentTarget);
                                setActiveSurface(
                                  openSkillsCliDetail(skill.name, null),
                                );
                              }}
                              onManageLinks={() => {
                                captureReturnFocus(document.activeElement);
                                setActiveSurface(
                                  openSkillsCliDetail(skill.name, "links"),
                                );
                              }}
                              onUninstall={() => {
                                captureReturnFocus(document.activeElement);
                                setActiveSurface(
                                  openSkillsCliUninstall([skill.name]),
                                );
                              }}
                              isLoading={isMutating}
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
          busy={batchBusy}
          exporting={isExporting}
          linkMenuOpen={linkMenuOpen}
          onLinkMenuOpenChange={setLinkMenuOpen}
          unlinkMenuOpen={unlinkMenuOpen}
          onUnlinkMenuOpenChange={setUnlinkMenuOpen}
          onLink={(agentId) => void handleLink(agentId)}
          onUnlink={() => void handleUnlink()}
          onUnlinkPlatform={(agentId) => void handleUnlinkPlatform(agentId)}
          onUpdate={() => void handleBatchUpdate()}
          onExportSelected={() => void handleExport("selected")}
          onUninstall={() =>
            setActiveSurface(openSkillsCliUninstall([...selectedCardNames]))
          }
          onClear={() => {
            setSelectMode(false);
            setSelectedCardNames(new Set());
          }}
          updateLockReason={installUpdateLockReason}
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

      <SkillsCliCleanupDialog
        open={activeSurface?.kind === "cleanup"}
        candidates={cleanupCandidates}
        busy={batchBusy}
        returnFocusRef={returnFocusRef}
        onOpenChange={(open) => {
          if (!open) {
            setActiveSurface(closeSkillsCliSurface());
          }
        }}
        onConfirm={(names) => {
          setActiveSurface(openSkillsCliUninstall(names));
        }}
      />

      <SkillsCliUninstallDialog
        open={activeSurface?.kind === "uninstall"}
        skillNames={uninstallNames}
        isMutating={isMutating}
        returnFocusRef={returnFocusRef}
        onOpenChange={(open) => {
          if (!open) {
            setActiveSurface(closeSkillsCliSurface());
          }
        }}
        previewRemoveGlobal={previewRemoveGlobal}
        removeGlobalBatch={removeGlobalBatch}
        onRemoved={handleUninstalled}
      />

      <SkillsCliDetailDrawer
        open={activeSurface?.kind === "detail"}
        skill={detailSkill}
        targets={targets}
        contentWidth={contentWidthPx}
        docState={docState}
        updateAvailable={skillHasPendingUpdate(
          detailSkill
            ? updateRowForSkill(updateInventory, detailSkill.name)
            : null,
        )}
        focusSection={detailFocus}
        isMutating={isMutating}
        returnFocusRef={returnFocusRef}
        onClose={handleDetailClose}
        onFocusConsumed={handleFocusConsumed}
        onToggleLink={handleDetailToggle}
        onForceUnlink={handleDetailForceUnlink}
        onLinkAll={handleDetailLinkAll}
        onUnlinkAll={handleDetailUnlinkAll}
        onRetryDoc={() => {
          if (detailName) {
            void useSkillsCliStore.getState().readSkillDoc(detailName);
          }
        }}
        onRevealFolder={() => {
          if (!detailSkill) {
            return;
          }
          return useSkillsCliStore
            .getState()
            .revealSkillFolder(detailSkill.name);
        }}
        onUpdate={
          detailSkill
            ? () =>
                openUpdateSurface({
                  repositoryKey: updateRowForSkill(
                    updateInventory,
                    detailSkill.name,
                  )?.repositoryKey ?? null,
                  skillNames: [detailSkill.name],
                  from: document.activeElement,
                })
            : undefined
        }
        onUninstall={() => {
          if (!detailSkill) {
            return;
          }
          setActiveSurface(openSkillsCliUninstall([detailSkill.name]));
        }}
        mutationLockReason={installUpdateLockReason}
        revealLockReason={revealLockReason}
      />

      <SkillsCliUpdateDrawer
        open={activeSurface?.kind === "update"}
        repositoryKey={
          activeSurface?.kind === "update" ? activeSurface.repositoryKey : ""
        }
        skillNames={
          activeSurface?.kind === "update" ? activeSurface.skillNames : []
        }
        skills={skills}
        inventory={updateInventory}
        contentWidth={contentWidthPx}
        updateError={updateError}
        updateJobPhase={updateJob.phase}
        updateProgress={updateProgress}
        returnFocusRef={returnFocusRef}
        onClose={() => setActiveSurface(closeSkillsCliSurface())}
        onApply={(input) => useSkillsCliStore.getState().applyUpdates(input)}
        onVerifyBaseline={(names) =>
          useSkillsCliStore.getState().verifyUpdateBaseline(names)
        }
        onRetryRecovery={(operationId) =>
          useSkillsCliStore.getState().retryUpdateRecovery(operationId)
        }
      />
    </div>
  );
}
