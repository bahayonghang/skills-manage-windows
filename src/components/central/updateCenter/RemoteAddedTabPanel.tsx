import { useTranslation } from "react-i18next";

import { Checkbox } from "@/components/ui/checkbox";
import type { DuplicateResolution } from "@/types";
import type { RemoteAddedSkill } from "@/types/skillUpdateInventory";
import { remoteAddedKey } from "@/components/central/updateCenter/keys";
import { SourceMeta } from "@/components/central/updateCenter/SourceMeta";
import {
  formatConflictSourceLabel,
  type RepositorySourceDisplayInfo,
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
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
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
  repositorySources,
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
        const source = repositorySources.get(item.repositoryId);
        const repositoryLabel = source?.label ?? item.repositoryId;
        const remoteSource = formatConflictSourceLabel(
          repositoryLabel,
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
          <article
            key={key}
            className="rounded-xl bg-background/80 p-3 ring-1 ring-border/80"
          >
            <div className="flex flex-wrap items-start gap-3">
              <Checkbox
                checked={decision.selected}
                onCheckedChange={(value) =>
                  onChange(key, { selected: !!value })
                }
                aria-label={item.skillName}
              />
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium text-foreground">
                  {item.skillName}
                </div>
                <SourceMeta
                  repositoryLabel={repositoryLabel}
                  sourcePath={item.sourcePath}
                  sourceUrl={source?.url}
                />
                {hasConflict && (
                  <div className="mt-1 text-xs text-warning-foreground">
                    {t("central.repositorySyncConflict", {
                      remoteSource,
                      skill:
                        existingConflict?.skillName ??
                        item.conflictExistingSkillId,
                      existingSource,
                    })}
                  </div>
                )}
              </div>
              <div className="flex w-full flex-wrap justify-end gap-1 text-xs sm:w-auto">
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
