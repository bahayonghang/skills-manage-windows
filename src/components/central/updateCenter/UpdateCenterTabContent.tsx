import { useTranslation } from "react-i18next";
import { Loader2, RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  UpdatableTabPanel,
  type UpdatableRowState,
} from "@/components/central/updateCenter/UpdatableTabPanel";
import {
  RemoteAddedTabPanel,
  type RemoteAddedRowState,
} from "@/components/central/updateCenter/RemoteAddedTabPanel";
import {
  RemoteMissingTabPanel,
  type RemoteMissingRowState,
} from "@/components/central/updateCenter/RemoteMissingTabPanel";
import {
  PlatformDuplicatesTabPanel,
  type PlatformDuplicateRowState,
} from "@/components/central/updateCenter/PlatformDuplicatesTabPanel";
import {
  DeletedPlatformCopiesTabPanel,
  type DeletedPlatformCopyRowState,
} from "@/components/central/updateCenter/DeletedPlatformCopiesTabPanel";
import { OrphansTabPanel } from "@/components/central/updateCenter/OrphansTabPanel";
import { UnsupportedTabPanel } from "@/components/central/updateCenter/UnsupportedTabPanel";
import {
  countsFromInventory,
  type DecisionState,
} from "@/components/central/updateCenter/decisionAggregation";

import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type { TFunction } from "i18next";

import type {
  FailedRepository,
  SkillRefreshMode,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";
import type {
  RepositorySourceDisplayInfo,
  SkillConflictSourceInfo,
} from "@/lib/centralConflictSource";

/**
 * Prefer the backend's stable code so the reason is localized; fall back to the
 * stored sentence for reconciliation reasons that carry their own text and for
 * inventories persisted before codes existed.
 */
function failedRepositoryReason(item: FailedRepository, t: TFunction): string {
  if (!item.errorCode) return item.error;
  const translated = t(`backendErrors.${item.errorCode}`, {
    defaultValue: "",
  }) as string;
  return translated || item.error;
}

export interface UpdateCenterTabHandlers {
  updateUpdatable: (skillId: string, patch: Partial<UpdatableRowState>) => void;
  toggleAllUpdatable: (selected: boolean) => void;
  updateAdded: (key: string, patch: Partial<RemoteAddedRowState>) => void;
  updateMissing: (
    skillId: string,
    patch: Partial<RemoteMissingRowState>,
  ) => void;
  updateDuplicates: (
    key: string,
    patch: Partial<PlatformDuplicateRowState>,
  ) => void;
  updateDeletedPlatformCopies: (
    key: string,
    patch: Partial<DeletedPlatformCopyRowState>,
  ) => void;
  retryRepositories: (repositoryIds: string[], mode?: SkillRefreshMode) => void;
}

interface UpdateCenterTabContentProps {
  tab: UpdateCenterTab;
  inventory: SkillUpdateInventory | null;
  decisions: DecisionState;
  handlers: UpdateCenterTabHandlers;
  existingSkillSources: ReadonlyMap<string, SkillConflictSourceInfo>;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
  retryingRepositoryIds: readonly string[];
  actionsDisabled: boolean;
}

export function UpdateCenterTabContent({
  tab,
  inventory,
  decisions,
  handlers,
  existingSkillSources,
  repositorySources,
  retryingRepositoryIds,
  actionsDisabled,
}: UpdateCenterTabContentProps) {
  const { t } = useTranslation();

  if (!inventory) {
    return (
      <p className="text-muted-foreground">
        {t("central.updateCenter.emptyAllClean")}
      </p>
    );
  }

  if (tab === "orphans") {
    return <OrphansTabPanel />;
  }

  if (tab === "failed") {
    return (
      <FailedRepositoriesPanel
        inventory={inventory}
        repositorySources={repositorySources}
        onRetry={handlers.retryRepositories}
        retryingRepositoryIds={retryingRepositoryIds}
        disabled={actionsDisabled}
      />
    );
  }

  const counts = countsFromInventory(inventory);
  if (counts[tab] === 0) {
    return (
      <p className="text-muted-foreground">
        {t("central.updateCenter.tabEmpty")}
      </p>
    );
  }

  switch (tab) {
    case "updatable":
      return (
        <UpdatableTabPanel
          items={inventory.updatable}
          state={decisions.updatable}
          repositorySources={repositorySources}
          onChange={handlers.updateUpdatable}
          onToggleAll={handlers.toggleAllUpdatable}
        />
      );
    case "added":
      return (
        <RemoteAddedTabPanel
          items={inventory.remoteAdded}
          state={decisions.added}
          existingSkillSources={existingSkillSources}
          repositorySources={repositorySources}
          onChange={handlers.updateAdded}
        />
      );
    case "missing":
      return (
        <RemoteMissingTabPanel
          items={inventory.remoteMissing}
          state={decisions.missing}
          repositorySources={repositorySources}
          onChange={handlers.updateMissing}
        />
      );
    case "unsupported":
      return <UnsupportedTabPanel items={inventory.unsupported ?? []} />;
    case "duplicates":
      return (
        <PlatformDuplicatesTabPanel
          items={inventory.platformDuplicates}
          state={decisions.duplicates}
          onChange={handlers.updateDuplicates}
        />
      );
    case "deletedPlatformCopies":
      return (
        <DeletedPlatformCopiesTabPanel
          items={inventory.deletedPlatformCopies ?? []}
          state={decisions.deletedPlatformCopies}
          onChange={handlers.updateDeletedPlatformCopies}
        />
      );
    default:
      return null;
  }
}

function FailedRepositoriesPanel({
  inventory,
  repositorySources,
  onRetry,
  retryingRepositoryIds,
  disabled,
}: {
  inventory: SkillUpdateInventory;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
  onRetry: (repositoryIds: string[], mode?: SkillRefreshMode) => void;
  retryingRepositoryIds: readonly string[];
  disabled: boolean;
}) {
  const { t } = useTranslation();
  if (inventory.failedRepositories.length === 0) {
    return (
      <p className="text-muted-foreground">
        {t("central.updateCenter.tabEmpty")}
      </p>
    );
  }

  const retryableIds = [
    ...new Set(
      inventory.failedRepositories
        .filter((item) => item.retry === "retryable")
        .map((item) => item.repositoryId),
    ),
  ];

  return (
    <div className="space-y-2">
      <div className="flex justify-end">
        <Button
          size="sm"
          variant="outline"
          disabled={disabled || retryableIds.length === 0}
          onClick={() => onRetry(retryableIds)}
        >
          <RefreshCw className="size-3.5" />
          {t("central.updateCenter.failed.retryAll", {
            count: retryableIds.length,
          })}
        </Button>
      </div>
      {inventory.failedRepositories.map((item) => {
        const isRetrying = retryingRepositoryIds.includes(item.repositoryId);
        return (
          <div
            key={`${item.repositoryId}:${item.errorCode ?? item.error}`}
            className="rounded-lg border border-destructive/30 bg-destructive/5 p-3"
          >
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="text-sm font-medium">
                  {repositorySources.get(item.repositoryId)?.label ??
                    item.repositoryId}
                </div>
                <p className="mt-1 text-xs text-destructive-text">
                  {failedRepositoryReason(item, t)}
                </p>
              </div>
              <FailedRepositoryAction
                item={item}
                isRetrying={isRetrying}
                disabled={disabled}
                onRetry={onRetry}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

/**
 * A transient failure is retried in place; a vanished source path is re-checked
 * in incremental mode so the skill reaches the removal decision bucket. Rows
 * from inventories stored before the classification existed offer no action.
 */
function FailedRepositoryAction({
  item,
  isRetrying,
  disabled,
  onRetry,
}: {
  item: FailedRepository;
  isRetrying: boolean;
  disabled: boolean;
  onRetry: (repositoryIds: string[], mode?: SkillRefreshMode) => void;
}) {
  const { t } = useTranslation();
  if (item.retry !== "retryable" && item.retry !== "decision_required") {
    return null;
  }
  const isDecision = item.retry === "decision_required";
  return (
    <Button
      size="sm"
      variant="outline"
      className="shrink-0"
      disabled={disabled || isRetrying}
      onClick={() =>
        onRetry([item.repositoryId], isDecision ? "sync" : undefined)
      }
    >
      {isRetrying ? (
        <Loader2 className="size-3.5 animate-spin" />
      ) : (
        <RefreshCw className="size-3.5" />
      )}
      {t(
        isDecision
          ? "central.updateCenter.failed.recheckWithSync"
          : "central.updateCenter.failed.retry",
      )}
    </Button>
  );
}
