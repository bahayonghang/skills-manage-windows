import { useTranslation } from "react-i18next";

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
import {
  countsFromInventory,
  type DecisionState,
} from "@/components/central/updateCenter/decisionAggregation";

import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type { SkillUpdateInventory } from "@/types/skillUpdateInventory";
import type {
  RepositorySourceDisplayInfo,
  SkillConflictSourceInfo,
} from "@/lib/centralConflictSource";

export interface UpdateCenterTabHandlers {
  updateUpdatable: (skillId: string, patch: Partial<UpdatableRowState>) => void;
  toggleAllUpdatable: (selected: boolean) => void;
  updateAdded: (key: string, patch: Partial<RemoteAddedRowState>) => void;
  updateMissing: (skillId: string, patch: Partial<RemoteMissingRowState>) => void;
  updateDuplicates: (
    key: string,
    patch: Partial<PlatformDuplicateRowState>,
  ) => void;
  updateDeletedPlatformCopies: (
    key: string,
    patch: Partial<DeletedPlatformCopyRowState>,
  ) => void;
}

interface UpdateCenterTabContentProps {
  tab: UpdateCenterTab;
  inventory: SkillUpdateInventory | null;
  decisions: DecisionState;
  handlers: UpdateCenterTabHandlers;
  existingSkillSources: ReadonlyMap<string, SkillConflictSourceInfo>;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
}

export function UpdateCenterTabContent({
  tab,
  inventory,
  decisions,
  handlers,
  existingSkillSources,
  repositorySources,
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
    return <FailedRepositoriesPanel inventory={inventory} repositorySources={repositorySources} />;
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
}: {
  inventory: SkillUpdateInventory;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
}) {
  const { t } = useTranslation();
  if (inventory.failedRepositories.length === 0) {
    return (
      <p className="text-muted-foreground">
        {t("central.updateCenter.tabEmpty")}
      </p>
    );
  }
  return (
    <div className="space-y-2">
      {inventory.failedRepositories.map((item) => (
        <div
          key={`${item.repositoryId}:${item.error}`}
          className="rounded-lg border border-destructive/30 bg-destructive/5 p-3"
        >
          <div className="text-sm font-medium">
            {repositorySources.get(item.repositoryId)?.label ?? item.repositoryId}
          </div>
          <p className="mt-1 text-xs text-destructive-text">{item.error}</p>
        </div>
      ))}
    </div>
  );
}
