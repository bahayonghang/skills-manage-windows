import { useTranslation } from "react-i18next";

import { Checkbox } from "@/components/ui/checkbox";
import type { DuplicateResolution } from "@/types";
import type { RemoteAddedSkill } from "@/types/skillUpdateInventory";
import { remoteAddedKey } from "@/components/central/updateCenter/keys";
import {
  formatConflictSourceLabel,
  type SkillConflictSourceInfo,
} from "@/lib/centralConflictSource";

export interface RemoteAddedRowState {
  selected: boolean;
  resolution: DuplicateResolution;
  renamedSkillId: string;
}

interface RemoteAddedTabPanelProps {
  items: RemoteAddedSkill[];
  state: Record<string, RemoteAddedRowState>;
  existingSkillSources: ReadonlyMap<string, SkillConflictSourceInfo>;
  repositoryLabels: ReadonlyMap<string, string>;
  onChange: (key: string, patch: Partial<RemoteAddedRowState>) => void;
}

const RESOLUTIONS: readonly DuplicateResolution[] = [
  "overwrite",
  "rename",
  "skip",
] as const;

export function RemoteAddedTabPanel({
  items,
  state,
  existingSkillSources,
  repositoryLabels,
  onChange,
}: RemoteAddedTabPanelProps) {
  const { t } = useTranslation();
  const unassignedSourceLabel = t("central.unassignedSource");

  return (
    <div className="max-h-[28rem] space-y-2 overflow-auto pr-1">
      {items.map((item) => {
        const key = remoteAddedKey(item.repositoryId, item.sourcePath);
        const hasConflict = Boolean(item.conflictExistingSkillId);
        const existingConflict = item.conflictExistingSkillId
          ? existingSkillSources.get(item.conflictExistingSkillId)
          : null;
        const remoteSource = formatConflictSourceLabel(
          repositoryLabels.get(item.repositoryId) ?? item.repositoryId,
          item.sourcePath,
          unassignedSourceLabel,
        );
        const existingSource = formatConflictSourceLabel(
          existingConflict?.repositoryLabel,
          existingConflict?.sourcePath,
          unassignedSourceLabel,
        );
        const decision: RemoteAddedRowState = state[key] ?? {
          selected: true,
          resolution: hasConflict ? "skip" : "overwrite",
          renamedSkillId: item.skillId,
        };
        return (
          <article key={key} className="rounded-xl border border-border bg-background p-3">
            <div className="flex items-start gap-3">
              <Checkbox
                checked={decision.selected}
                onCheckedChange={(value) => onChange(key, { selected: !!value })}
                aria-label={item.skillName}
              />
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium text-foreground">
                  {item.skillName}
                </div>
                <div className="text-xs text-muted-foreground">
                  {item.repositoryId} · {item.sourcePath}
                </div>
                {hasConflict && (
                  <div className="mt-1 text-xs text-amber-700 dark:text-amber-300">
                    {t("central.repositorySyncConflict", {
                      remoteSource,
                      skill: existingConflict?.skillName ?? item.conflictExistingSkillId,
                      existingSource,
                    })}
                  </div>
                )}
              </div>
              <div className="flex flex-wrap gap-1 text-xs">
                {RESOLUTIONS.map((resolution) => (
                  <button
                    key={resolution}
                    type="button"
                    className={`rounded-lg border px-2 py-1 ${
                      decision.resolution === resolution
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-border text-muted-foreground"
                    }`}
                    onClick={() => onChange(key, { resolution })}
                  >
                    {t(`central.updateCenter.actions.${resolution}`)}
                  </button>
                ))}
              </div>
            </div>
            {decision.resolution === "rename" && (
              <input
                className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
                value={decision.renamedSkillId}
                onChange={(event) =>
                  onChange(key, { renamedSkillId: event.target.value })
                }
                aria-label={t("central.updateCenter.renameLabel")}
                placeholder={t("central.updateCenter.renameLabel")}
              />
            )}
          </article>
        );
      })}
    </div>
  );
}
