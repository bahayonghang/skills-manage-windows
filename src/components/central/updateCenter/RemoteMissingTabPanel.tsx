import { useTranslation } from "react-i18next";

import type { RemoteMissingSkill } from "@/types/skillUpdateInventory";

export type RemoteMissingDecisionValue = "keep" | "delete";

export interface RemoteMissingRowState {
  decision: RemoteMissingDecisionValue;
  removeAgentIds: string[];
}

interface RemoteMissingTabPanelProps {
  items: RemoteMissingSkill[];
  state: Record<string, RemoteMissingRowState>;
  onChange: (skillId: string, patch: Partial<RemoteMissingRowState>) => void;
}

const CHOICES: readonly RemoteMissingDecisionValue[] = ["keep", "delete"] as const;

export function RemoteMissingTabPanel({
  items,
  state,
  onChange,
}: RemoteMissingTabPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="max-h-[28rem] space-y-2 overflow-auto pr-1">
      {items.map((item) => {
        const skillId = item.state.skill_id;
        const decision = state[skillId]?.decision ?? "keep";
        return (
          <article key={skillId} className="rounded-xl border border-border bg-background p-3">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium text-foreground">
                  {skillId}
                </div>
                <div className="mt-1 space-y-0.5 text-xs text-muted-foreground">
                  <div>
                    {item.repositoryId
                      ? t("central.updateCenter.repositorySource", {
                          repository: item.repositoryId,
                        })
                      : t("central.updateCenter.repositorySourceUnknown")}
                  </div>
                  {item.state.source_path && (
                    <div>
                      {t("central.updateCenter.sourcePath", {
                        path: item.state.source_path,
                      })}
                    </div>
                  )}
                </div>
              </div>
              <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs">
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
