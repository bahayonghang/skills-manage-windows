import { useTranslation } from "react-i18next";

import { Checkbox } from "@/components/ui/checkbox";
import type { PlatformDuplicateGroup } from "@/types/skillUpdateInventory";
import { duplicateGroupKey } from "@/components/central/updateCenter/keys";

export interface PlatformDuplicateRowState {
  selectedPaths: string[];
}

interface PlatformDuplicatesTabPanelProps {
  items: PlatformDuplicateGroup[];
  state: Record<string, PlatformDuplicateRowState>;
  onChange: (key: string, patch: Partial<PlatformDuplicateRowState>) => void;
}

export function PlatformDuplicatesTabPanel({
  items,
  state,
  onChange,
}: PlatformDuplicatesTabPanelProps) {
  const { t } = useTranslation();

  function togglePath(key: string, current: string[], path: string, checked: boolean) {
    const next = checked
      ? Array.from(new Set([...current, path]))
      : current.filter((entry) => entry !== path);
    onChange(key, { selectedPaths: next });
  }

  return (
    <div className="max-h-[28rem] space-y-2 overflow-auto pr-1">
      {items.map((group) => {
        const key = duplicateGroupKey(group);
        const selected = state[key]?.selectedPaths ?? [];
        return (
          <section
            key={key}
            className="rounded-xl border border-border bg-background p-3"
          >
            <div className="flex items-center justify-between gap-2 text-sm font-medium text-foreground">
              <span>
                {t("central.updateCenter.duplicateGroup", {
                  skill: group.skillName,
                  agent: group.agentId,
                })}
              </span>
              <span className="text-xs text-muted-foreground">
                {t("central.updateCenter.selected", { count: selected.length })}
              </span>
            </div>
            <div className="mt-2 space-y-1">
              {group.writablePaths.map((path) => {
                const checked = selected.includes(path);
                return (
                  <label
                    key={path}
                    className="flex cursor-pointer items-center gap-2 rounded-lg border border-border/70 px-2 py-1.5 text-xs"
                  >
                    <Checkbox
                      checked={checked}
                      onCheckedChange={(value) =>
                        togglePath(key, selected, path, !!value)
                      }
                      aria-label={path}
                    />
                    <span className="min-w-0 flex-1 truncate text-muted-foreground">
                      {path}
                    </span>
                  </label>
                );
              })}
            </div>
            {group.pluginPaths.length > 0 && (
              <p className="mt-2 text-xs text-muted-foreground">
                {t("central.updateCenter.pluginCopiesReadOnly", {
                  paths: group.pluginPaths.join(", "),
                })}
              </p>
            )}
          </section>
        );
      })}
    </div>
  );
}
