import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronUp,
  Download,
  Loader2,
  RefreshCw,
  RotateCcw,
  Rows3,
  Search,
  Terminal,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { LogsActivityCard } from "@/components/logs/LogsActivityCard";
import { LogsEmptyState } from "@/components/logs/LogsEmptyState";
import { LogsKpiRow } from "@/components/logs/LogsKpiRow";
import { LogsListRow } from "@/components/logs/LogsListRow";
import type { LogsListDensity } from "@/components/logs/LogsListRow";
import { LogsListSkeleton } from "@/components/logs/LogsListSkeleton";
import { LogsLoadMore } from "@/components/logs/LogsLoadMore";
import { LogsQuickFilters } from "@/components/logs/LogsQuickFilters";
import { OperationLogDetailDrawer } from "@/components/logs/OperationLogDetailDrawer";
import {
  aggregateLogsKpi,
  dayBoundsIso,
  groupLogsByDay,
} from "@/components/logs/logsUtils";
import { useLogsKeyboard } from "@/components/logs/useLogsKeyboard";
import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Input } from "@/components/ui/input";
import { RuntimeLogsPanel } from "@/components/logs/RuntimeLogsPanel";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { OperationLogEntry, OperationLogFilter } from "@/types";

function downloadText(payload: string, fileName: string, type: string) {
  const blob = new Blob([payload], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function downloadJson(payload: string) {
  const now = new Date();
  const timestamp = now
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "");
  downloadText(
    payload,
    `skillport-operation-logs-${timestamp}.json`,
    "application/json"
  );
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

const LOG_DENSITY_STORAGE_KEY = "skillport.logs.density";

function loadDensity(): LogsListDensity {
  if (typeof window === "undefined") return "comfortable";
  const stored = window.localStorage.getItem(LOG_DENSITY_STORAGE_KEY);
  return stored === "compact" ? "compact" : "comfortable";
}

function OperationLogsPanel() {
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
  const loadMore = useOperationLogStore((s) => s.loadMore);
  const [query, setQuery] = useState(filter.query ?? "");
  const [advancedManual, setAdvancedManual] = useState(false);
  const showAdvanced =
    advancedManual ||
    Boolean(filter.action || filter.createdAfter || filter.createdBefore);
  const [density, setDensity] = useState<LogsListDensity>(() => loadDensity());

  const searchContainerRef = useRef<HTMLDivElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const rowsContainerRef = useRef<HTMLDivElement | null>(null);

  const kpi = useMemo(() => aggregateLogsKpi(entries), [entries]);
  const groups = useMemo(() => groupLogsByDay(entries), [entries]);
  const hasFilters = Boolean(
    filter.query ||
      filter.targetKind ||
      filter.level ||
      filter.status ||
      filter.action ||
      filter.createdAfter ||
      filter.createdBefore,
  );

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(LOG_DENSITY_STORAGE_KEY, density);
  }, [density]);

  useEffect(() => {
    searchInputRef.current =
      searchContainerRef.current?.querySelector<HTMLInputElement>("input") ??
      null;
  }, []);

  useLogsKeyboard({
    searchInputRef,
    rowsContainerRef,
    onRefresh: () => {
      loadLogs().catch((err) => toast.error(String(err)));
    },
    onExport: () => {
      handleExport();
    },
  });

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
              variant="ghost"
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
              variant="ghost"
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
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                setDensity((current) =>
                  current === "compact" ? "comfortable" : "compact",
                )
              }
              aria-label={t("logs.density.toggleLabel")}
              title={`${t("logs.density.toggleLabel")} · ${t(`logs.density.${density}`)}`}
              data-testid="logs-density-toggle"
            >
              <Rows3 className="size-4" />
              {t(`logs.density.${density}`)}
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
          className="mt-4 grid gap-2 lg:grid-cols-[minmax(15rem,1.4fr)_repeat(3,minmax(8rem,0.9fr))_auto]"
        >
          <div ref={searchContainerRef} className="relative">
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
            <option value="wsl">{t("logs.targets.wsl")}</option>
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
          <Button type="submit" size="sm" disabled={isLoading}>
            {t("common.search")}
          </Button>
        </form>

        <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
          <LogsQuickFilters filter={filter} onApply={updateFilter} />
          <button
            type="button"
            onClick={() => setAdvancedManual((value) => !value)}
            aria-expanded={showAdvanced}
            className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            {showAdvanced ? (
              <ChevronUp className="size-3.5" />
            ) : (
              <ChevronDown className="size-3.5" />
            )}
            {t(showAdvanced ? "logs.advancedFilters.hide" : "logs.advancedFilters.show")}
          </button>
        </div>

        {showAdvanced && (
          <div className="mt-3 grid gap-2 lg:grid-cols-3">
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
          </div>
        )}
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
        <div className="mx-5 mt-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive-text">
          {error}
        </div>
      )}

      <div className="min-h-0 flex-1 space-y-5 overflow-auto p-5">
        <LogsKpiRow kpi={kpi} total={total} loadedCount={entries.length} />

        {entries.length > 0 && (
          <LogsActivityCard
            entries={entries}
            selectedDayKey={null}
            onSelectDay={(_dayKey, date) => {
              const bounds = dayBoundsIso(date);
              updateFilter({
                createdAfter: bounds.start,
                createdBefore: bounds.end,
              });
            }}
          />
        )}

        <div className="overflow-hidden rounded-xl border border-border">
          <div className="grid grid-cols-[9rem_4.5rem_minmax(0,1fr)_minmax(0,1.6fr)_5rem_7rem] gap-3 border-b border-border bg-muted/40 px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            <div>{t("logs.columns.time")}</div>
            <div>{t("logs.columns.level")}</div>
            <div>{t("logs.columns.target")}</div>
            <div>{t("logs.columns.action")}</div>
            <div>{t("logs.columns.duration")}</div>
            <div>{t("logs.columns.result")}</div>
          </div>

          {isLoading && entries.length === 0 ? (
            <LogsListSkeleton rows={6} />
          ) : entries.length === 0 ? (
            <LogsEmptyState
              hasFilters={hasFilters}
              onResetFilters={handleResetFilters}
            />
          ) : (
            <div ref={rowsContainerRef}>
              {groups.map((group) => (
                <div key={group.key}>
                  <div className="border-b border-border/70 bg-muted/30 px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t(`logs.groups.${group.key}`)} · {group.entries.length}
                  </div>
                  {group.entries.map((entry) => (
                    <LogsListRow
                      key={entry.id}
                      entry={entry}
                      density={density}
                      onOpen={handleOpenDetail}
                    />
                  ))}
                </div>
              ))}
              <LogsLoadMore
                loaded={entries.length}
                total={total}
                isLoading={isLoading}
                onLoadMore={() =>
                  loadMore().catch((err) =>
                    toast.error(t("logs.loadError", { error: String(err) })),
                  )
                }
              />
            </div>
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


type LogConsoleMode = "operation" | "runtime";

export function OperationLogsView() {
  const { t } = useTranslation();
  const [mode, setMode] = useState<LogConsoleMode>("operation");

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <div className="shrink-0 border-b border-border bg-gradient-to-r from-background via-muted/20 to-background px-5 py-4">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
          <div>
            <div className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
              {t("logs.console.eyebrow")}
            </div>
            <h1 className="mt-1 text-2xl font-semibold tracking-tight">
              {t("logs.console.title")}
            </h1>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              {t("logs.console.description")}
            </p>
          </div>
          <div
            role="tablist"
            aria-label={t("logs.console.modeLabel")}
            className="inline-flex rounded-xl border border-border bg-card/80 p-1 shadow-sm"
          >
            {(["operation", "runtime"] as const).map((item) => (
              <button
                key={item}
                type="button"
                role="tab"
                aria-selected={mode === item}
                onClick={() => setMode(item)}
                className={`inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                  mode === item
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground"
                }`}
              >
                {item === "operation" ? (
                  <Rows3 className="size-4" />
                ) : (
                  <Terminal className="size-4" />
                )}
                {t(`logs.console.modes.${item}`)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1">
        {mode === "operation" ? <OperationLogsPanel /> : <RuntimeLogsPanel />}
      </div>
    </div>
  );
}
