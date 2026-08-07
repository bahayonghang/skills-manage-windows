import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Trash2 } from "lucide-react";
import { toast } from "sonner";

import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import {
  useUpdateCenterStore,
  type UpdateCenterTab,
} from "@/stores/updateCenterStore";
import type {
  SkillRefreshMode,
  SkillRefreshScope,
  SkillRefreshScopeKind,
} from "@/types/skillUpdateInventory";
import type { UpdatableRowState } from "@/components/central/updateCenter/UpdatableTabPanel";
import {
  UpdateCenterTabContent,
  type UpdateCenterTabHandlers,
} from "@/components/central/updateCenter/UpdateCenterTabContent";
import { UpdateCenterToolbar } from "@/components/central/updateCenter/UpdateCenterToolbar";
import {
  buildDecisions,
  buildDeletedPlatformCopyCleanupDecisions,
  buildInitialState,
  countDeletedPlatformCopyPaths,
  countDecisionSelections,
  countsFromInventory,
  emptyDecisionState,
  inventorySignature,
  summarizeDecisionSelections,
  type DecisionState,
} from "@/components/central/updateCenter/decisionAggregation";
import {
  buildRefreshScope,
  coerceRefreshScopeKind,
  isRefreshScopeEnabled,
} from "@/lib/updateCenterRefreshScope";
import {
  buildRepositorySourceDisplayMap,
  buildSkillConflictSourceMap,
} from "@/lib/centralConflictSource";

const TAB_ORDER: readonly UpdateCenterTab[] = [
  "updatable",
  "added",
  "missing",
  "failed",
  "unsupported",
  "duplicates",
  "deletedPlatformCopies",
  "orphans",
] as const;

export function UpdateCenterDialog() {
  const { t } = useTranslation();
  const inventory = useUpdateCenterStore((state) => state.inventory);
  const isDialogOpen = useUpdateCenterStore((state) => state.isDialogOpen);
  const isRefreshing = useUpdateCenterStore((state) => state.isRefreshing);
  const isApplying = useUpdateCenterStore((state) => state.isApplying);
  const isForcing = useUpdateCenterStore((state) => state.isForcing);
  const activeTab = useUpdateCenterStore((state) => state.activeTab);
  const refreshContext = useUpdateCenterStore((state) => state.refreshContext);
  const refreshMode = useUpdateCenterStore((state) => state.refreshMode);
  const lastRefreshedAt = useUpdateCenterStore(
    (state) => state.lastRefreshedAt,
  );
  const error = useUpdateCenterStore((state) => state.error);
  const closeDialog = useUpdateCenterStore((state) => state.closeDialog);
  const refresh = useUpdateCenterStore((state) => state.refresh);
  const retryRepositories = useUpdateCenterStore(
    (state) => state.retryRepositories,
  );
  const retryingRepositoryIds = useUpdateCenterStore(
    (state) => state.retryingRepositoryIds,
  );
  const apply = useUpdateCenterStore((state) => state.apply);
  const clear = useUpdateCenterStore((state) => state.clear);
  const forceUpdateSkills = useUpdateCenterStore(
    (state) => state.forceUpdateSkills,
  );
  const forceMirrorRepositories = useUpdateCenterStore(
    (state) => state.forceMirrorRepositories,
  );
  const setActiveTab = useUpdateCenterStore((state) => state.setActiveTab);
  const setRefreshMode = useUpdateCenterStore((state) => state.setRefreshMode);
  const skills = useCentralSkillsStore((state) => state.skills ?? []);
  const repositories = useCentralSkillsStore(
    (state) => state.repositories ?? [],
  );

  const [scopeKind, setScopeKind] = useState<SkillRefreshScopeKind>("all");
  const [decisions, setDecisions] = useState<DecisionState>(emptyDecisionState);
  const [isCleaningLeftovers, setIsCleaningLeftovers] = useState(false);
  const existingSkillSources = useMemo(
    () => buildSkillConflictSourceMap(skills),
    [skills],
  );
  const repositorySources = useMemo(
    () => buildRepositorySourceDisplayMap(repositories),
    [repositories],
  );

  const inventoryKey = useMemo(
    () => inventorySignature(inventory),
    [inventory],
  );
  const inventoryRef = useRef(inventory);

  useEffect(() => {
    inventoryRef.current = inventory;
  }, [inventory, inventoryKey]);

  useEffect(() => {
    if (!isDialogOpen) return;
    setDecisions(buildInitialState(inventoryRef.current, scopeKind));
  }, [isDialogOpen, inventoryKey, scopeKind]);

  const counts = countsFromInventory(inventory);
  const scopeEnabled = useMemo(
    () => ({
      all: true,
      repositories: isRefreshScopeEnabled("repositories", refreshContext),
      platform: isRefreshScopeEnabled("platform", refreshContext),
      skills: isRefreshScopeEnabled("skills", refreshContext),
    }),
    [refreshContext],
  );
  const totalSelected = useMemo(
    () => countDecisionSelections(decisions, inventory),
    [decisions, inventory],
  );
  const deletedPlatformCopyPathCount = useMemo(
    () => countDeletedPlatformCopyPaths(inventory),
    [inventory],
  );
  const selectionSummary = useMemo(
    () => summarizeDecisionSelections(decisions, inventory),
    [decisions, inventory],
  );
  const selectedUpdateIds = useMemo(() => {
    if (!inventory) return [];
    return inventory.updatable
      .map((item) => item.state.skill_id)
      .filter((skillId) => decisions.updatable[skillId]?.selected);
  }, [decisions.updatable, inventory]);

  useEffect(() => {
    if (!isDialogOpen) return;
    const next = coerceRefreshScopeKind(scopeKind, refreshContext);
    if (next !== scopeKind) {
      setScopeKind(next);
    }
  }, [isDialogOpen, refreshContext, scopeKind]);

  useEffect(() => {
    if (!isDialogOpen) return;
    if (refreshContext.agentIds.length > 0) {
      setScopeKind("platform");
    } else if (refreshContext.repositoryIds.length > 0) {
      setScopeKind("repositories");
    } else if (refreshContext.skillIds.length > 0) {
      setScopeKind("skills");
    } else {
      setScopeKind("all");
    }
  }, [isDialogOpen, refreshContext]);

  function currentRefreshScope(): SkillRefreshScope {
    return buildRefreshScope(scopeKind, refreshContext, refreshMode);
  }

  function handleRefresh() {
    const scope = currentRefreshScope();
    void refresh(scope);
  }

  function handleClear() {
    void clear(currentRefreshScope());
  }

  async function handleApplySelected() {
    if (!inventory) return;
    const scope = currentRefreshScope();
    const payload = buildDecisions(
      decisions,
      inventory,
      scope.kind === "platform" ? (scope.agentIds ?? []) : undefined,
    );
    try {
      const result = await apply(payload, scope);
      const succeeded =
        result.updatedSkillIds.length +
        result.keptMissingSkillIds.length +
        result.deletedSkillIds.length +
        result.importedSkillIds.length +
        result.skippedAdditions.length +
        result.unskippedAdditions.length +
        result.removedPlatformDuplicatePaths.length +
        result.removedDeletedPlatformCopyPaths.length;
      const failedCount = result.failures.length;
      if (failedCount === 0) {
        toast.success(
          t("central.updateCenter.applySuccess", { count: succeeded }),
        );
      } else {
        toast.error(
          t("central.updateCenter.applyPartialFailure", {
            succeeded,
            failed: failedCount,
          }),
        );
        for (const failure of result.failures.slice(0, 3)) {
          const message = failure.errorCode
            ? formatBackendError(
                {
                  code: failure.errorCode,
                  message: t("backendErrors.central_updates.item_failed"),
                  retryable: false,
                },
                t,
              )
            : t("backendErrors.central_updates.item_failed");
          toast.error(`${failure.step}: ${message}`);
        }
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.applyError", {
          error: formatBackendError(err, t),
        }),
      );
    }
  }

  async function handleCleanAllDeletedPlatformCopies() {
    if (!inventory || deletedPlatformCopyPathCount === 0) return;
    const confirmed = window.confirm(
      t("central.updateCenter.deletedPlatformCopies.cleanupAllConfirm", {
        count: deletedPlatformCopyPathCount,
      }),
    );
    if (!confirmed) return;
    const scope = currentRefreshScope();
    const payload = buildDeletedPlatformCopyCleanupDecisions(
      inventory,
      scope.kind === "platform" ? (scope.agentIds ?? []) : undefined,
    );
    setIsCleaningLeftovers(true);
    try {
      const result = await apply(payload, scope);
      const succeeded = result.removedDeletedPlatformCopyPaths.length;
      const failedCount = result.failures.length;
      if (failedCount === 0) {
        toast.success(
          t("central.updateCenter.deletedPlatformCopies.cleanupAllSuccess", {
            count: succeeded,
          }),
        );
      } else {
        toast.error(
          t("central.updateCenter.deletedPlatformCopies.cleanupAllPartial", {
            succeeded,
            failed: failedCount,
          }),
        );
        for (const failure of result.failures.slice(0, 3)) {
          toast.error(
            `${failure.step}: ${formatBackendError(
              `${failure.errorCode ?? "central_updates.item_failure"}:${failure.error}`,
              t,
            )}`,
          );
        }
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.deletedPlatformCopies.cleanupAllError", {
          error: formatBackendError(err, t),
        }),
      );
    } finally {
      setIsCleaningLeftovers(false);
    }
  }

  async function handleRetryRepositories(
    repositoryIds: string[],
    mode?: SkillRefreshMode,
  ) {
    if (repositoryIds.length === 0) return;
    try {
      const result = await retryRepositories(
        repositoryIds,
        currentRefreshScope(),
        mode ? { mode } : undefined,
      );
      if (result.failed.length === 0) {
        toast.success(
          t("central.updateCenter.failed.retrySuccess", {
            count: result.succeeded.length,
          }),
        );
      } else {
        toast.error(
          t("central.updateCenter.failed.retryPartial", {
            succeeded: result.succeeded.length,
            failed: result.failed.length,
          }),
        );
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.failed.retryError", {
          error: formatBackendError(err, t),
        }),
      );
    }
  }

  async function handleForceUpdateSelected() {
    if (selectedUpdateIds.length === 0) return;
    const confirmed = window.confirm(
      t("central.updateCenter.forceUpdateConfirm", {
        count: selectedUpdateIds.length,
      }),
    );
    if (!confirmed) return;
    const scope = currentRefreshScope();
    try {
      const result = await forceUpdateSkills(
        { skillIds: selectedUpdateIds, refreshCopyInstallations: true },
        scope,
      );
      toast.success(
        t("central.updateCenter.forceUpdateSuccess", {
          count: result.overwritten.length,
        }),
      );
      for (const failure of result.failed.slice(0, 3)) {
        toast.error(`${failure.skillId}: ${failure.error}`);
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.forceUpdateError", { error: String(err) }),
      );
    }
  }

  async function handleForceMirrorRepositories() {
    const repositoryIds = refreshContext.repositoryIds;
    if (scopeKind !== "repositories" || repositoryIds.length === 0) return;
    const confirmed = window.confirm(
      t("central.updateCenter.forceMirrorConfirm", {
        repositories: repositoryIds.length,
        updates: inventory?.updatable.length ?? 0,
        additions: inventory?.remoteAdded.length ?? 0,
        deletes: inventory?.remoteMissing.length ?? 0,
      }),
    );
    if (!confirmed) return;
    const scope = currentRefreshScope();
    try {
      const result = await forceMirrorRepositories(
        {
          repositoryIds,
          overwriteTracked: true,
          importAdded: true,
          deleteMissing: true,
          removeCopyInstallationsForDeleted: true,
        },
        scope,
      );
      toast.success(
        t("central.updateCenter.forceMirrorSuccess", {
          overwritten: result.overwritten.length,
          imported: result.imported.length,
          deleted: result.deleted.succeeded.length,
        }),
      );
      for (const failure of result.failedItems.slice(0, 3)) {
        toast.error(`${failure.skillId}: ${failure.error}`);
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.forceMirrorError", { error: String(err) }),
      );
    }
  }

  const handlers: UpdateCenterTabHandlers = {
    updateUpdatable(skillId, patch) {
      setDecisions((current) => ({
        ...current,
        updatable: {
          ...current.updatable,
          [skillId]: {
            ...(current.updatable[skillId] ?? { selected: false }),
            ...patch,
          },
        },
      }));
    },
    toggleAllUpdatable(selected) {
      if (!inventory) return;
      setDecisions((current) => {
        const next: Record<string, UpdatableRowState> = {};
        for (const item of inventory.updatable) {
          next[item.state.skill_id] = { selected };
        }
        return { ...current, updatable: next };
      });
    },
    updateAdded(key, patch) {
      setDecisions((current) => ({
        ...current,
        added: {
          ...current.added,
          [key]: {
            ...(current.added[key] ?? {
              selected: true,
              resolution: "overwrite",
              renamedSkillId: "",
            }),
            ...patch,
          },
        },
      }));
    },
    updateMissing(skillId, patch) {
      setDecisions((current) => ({
        ...current,
        missing: {
          ...current.missing,
          [skillId]: {
            ...(current.missing[skillId] ?? {
              decision: "keep",
              removeAgentIds: [],
            }),
            ...patch,
          },
        },
      }));
    },
    updateDuplicates(key, patch) {
      setDecisions((current) => ({
        ...current,
        duplicates: {
          ...current.duplicates,
          [key]: {
            ...(current.duplicates[key] ?? { selectedPaths: [] }),
            ...patch,
          },
        },
      }));
    },
    updateDeletedPlatformCopies(key, patch) {
      setDecisions((current) => ({
        ...current,
        deletedPlatformCopies: {
          ...current.deletedPlatformCopies,
          [key]: {
            ...(current.deletedPlatformCopies[key] ?? { selectedPaths: [] }),
            ...patch,
          },
        },
      }));
    },
    retryRepositories(repositoryIds, mode) {
      void handleRetryRepositories(repositoryIds, mode);
    },
  };

  return (
    <Dialog
      open={isDialogOpen}
      onOpenChange={(open) => {
        if (!open) closeDialog();
      }}
    >
      <DialogContent className="sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle>{t("central.updateCenter.title")}</DialogTitle>
        </DialogHeader>

        <DialogBody className="space-y-4">
          <UpdateCenterToolbar
            scopeKind={scopeKind}
            onScopeKindChange={setScopeKind}
            refreshMode={refreshMode}
            onRefreshModeChange={setRefreshMode}
            isRefreshing={isRefreshing}
            onRefresh={handleRefresh}
            lastRefreshedAt={lastRefreshedAt}
            activeTab={activeTab}
            onTabChange={setActiveTab}
            tabOrder={TAB_ORDER}
            counts={counts}
            scopeEnabled={scopeEnabled}
          />

          <div className="min-h-[20rem] rounded-xl border border-border p-4 text-sm">
            {error ? (
              <p className="mb-2 text-xs text-destructive-text" role="alert">
                {formatBackendError(error, t)}
              </p>
            ) : null}
            <UpdateCenterTabContent
              tab={activeTab}
              inventory={inventory}
              decisions={decisions}
              handlers={handlers}
              existingSkillSources={existingSkillSources}
              repositorySources={repositorySources}
              retryingRepositoryIds={retryingRepositoryIds}
              actionsDisabled={isRefreshing || isApplying || isForcing}
            />
          </div>
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            size="sm"
            onClick={handleClear}
            disabled={isApplying || isRefreshing || isForcing}
          >
            {t("central.updateCenter.clearInventory")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleForceUpdateSelected}
            disabled={
              isApplying ||
              isRefreshing ||
              isForcing ||
              selectedUpdateIds.length === 0
            }
          >
            {t("central.updateCenter.forceUpdateSelected", {
              count: selectedUpdateIds.length,
            })}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={handleCleanAllDeletedPlatformCopies}
            disabled={
              isApplying ||
              isRefreshing ||
              isForcing ||
              deletedPlatformCopyPathCount === 0 ||
              !inventory
            }
          >
            {isCleaningLeftovers ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t(
                  "central.updateCenter.deletedPlatformCopies.cleanupAllApplying",
                )}
              </>
            ) : (
              <>
                <Trash2 className="size-3.5" />
                {t("central.updateCenter.deletedPlatformCopies.cleanupAll", {
                  count: deletedPlatformCopyPathCount,
                })}
              </>
            )}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={handleForceMirrorRepositories}
            disabled={
              isApplying ||
              isRefreshing ||
              isForcing ||
              scopeKind !== "repositories" ||
              refreshContext.repositoryIds.length === 0
            }
          >
            {t("central.updateCenter.forceMirrorRepositories")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={closeDialog}
            disabled={isApplying || isForcing}
          >
            {t("common.cancel")}
          </Button>
          <Button
            size="sm"
            onClick={handleApplySelected}
            disabled={
              isApplying || isForcing || totalSelected === 0 || !inventory
            }
          >
            {(isApplying && !isCleaningLeftovers) || isForcing ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t(
                  isForcing
                    ? "central.updateCenter.forcingChanges"
                    : "central.updateCenter.applyingChanges",
                )}
              </>
            ) : (
              t("central.updateCenter.applySelected", { count: totalSelected })
            )}
          </Button>
        </DialogFooter>
        <p className="px-6 pb-4 text-xs text-muted-foreground">
          {t("central.updateCenter.selectionSummary", {
            ...selectionSummary,
          })}
        </p>
      </DialogContent>
    </Dialog>
  );
}
