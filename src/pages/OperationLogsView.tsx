import { FormEvent, useEffect, useState } from "react";
import {
  Download,
  Loader2,
  RefreshCw,
  RotateCcw,
  Search,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { OperationLogDetailDrawer } from "@/components/logs/OperationLogDetailDrawer";
import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Input } from "@/components/ui/input";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { OperationLogEntry, OperationLogFilter } from "@/types";
import { cn } from "@/lib/utils";

function formatDateTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function statusClass(status: string): string {
  switch (status) {
    case "succeeded":
      return "bg-emerald-500/10 text-emerald-600";
    case "failed":
      return "bg-destructive/10 text-destructive";
    case "partial":
      return "bg-amber-500/10 text-amber-600";
    default:
      return "bg-muted text-muted-foreground";
  }
}

function levelClass(level: string): string {
  switch (level) {
    case "error":
      return "text-destructive";
    case "warn":
      return "text-amber-600";
    default:
      return "text-muted-foreground";
  }
}

function downloadJson(payload: string) {
  const now = new Date();
  const timestamp = now
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "");
  const blob = new Blob([payload], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `skillport-operation-logs-${timestamp}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function toDateInput(value?: string): string {
  return value?.slice(0, 10) ?? "";
}

function fromStartDateInput(value: string): string | undefined {
  return value ? `${value}T00:00:00Z` : undefined;
}

function fromEndDateInput(value: string): string | undefined {
  return value ? `${value}T23:59:59Z` : undefined;
}

export function OperationLogsView() {
  const { t } = useTranslation();
  const entries = useOperationLogStore((s) => s.entries);
  const total = useOperationLogStore((s) => s.total);
  const filter = useOperationLogStore((s) => s.filter);
  const selectedEntry = useOperationLogStore((s) => s.selectedEntry);
  const isLoading = useOperationLogStore((s) => s.isLoading);
  const isClearing = useOperationLogStore((s) => s.isClearing);
  const isExporting = useOperationLogStore((s) => s.isExporting);
  const error = useOperationLogStore((s) => s.error);
  const loadLogs = useOperationLogStore((s) => s.loadLogs);
  const loadLogDetail = useOperationLogStore((s) => s.loadLogDetail);
  const setFilter = useOperationLogStore((s) => s.setFilter);
  const clearFilters = useOperationLogStore((s) => s.clearFilters);
  const clearLogs = useOperationLogStore((s) => s.clearLogs);
  const exportLogs = useOperationLogStore((s) => s.exportLogs);
  const closeDetail = useOperationLogStore((s) => s.closeDetail);
  const [query, setQuery] = useState(filter.query ?? "");

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  function updateFilter(partial: Partial<OperationLogFilter>) {
    const nextFilter = { ...filter, ...partial, offset: 0 };
    setFilter(partial);
    loadLogs(nextFilter).catch((err) => {
      toast.error(t("logs.loadError", { error: String(err) }));
    });
  }

  function handleSearch(event: FormEvent) {
    event.preventDefault();
    updateFilter({ query });
  }

  async function handleOpenDetail(entry: OperationLogEntry) {
    try {
      await loadLogDetail(entry.id);
    } catch (err) {
      toast.error(t("logs.detailError", { error: String(err) }));
    }
  }

  async function handleExport() {
    try {
      const payload = await exportLogs();
      downloadJson(payload);
      toast.success(t("logs.exported"));
    } catch (err) {
      toast.error(t("logs.exportError", { error: String(err) }));
    }
  }

  async function handleClear() {
    try {
      const deleted = await clearLogs();
      toast.success(t("logs.cleared", { count: deleted }));
    } catch (err) {
      toast.error(t("logs.clearError", { error: String(err) }));
    }
  }

  function handleResetFilters() {
    clearFilters();
    setQuery("");
    loadLogs({ query: undefined, offset: 0 }).catch((err) => {
      toast.error(t("logs.loadError", { error: String(err) }));
    });
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b border-border px-5 py-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">{t("logs.title")}</h1>
            <p className="mt-1 text-sm text-muted-foreground">{t("logs.description")}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => loadLogs().catch((err) => toast.error(String(err)))}
              disabled={isLoading}
            >
              {isLoading ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <RefreshCw className="size-4" />
              )}
              {t("common.refresh")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleExport}
              disabled={isExporting || isLoading}
            >
              {isExporting ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Download className="size-4" />
              )}
              {t("common.export")}
            </Button>
            <InlineConfirmAction
              idleAriaLabel={t("logs.clear")}
              idleTitle={t("logs.clear")}
              confirmLabel={t("logs.confirmClear")}
              onConfirm={handleClear}
              icon={<Trash2 className="size-4" />}
              disabled={isClearing || isLoading}
              isLoading={isClearing}
            />
          </div>
        </div>

        <form
          onSubmit={handleSearch}
          className="mt-4 grid gap-2 lg:grid-cols-[minmax(15rem,1.4fr)_repeat(6,minmax(8rem,0.7fr))_auto]"
        >
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("logs.searchPlaceholder")}
              className="pl-8"
            />
          </div>
          <select
            value={filter.targetKind ?? ""}
            onChange={(event) => updateFilter({ targetKind: event.target.value as never })}
            aria-label={t("logs.filters.target")}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            <option value="">{t("logs.filters.allTargets")}</option>
            <option value="local">{t("logs.targets.local")}</option>
            <option value="ssh">{t("logs.targets.ssh")}</option>
          </select>
          <select
            value={filter.level ?? ""}
            onChange={(event) => updateFilter({ level: event.target.value as never })}
            aria-label={t("logs.filters.level")}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            <option value="">{t("logs.filters.allLevels")}</option>
            <option value="info">info</option>
            <option value="warn">warn</option>
            <option value="error">error</option>
          </select>
          <select
            value={filter.status ?? ""}
            onChange={(event) => updateFilter({ status: event.target.value as never })}
            aria-label={t("logs.filters.status")}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            <option value="">{t("logs.filters.allStatuses")}</option>
            <option value="succeeded">{t("logs.status.succeeded")}</option>
            <option value="failed">{t("logs.status.failed")}</option>
            <option value="partial">{t("logs.status.partial")}</option>
            <option value="cancelled">{t("logs.status.cancelled")}</option>
          </select>
          <Input
            value={filter.action ?? ""}
            onChange={(event) => updateFilter({ action: event.target.value })}
            placeholder={t("logs.filters.action")}
            aria-label={t("logs.filters.action")}
          />
          <Input
            type="date"
            value={toDateInput(filter.createdAfter)}
            onChange={(event) => updateFilter({ createdAfter: fromStartDateInput(event.target.value) })}
            aria-label={t("logs.filters.from")}
          />
          <Input
            type="date"
            value={toDateInput(filter.createdBefore)}
            onChange={(event) => updateFilter({ createdBefore: fromEndDateInput(event.target.value) })}
            aria-label={t("logs.filters.to")}
          />
          <Button type="submit" size="sm" disabled={isLoading}>
            {t("common.search")}
          </Button>
        </form>
        <div className="mt-2 flex items-center justify-between text-xs text-muted-foreground">
          <span>{t("logs.total", { count: total })}</span>
          <button
            type="button"
            onClick={handleResetFilters}
            className="inline-flex items-center gap-1 rounded px-2 py-1 hover:bg-muted"
          >
            <RotateCcw className="size-3" />
            {t("logs.resetFilters")}
          </button>
        </div>
      </div>

      {error && (
        <div className="mx-5 mt-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-auto p-5">
        <div className="overflow-hidden rounded-xl border border-border">
          <div className="grid grid-cols-[9rem_5rem_minmax(8rem,1fr)_minmax(8rem,1fr)_7rem] gap-3 border-b border-border bg-muted/40 px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            <div>{t("logs.columns.time")}</div>
            <div>{t("logs.columns.level")}</div>
            <div>{t("logs.columns.target")}</div>
            <div>{t("logs.columns.action")}</div>
            <div>{t("logs.columns.result")}</div>
          </div>

          {isLoading && entries.length === 0 ? (
            <div className="flex h-44 items-center justify-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              {t("logs.loading")}
            </div>
          ) : entries.length === 0 ? (
            <div className="flex h-44 items-center justify-center text-sm text-muted-foreground">
              {t("logs.empty")}
            </div>
          ) : (
            entries.map((entry) => (
              <button
                key={entry.id}
                type="button"
                onClick={() => handleOpenDetail(entry)}
                className="grid w-full grid-cols-[9rem_5rem_minmax(8rem,1fr)_minmax(8rem,1fr)_7rem] gap-3 border-b border-border px-3 py-3 text-left text-sm transition-colors last:border-b-0 hover:bg-muted/40"
              >
                <span className="text-muted-foreground">{formatDateTime(entry.createdAt)}</span>
                <span className={cn("font-medium", levelClass(entry.level))}>{entry.level}</span>
                <span className="min-w-0">
                  <span className="block truncate">{entry.targetLabel ?? entry.targetId}</span>
                  <span className="text-xs text-muted-foreground">{entry.targetKind}</span>
                </span>
                <span className="min-w-0">
                  <span className="block truncate font-mono text-xs">{entry.action}</span>
                  <span className="block truncate text-xs text-muted-foreground">
                    {entry.summary}
                  </span>
                </span>
                <span
                  className={cn(
                    "inline-flex h-6 items-center justify-center rounded-full px-2 text-xs font-medium",
                    statusClass(entry.status)
                  )}
                >
                  {t(`logs.status.${entry.status}`, { defaultValue: entry.status })}
                </span>
              </button>
            ))
          )}
        </div>
      </div>

      <OperationLogDetailDrawer
        open={selectedEntry !== null}
        entry={selectedEntry}
        onOpenChange={(open) => {
          if (!open) closeDetail();
        }}
      />
    </div>
  );
}
