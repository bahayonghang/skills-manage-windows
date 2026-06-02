import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import type { UpdateCheckMode } from "@/pages/centralUpdateCheckMode";

interface UpdateModePreferenceSelectProps {
  mode: UpdateCheckMode;
  disabled: boolean;
  syncDisabled: boolean;
  syncDisabledReason: string;
  onChange: (mode: UpdateCheckMode) => void;
}

export function UpdateModePreferenceSelect({
  mode,
  disabled,
  syncDisabled,
  syncDisabledReason,
  onChange,
}: UpdateModePreferenceSelectProps) {
  const { t } = useTranslation();
  const title = syncDisabled ? syncDisabledReason : t("central.updateCheckMode.inlineHint");

  return (
    <label
      className={cn(
        "inline-flex h-9 shrink-0 items-center gap-2 rounded-lg border border-border bg-background px-2 text-xs text-muted-foreground",
        disabled && "opacity-60",
      )}
      title={title}
    >
      <span className="whitespace-nowrap font-medium">
        {t("central.updateCheckMode.inlineLabel")}
      </span>
      <select
        data-testid="central-update-check-mode-select"
        className="h-7 rounded-md border border-border bg-card px-2 text-xs font-medium text-foreground outline-none focus:ring-2 focus:ring-ring"
        value={mode}
        disabled={disabled}
        onChange={(event) => {
          const next = event.target.value as UpdateCheckMode;
          if (next === "sync" && syncDisabled) {
            return;
          }
          onChange(next);
        }}
      >
        <option value="regular">
          {t("central.updateCheckMode.regular.title")}
        </option>
        <option value="sync" disabled={syncDisabled}>
          {t("central.updateCheckMode.sync.title")}
        </option>
      </select>
    </label>
  );
}
