import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
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
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import {
  useUpdateCenterStore,
  type UpdateCenterTab,
} from "@/stores/updateCenterStore";
import type {
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
  buildInitialState,
  countDecisionSelections,
  countsFromInventory,
  emptyDecisionState,
  inventorySignature,
  type DecisionState,
} from "@/components/central/updateCenter/decisionAggregation";
import {
  buildRefreshScope,
  coerceRefreshScopeKind,
  isRefreshScopeEnabled,
} from "@/lib/updateCenterRefreshScope";
import {
  buildRepositoryDisplayNameMap,
  buildSkillConflictSourceMap,
} from "@/lib/centralConflictSource";

const TAB_ORDER: readonly UpdateCenterTab[] = [
  "updatable",
  "added",
  "missing",
  "duplicates",
  "orphans",
] as const;

export function UpdateCenterDialog() {
  const { t } = useTranslation();
  const inventory = useUpdateCenterStore((state) => state.inventory);
  const isDialogOpen = useUpdateCenterStore((state) => state.isDialogOpen);
  const isRefreshing = useUpdateCenterStore((state) => state.isRefreshing);
  const isApplying = useUpdateCenterStore((state) => state.isApplying);
  const activeTab = useUpdateCenterStore((state) => state.activeTab);
  const refreshContext = useUpdateCenterStore((state) => state.refreshContext);
  const lastRefreshedAt = useUpdateCenterStore((state) => state.lastRefreshedAt);
  const error = useUpdateCenterStore((state) => state.error);
  const closeDialog = useUpdateCenterStore((state) => state.closeDialog);
  const refresh = useUpdateCenterStore((state) => state.refresh);
  const apply = useUpdateCenterStore((state) => state.apply);
  const clear = useUpdateCenterStore((state) => state.clear);
  const setActiveTab = useUpdateCenterStore((state) => state.setActiveTab);
  const skills = useCentralSkillsStore((state) => state.skills ?? []);
  const repositories = useCentralSkillsStore((state) => state.repositories ?? []);

  const [scopeKind, setScopeKind] = useState<SkillRefreshScopeKind>("all");
  const [decisions, setDecisions] = useState<DecisionState>(emptyDecisionState);
  const existingSkillSources = useMemo(
    () => buildSkillConflictSourceMap(skills),
    [skills],
  );
  const repositoryLabels = useMemo(
    () => buildRepositoryDisplayNameMap(repositories),
    [repositories],
  );

  const inventoryKey = useMemo(
    () => inventorySignature(inventory),
    [inventory],
  );

  useEffect(() => {
    if (!isDialogOpen) return;
    setDecisions(buildInitialState(inventory));
    // 依赖 inventory 内容签名而非引用，避免无意义重置。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isDialogOpen, inventoryKey]);

  const counts = countsFromInventory(inventory);
  const scopeEnabled = useMemo(
    () => ({
      all: true,
      repositories: isRefreshScopeEnabled("repositories", refreshContext),
      skills: isRefreshScopeEnabled("skills", refreshContext),
    }),
    [refreshContext],
  );
  const totalSelected = useMemo(
    () => countDecisionSelections(decisions, inventory),
    [decisions, inventory],
  );

  useEffect(() => {
    if (!isDialogOpen) return;
    const next = coerceRefreshScopeKind(scopeKind, refreshContext);
    if (next !== scopeKind) {
      setScopeKind(next);
    }
  }, [isDialogOpen, refreshContext, scopeKind]);

  function handleRefresh() {
    const scope: SkillRefreshScope = buildRefreshScope(
      scopeKind,
      refreshContext,
    );
    void refresh(scope);
  }

  function handleClear() {
    void clear();
  }

  async function handleApplyAll() {
    if (!inventory) return;
    const payload = buildDecisions(decisions, inventory);
    try {
      const result = await apply(payload);
      const succeeded =
        result.updatedSkillIds.length
        + result.keptMissingSkillIds.length
        + result.deletedSkillIds.length
        + result.importedSkillIds.length
        + result.skippedAdditions.length
        + result.unskippedAdditions.length
        + result.removedPlatformDuplicatePaths.length;
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
          toast.error(`${failure.step}: ${failure.error}`);
        }
      }
    } catch (err) {
      toast.error(
        t("central.updateCenter.applyError", { error: String(err) }),
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
              <p className="mb-2 text-xs text-destructive" role="alert">
                {error}
              </p>
            ) : null}
            <UpdateCenterTabContent
              tab={activeTab}
              inventory={inventory}
              decisions={decisions}
              handlers={handlers}
              existingSkillSources={existingSkillSources}
              repositoryLabels={repositoryLabels}
            />
          </div>
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            size="sm"
            onClick={handleClear}
            disabled={isApplying || isRefreshing}
          >
            {t("central.updateCenter.clearInventory")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={closeDialog}
            disabled={isApplying}
          >
            {t("common.cancel")}
          </Button>
          <Button
            size="sm"
            onClick={handleApplyAll}
            disabled={isApplying || totalSelected === 0 || !inventory}
          >
            {isApplying ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.updateCenter.applyingChanges")}
              </>
            ) : (
              t("central.updateCenter.applyAll", { count: totalSelected })
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
