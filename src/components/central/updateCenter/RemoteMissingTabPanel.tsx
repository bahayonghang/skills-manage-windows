import { useTranslation } from "react-i18next";

import { SourceMeta } from "@/components/central/updateCenter/SourceMeta";
import type { RemoteMissingSkill } from "@/types/skillUpdateInventory";
import type { RepositorySourceDisplayInfo } from "@/lib/centralConflictSource";

export type RemoteMissingDecisionValue = "keep" | "delete";

export interface RemoteMissingRowState {
  decision: RemoteMissingDecisionValue;
  removeAgentIds: string[];
}

interface RemoteMissingTabPanelProps {
  items: RemoteMissingSkill[];
  state: Record<string, RemoteMissingRowState>;
  repositorySources: ReadonlyMap<string, RepositorySourceDisplayInfo>;
  onChange: (skillId: string, patch: Partial<RemoteMissingRowState>) => void;
}

const CHOICES: readonly RemoteMissingDecisionValue[] = ["keep", "delete"] as const;

export function RemoteMissingTabPanel({
  items,
  state,
  repositorySources,
  onChange,
}: RemoteMissingTabPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="max-h-[28rem] space-y-2 overflow-auto pr-1">
      {items.map((item) => {
        const skillId = item.state.skill_id;
        const decision = state[skillId]?.decision ?? "keep";
        const source = item.repositoryId
          ? repositorySources.get(item.repositoryId)
          : null;
        const repositoryLabel = source?.label ?? item.repositoryId;
        return (
          <article key={skillId} className="rounded-xl bg-background/80 p-3 ring-1 ring-border/80">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium text-foreground">
                  {skillId}
                </div>
                <SourceMeta
                  repositoryLabel={repositoryLabel}
                  sourcePath={item.state.source_path}
                  sourceUrl={item.state.source_url ?? source?.url}
                />
              </div>
              <div className="grid w-full grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs sm:w-auto">
                {CHOICES.map((next) => (
                  <button
                    key={next}
                    type="button"
                    className={`rounded-lg px-3 py-1.5 font-medium ${
                      decision === next
                        ? next === "delete"
                          ? "bg-destructive text-destructive-foreground"
                          : "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:bg-background"
                    }`}
                    onClick={() => onChange(skillId, { decision: next })}
                  >
                    {t(`central.updateCenter.actions.${next}`)}
                  </button>
                ))}
              </div>
            </div>
          </article>
        );
      })}
    </div>
  );
}
