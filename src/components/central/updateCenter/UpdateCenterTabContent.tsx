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
import { OrphansTabPanel } from "@/components/central/updateCenter/OrphansTabPanel";
import {
  countsFromInventory,
  type DecisionState,
} from "@/components/central/updateCenter/decisionAggregation";

import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type { SkillUpdateInventory } from "@/types/skillUpdateInventory";

export interface UpdateCenterTabHandlers {
  updateUpdatable: (skillId: string, patch: Partial<UpdatableRowState>) => void;
  toggleAllUpdatable: (selected: boolean) => void;
  updateAdded: (key: string, patch: Partial<RemoteAddedRowState>) => void;
  updateMissing: (skillId: string, patch: Partial<RemoteMissingRowState>) => void;
  updateDuplicates: (
    key: string,
    patch: Partial<PlatformDuplicateRowState>,
  ) => void;
}

interface UpdateCenterTabContentProps {
  tab: UpdateCenterTab;
  inventory: SkillUpdateInventory | null;
  decisions: DecisionState;
  handlers: UpdateCenterTabHandlers;
}

export function UpdateCenterTabContent({
  tab,
  inventory,
  decisions,
  handlers,
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
          onChange={handlers.updateUpdatable}
          onToggleAll={handlers.toggleAllUpdatable}
        />
      );
    case "added":
      return (
        <RemoteAddedTabPanel
          items={inventory.remoteAdded}
          state={decisions.added}
          onChange={handlers.updateAdded}
        />
      );
    case "missing":
      return (
        <RemoteMissingTabPanel
          items={inventory.remoteMissing}
          state={decisions.missing}
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
    default:
      return null;
  }
}
