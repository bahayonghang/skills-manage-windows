import { useTranslation } from "react-i18next";

import { OperationLogFilter } from "@/types";
import { cn } from "@/lib/utils";

interface LogsQuickFiltersProps {
  filter: OperationLogFilter;
  onApply: (partial: Partial<OperationLogFilter>) => void;
}

function QuickChip({
  active,
  onClick,
  label,
  testId,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  testId?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      data-testid={testId}
      className={cn(
        "inline-flex h-7 items-center rounded-full border px-3 text-xs font-medium transition-colors",
        active
          ? "border-primary/40 bg-primary/15 text-primary"
          : "border-border bg-background text-muted-foreground hover:border-primary/40 hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}

function todayBounds(): { start: string; end: string } {
  const start = new Date();
  start.setHours(0, 0, 0, 0);
  const end = new Date(start);
  end.setDate(start.getDate() + 1);
  end.setMilliseconds(end.getMilliseconds() - 1);
  return { start: start.toISOString(), end: end.toISOString() };
}

function nowMinusHoursIso(hours: number): string {
  return new Date(Date.now() - hours * 60 * 60 * 1000).toISOString();
}

export function LogsQuickFilters({ filter, onApply }: LogsQuickFiltersProps) {
  const { t } = useTranslation();
  const isFailed = filter.status === "failed";
  const isLocal = filter.targetKind === "local";
  const isSsh = filter.targetKind === "ssh";
  const isWsl = filter.targetKind === "wsl";

  return (
    <div
      role="group"
      aria-label={t("logs.quickFilters.label")}
      className="flex flex-wrap items-center gap-2"
    >
      <span className="text-[0.68rem] font-medium uppercase tracking-wide text-muted-foreground">
        {t("logs.quickFilters.label")}
      </span>
      <QuickChip
        testId="logs-chip-today"
        active={false}
        onClick={() => {
          const bounds = todayBounds();
          onApply({
            createdAfter: bounds.start,
            createdBefore: bounds.end,
          });
        }}
        label={t("logs.quickFilters.today")}
      />
      <QuickChip
        testId="logs-chip-24h"
        active={false}
        onClick={() =>
          onApply({
            createdAfter: nowMinusHoursIso(24),
            createdBefore: undefined,
          })
        }
        label={t("logs.quickFilters.last24h")}
      />
      <QuickChip
        testId="logs-chip-7d"
        active={false}
        onClick={() =>
          onApply({
            createdAfter: nowMinusHoursIso(24 * 7),
            createdBefore: undefined,
          })
        }
        label={t("logs.quickFilters.last7days")}
      />
      <span className="mx-1 h-4 w-px bg-border" aria-hidden="true" />
      <QuickChip
        testId="logs-chip-failed"
        active={isFailed}
        onClick={() =>
          onApply({ status: isFailed ? undefined : "failed" })
        }
        label={t("logs.quickFilters.failed")}
      />
      <span className="mx-1 h-4 w-px bg-border" aria-hidden="true" />
      <QuickChip
        testId="logs-chip-local"
        active={isLocal}
        onClick={() =>
          onApply({ targetKind: isLocal ? undefined : "local" })
        }
        label={t("logs.quickFilters.local")}
      />
      <QuickChip
        testId="logs-chip-ssh"
        active={isSsh}
        onClick={() =>
          onApply({ targetKind: isSsh ? undefined : "ssh" })
        }
        label={t("logs.quickFilters.ssh")}
      />
      <QuickChip
        testId="logs-chip-wsl"
        active={isWsl}
        onClick={() =>
          onApply({ targetKind: isWsl ? undefined : "wsl" })
        }
        label={t("logs.quickFilters.wsl")}
      />
    </div>
  );
}
