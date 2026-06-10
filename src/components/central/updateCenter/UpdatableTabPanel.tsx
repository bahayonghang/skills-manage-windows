import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { SourceMeta } from "@/components/central/updateCenter/SourceMeta";
import type { UpdatableSkill } from "@/types/skillUpdateInventory";
import type { RepositorySourceDisplayInfo } from "@/lib/centralConflictSource";

export interface UpdatableRowState {
  selected: boolean;
}

interface UpdatableTabPanelProps {
  items: UpdatableSkill[];
  state: Record<string, UpdatableRowState>;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
  onChange: (skillId: string, patch: Partial<UpdatableRowState>) => void;
  onToggleAll: (selected: boolean) => void;
}

export function UpdatableTabPanel({
  items,
  state,
  repositorySources,
  onChange,
  onToggleAll,
}: UpdatableTabPanelProps) {
  const { t } = useTranslation();
  const selectedCount = useMemo(
    () => items.filter((item) => state[item.state.skill_id]?.selected).length,
    [items, state],
  );
  const allSelected = selectedCount === items.length && items.length > 0;

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-muted/30 px-3 py-2">
        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onToggleAll(true)}
            disabled={allSelected}
          >
            {t("central.updateCenter.selectAll")}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onToggleAll(false)}
            disabled={selectedCount === 0}
          >
            {t("central.updateCenter.deselectAll")}
          </Button>
        </div>
        <span className="text-xs text-muted-foreground">
          {t("central.updateCenter.selected", { count: selectedCount })}
        </span>
      </div>

      <div className="max-h-[26rem] space-y-2 overflow-auto pr-1">
        {items.map((item) => {
          const skillId = item.state.skill_id;
          const checked = state[skillId]?.selected ?? false;
          const source = item.repositoryId
            ? repositorySources.get(item.repositoryId)
            : null;
          const repositoryLabel = source?.label ?? item.repositoryId;
          return (
            <label
              key={skillId}
              className="flex cursor-pointer items-start gap-3 rounded-xl bg-background/80 p-3 ring-1 ring-border/80 transition-colors hover:bg-muted/20"
            >
              <Checkbox
                checked={checked}
                onCheckedChange={(value) => onChange(skillId, { selected: !!value })}
                aria-label={skillId}
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-foreground">
                  {skillId}
                </span>
                <SourceMeta
                  repositoryLabel={repositoryLabel}
                  sourcePath={item.state.source_path}
                  sourceUrl={item.state.source_url ?? source?.url}
                />
              </span>
            </label>
          );
        })}
      </div>
    </div>
  );
}
