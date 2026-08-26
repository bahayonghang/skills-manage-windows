import { FormEvent, useEffect, useRef, useState } from "react";
import {
  Bug,
  Copy,
  Download,
  FileText,
  Loader2,
  MonitorSmartphone,
  RefreshCw,
  RotateCcw,
  Search,
  Server,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import {
  formatLogUiError,
  safeCorrelationId,
} from "@/components/logs/logDiagnostics";
import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Input } from "@/components/ui/input";
import { useRuntimeLogStore } from "@/stores/runtimeLogStore";
import type { RuntimeLogLine } from "@/types/runtimeLogs";

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

function downloadRuntimeLog(payload: string, fileName: string) {
  downloadText(payload, fileName, "text/plain");
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatRuntimeDate(value?: string | null): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function runtimeLevelClass(level?: string | null): string {
  switch (level) {
    case "error":
      return "border-destructive/30 bg-destructive/10 text-destructive-text";
    case "warn":
      return "border-warning/30 bg-warning/10 text-warning-foreground";
    case "debug":
    case "trace":
      return "border-info/30 bg-info/10 text-info-foreground";
    default:
      return "border-success/25 bg-success/10 text-success-foreground";
  }
}

function RuntimeLineRow({
  line,
  selected,
  onSelect,
}: {
  line: RuntimeLogLine;
  selected: boolean;
  onSelect: (line: RuntimeLogLine) => void;
}) {
  const { t } = useTranslation();
  const sourceKind = line.eventSource ?? "legacy";
  const SourceIcon =
    sourceKind === "backend"
      ? Server
      : sourceKind === "frontend"
        ? MonitorSmartphone
        : FileText;
  return (
    <button
      type="button"
      onClick={() => onSelect(line)}
      className={`grid w-full grid-cols-[4.5rem_5rem_minmax(8rem,0.7fr)_minmax(0,1.6fr)] gap-3 border-b border-border/70 px-3 py-2 text-left text-xs transition-colors hover:bg-muted/40 ${
        selected ? "bg-primary/10" : "bg-background"
      }`}
    >
      <div className="font-mono text-muted-foreground">#{line.lineNumber}</div>
      <div>
        <span
          className={`inline-flex rounded-full border px-2 py-0.5 font-mono text-xs uppercase ${runtimeLevelClass(
            line.level,
          )}`}
        >
          {line.level ?? "log"}
        </span>
      </div>
      <div className="min-w-0">
        <span className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/40 px-2 py-0.5 text-ui-meta text-muted-foreground">
          <SourceIcon className="size-3" />
          {t(`logs.runtime.sources.${sourceKind}`)}
        </span>
        <div className="mt-1 truncate font-mono text-muted-foreground">
          {line.source}
        </div>
      </div>
      <div className="truncate font-mono text-foreground">
        {line.message || line.raw}
      </div>
    </button>
  );
}

export function RuntimeLogsPanel({
  onInspectOperation,
}: {
  onInspectOperation?: (operationId: string) => void;
}) {
  const { t } = useTranslation();
  const translationRef = useRef(t);
  translationRef.current = t;
  const files = useRuntimeLogStore((s) => s.files);
  const selectedFileName = useRuntimeLogStore((s) => s.selectedFileName);
  const readResult = useRuntimeLogStore((s) => s.readResult);
  const filter = useRuntimeLogStore((s) => s.filter);
  const isLoadingFiles = useRuntimeLogStore((s) => s.isLoadingFiles);
  const isLoadingLines = useRuntimeLogStore((s) => s.isLoadingLines);
  const isClearing = useRuntimeLogStore((s) => s.isClearing);
  const isExporting = useRuntimeLogStore((s) => s.isExporting);
  const error = useRuntimeLogStore((s) => s.error);
  const loadFiles = useRuntimeLogStore((s) => s.loadFiles);
  const selectFile = useRuntimeLogStore((s) => s.selectFile);
  const readFile = useRuntimeLogStore((s) => s.readFile);
  const setFilter = useRuntimeLogStore((s) => s.setFilter);
  const clearFilters = useRuntimeLogStore((s) => s.clearFilters);
  const clearLogs = useRuntimeLogStore((s) => s.clearLogs);
  const exportLog = useRuntimeLogStore((s) => s.exportLog);
  const [query, setQuery] = useState(filter.query ?? "");
  const [operationIdDraft, setOperationIdDraft] = useState(
    filter.operationId ?? "",
  );
  const [selectedLine, setSelectedLine] = useState<RuntimeLogLine | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadFiles()
      .then((nextFiles) => {
        if (cancelled || nextFiles.length === 0) return;
        return readFile({
          fileName: nextFiles[0].fileName,
          tail: true,
          offset: 0,
        });
      })
      .catch((err) =>
        toast.error(
          translationRef.current("logs.runtime.loadError", {
            error: formatLogUiError(err, translationRef.current),
          }),
        ),
      );
    return () => {
      cancelled = true;
    };
  }, [loadFiles, readFile]);

  useEffect(() => {
    setSelectedLine(null);
  }, [selectedFileName]);

  const selectedFile =
    files.find((file) => file.fileName === selectedFileName) ?? null;
  const selectedCorrelationId = safeCorrelationId(selectedLine?.operationId);
  const lines = readResult?.lines ?? [];
  const hasRuntimeFilters = Boolean(
    filter.query ||
    filter.level ||
    filter.source ||
    filter.operationId ||
    filter.eventSource,
  );

  function updateRuntimeFilter(partial: Partial<typeof filter>) {
    const nextFilter = { ...filter, ...partial };
    setFilter(partial);
    readFile({
      query: nextFilter.query,
      level: nextFilter.level,
      source: nextFilter.source,
      operationId: nextFilter.operationId,
      eventSource: nextFilter.eventSource,
      offset: 0,
      tail: true,
    }).catch((err) =>
      toast.error(
        t("logs.runtime.loadError", { error: formatLogUiError(err, t) }),
      ),
    );
  }

  function handleRuntimeSearch(event: FormEvent) {
    event.preventDefault();
    updateRuntimeFilter({ query, operationId: operationIdDraft });
  }

  async function handleRuntimeExport() {
    if (!selectedFileName) return;
    try {
      const payload = await exportLog(selectedFileName);
      downloadRuntimeLog(payload, selectedFileName);
      toast.success(t("logs.runtime.exported"));
    } catch (err) {
      toast.error(
        t("logs.runtime.exportError", {
          error: formatLogUiError(err, t),
        }),
      );
    }
  }

  async function handleRuntimeClear() {
    if (!selectedFileName) return;
    try {
      const deleted = await clearLogs({ fileName: selectedFileName });
      toast.success(t("logs.runtime.cleared", { count: deleted }));
    } catch (err) {
      toast.error(
        t("logs.runtime.clearError", { error: formatLogUiError(err, t) }),
      );
    }
  }

  function handleRuntimeResetFilters() {
    clearFilters();
    setQuery("");
    setOperationIdDraft("");
    readFile({
      query: undefined,
      level: undefined,
      source: undefined,
      operationId: undefined,
      eventSource: undefined,
      offset: 0,
      tail: true,
    }).catch((err) =>
      toast.error(
        t("logs.runtime.loadError", { error: formatLogUiError(err, t) }),
      ),
    );
  }

  async function handleCopy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t("logs.copy.success"));
    } catch {
      toast.error(t("logs.copy.failure"));
    }
  }

  function handleCopyRuntimeLine() {
    if (selectedLine) void handleCopy(selectedLine.raw);
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b border-border px-5 py-4">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <h2 className="text-xl font-semibold tracking-tight">
              {t("logs.runtime.title")}
            </h2>
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
              {t("logs.runtime.description")}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                loadFiles()
                  .then((nextFiles) => {
                    const nextFileName =
                      selectedFileName ?? nextFiles[0]?.fileName;
                    if (nextFileName) {
                      return readFile({
                        fileName: nextFileName,
                        tail: true,
                        offset: 0,
                      });
                    }
                    return undefined;
                  })
                  .catch((err) =>
                    toast.error(
                      t("logs.runtime.loadError", {
                        error: formatLogUiError(err, t),
                      }),
                    ),
                  );
              }}
              disabled={isLoadingFiles || isLoadingLines}
            >
              {isLoadingFiles || isLoadingLines ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <RefreshCw className="size-4" />
              )}
              {t("common.refresh")}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleRuntimeExport}
              disabled={!selectedFileName || isExporting || isLoadingLines}
            >
              {isExporting ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Download className="size-4" />
              )}
              {t("common.export")}
            </Button>
            <InlineConfirmAction
              idleAriaLabel={t("logs.runtime.clearSelected")}
              idleTitle={t("logs.runtime.clearSelected")}
              confirmLabel={t("logs.runtime.confirmClearSelected")}
              onConfirm={handleRuntimeClear}
              icon={<Trash2 className="size-4" />}
              disabled={!selectedFileName || isClearing || isLoadingLines}
              isLoading={isClearing}
            />
          </div>
        </div>

        <form
          onSubmit={handleRuntimeSearch}
          className="mt-4 grid gap-2 sm:grid-cols-2 2xl:grid-cols-[minmax(13rem,1.1fr)_8rem_minmax(9rem,0.7fr)_minmax(13rem,1fr)_9rem_auto]"
        >
          <div className="relative sm:col-span-2 2xl:col-span-1">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("logs.runtime.searchPlaceholder")}
              className="pl-8"
            />
          </div>
          <select
            value={filter.level ?? ""}
            onChange={(event) =>
              updateRuntimeFilter({ level: event.target.value as never })
            }
            aria-label={t("logs.filters.level")}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            <option value="">{t("logs.filters.allLevels")}</option>
            <option value="error">error</option>
            <option value="warn">warn</option>
            <option value="info">info</option>
            <option value="debug">debug</option>
            <option value="trace">trace</option>
          </select>
          <Input
            value={filter.source ?? ""}
            onChange={(event) =>
              updateRuntimeFilter({ source: event.target.value })
            }
            placeholder={t("logs.runtime.sourcePlaceholder")}
            aria-label={t("logs.runtime.sourceLabel")}
          />
          <Input
            value={operationIdDraft}
            onChange={(event) => setOperationIdDraft(event.target.value)}
            placeholder={t("logs.runtime.operationIdPlaceholder")}
            aria-label={t("logs.runtime.operationIdLabel")}
            className="font-mono"
          />
          <select
            value={filter.eventSource ?? ""}
            onChange={(event) =>
              updateRuntimeFilter({
                eventSource: event.target.value as "backend" | "frontend" | "",
              })
            }
            aria-label={t("logs.runtime.eventSourceLabel")}
            className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
          >
            <option value="">{t("logs.runtime.allEventSources")}</option>
            <option value="backend">{t("logs.runtime.sources.backend")}</option>
            <option value="frontend">
              {t("logs.runtime.sources.frontend")}
            </option>
          </select>
          <Button
            type="submit"
            size="sm"
            disabled={isLoadingLines || !selectedFileName}
          >
            {t("common.search")}
          </Button>
        </form>

        <div className="mt-2 flex flex-wrap items-center justify-between gap-3 text-xs text-muted-foreground">
          <span>
            {selectedFile
              ? t("logs.runtime.fileMeta", {
                  date: selectedFile.date,
                  size: formatBytes(selectedFile.sizeBytes),
                })
              : t("logs.runtime.noFileSelected")}
          </span>
          <button
            type="button"
            onClick={handleRuntimeResetFilters}
            disabled={!selectedFileName}
            className="inline-flex items-center gap-1 rounded px-2 py-1 hover:bg-muted disabled:opacity-50"
          >
            <RotateCcw className="size-3" />
            {t("logs.resetFilters")}
          </button>
        </div>
      </div>

      {error && (
        <div className="mx-5 mt-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive-text">
          {t("logs.runtime.loadError", {
            error: formatLogUiError(error, t),
          })}
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden p-5 xl:grid-cols-[16rem_minmax(0,1fr)_minmax(18rem,0.65fr)]">
        <aside className="min-h-0 overflow-hidden rounded-xl border border-border bg-card/80">
          <div className="flex items-center justify-between border-b border-border px-3 py-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {t("logs.runtime.files")}
            </div>
            <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
              {files.length}
            </span>
          </div>
          <div className="max-h-full overflow-auto p-2">
            {isLoadingFiles && files.length === 0 ? (
              <div className="flex items-center gap-2 rounded-lg px-2 py-3 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("logs.runtime.loadingFiles")}
              </div>
            ) : files.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border p-4 text-sm text-muted-foreground">
                {t("logs.runtime.emptyFiles")}
              </div>
            ) : (
              <div className="space-y-1">
                {files.map((file) => (
                  <button
                    key={file.fileName}
                    type="button"
                    onClick={() =>
                      selectFile(file.fileName).catch((err) =>
                        toast.error(
                          t("logs.runtime.loadError", {
                            error: formatLogUiError(err, t),
                          }),
                        ),
                      )
                    }
                    className={`w-full rounded-lg border px-3 py-2 text-left transition-colors ${
                      file.fileName === selectedFileName
                        ? "border-primary/40 bg-primary/10 text-foreground"
                        : "border-transparent hover:border-border hover:bg-muted/50"
                    }`}
                  >
                    <div className="font-mono text-sm font-semibold">
                      {file.date}
                    </div>
                    <div className="mt-1 flex items-center justify-between gap-2 text-xs text-muted-foreground">
                      <span>{formatBytes(file.sizeBytes)}</span>
                      <span>{formatRuntimeDate(file.modifiedAt)}</span>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </aside>

        <section className="min-h-0 overflow-hidden rounded-xl border border-border bg-card/80">
          <div className="grid grid-cols-[4.5rem_5rem_minmax(8rem,0.7fr)_minmax(0,1.6fr)] gap-3 border-b border-border bg-muted/40 px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            <div>{t("logs.runtime.columns.line")}</div>
            <div>{t("logs.columns.level")}</div>
            <div>{t("logs.runtime.columns.source")}</div>
            <div>{t("logs.runtime.columns.message")}</div>
          </div>
          <div className="min-h-0 max-h-full overflow-auto">
            {isLoadingLines && lines.length === 0 ? (
              <div className="flex items-center gap-2 p-4 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("logs.runtime.loadingLines")}
              </div>
            ) : lines.length === 0 ? (
              <div className="flex min-h-48 flex-col items-center justify-center gap-3 p-6 text-center text-sm text-muted-foreground">
                <Bug className="size-8" />
                <div>
                  {hasRuntimeFilters
                    ? t("logs.runtime.emptyFiltered")
                    : t("logs.runtime.emptyLines")}
                </div>
              </div>
            ) : (
              lines.map((line) => (
                <RuntimeLineRow
                  key={`${readResult?.fileName}-${line.lineNumber}`}
                  line={line}
                  selected={selectedLine?.lineNumber === line.lineNumber}
                  onSelect={setSelectedLine}
                />
              ))
            )}
          </div>
          {readResult && readResult.total > lines.length && (
            <div className="border-t border-border bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
              {t("logs.runtime.loadedWindow", {
                loaded: lines.length,
                total: readResult.total,
                offset: readResult.offset,
              })}
            </div>
          )}
        </section>

        <aside className="min-h-0 overflow-hidden rounded-xl border border-border bg-card/80">
          <div className="flex items-center justify-between border-b border-border px-3 py-2">
            <div className="inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              <FileText className="size-3.5" />
              {t("logs.runtime.lineDetail")}
            </div>
            <Button
              variant="ghost"
              size="xs"
              onClick={handleCopyRuntimeLine}
              disabled={!selectedLine}
              data-testid="runtime-log-copy-line"
            >
              <Copy className="size-3.5" />
              {t("logs.runtime.copyRaw")}
            </Button>
          </div>
          {selectedLine ? (
            <div className="flex h-full min-h-0 flex-col p-3">
              <dl className="grid gap-3 text-sm">
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("logs.runtime.fields.timestamp")}
                  </dt>
                  <dd className="mt-1 font-mono">
                    {selectedLine.timestamp ?? "—"}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("logs.runtime.fields.source")}
                  </dt>
                  <dd className="mt-1 break-all font-mono">
                    {selectedLine.source}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("logs.runtime.fields.eventSource")}
                  </dt>
                  <dd className="mt-1">
                    {t(
                      `logs.runtime.sources.${selectedLine.eventSource ?? "legacy"}`,
                    )}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("logs.runtime.fields.operationId")}
                  </dt>
                  <dd className="mt-1 flex min-w-0 items-center gap-1 font-mono text-xs">
                    <span className="min-w-0 break-all">
                      {selectedCorrelationId ?? t("logs.diagnostics.legacyId")}
                    </span>
                    {selectedCorrelationId ? (
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        className="shrink-0"
                        aria-label={t("logs.copy.id")}
                        title={t("logs.copy.id")}
                        onClick={() => void handleCopy(selectedCorrelationId)}
                        data-testid="runtime-log-copy-operation-id"
                      >
                        <Copy />
                      </Button>
                    ) : null}
                  </dd>
                </div>
              </dl>
              {selectedCorrelationId && onInspectOperation ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="mt-3"
                  onClick={() => onInspectOperation(selectedCorrelationId)}
                  data-testid="runtime-log-view-operation"
                >
                  {t("logs.runtime.viewOperation")}
                </Button>
              ) : null}
              <pre className="mt-4 min-h-0 flex-1 overflow-auto rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap">
                {selectedLine.raw}
              </pre>
            </div>
          ) : (
            <div className="flex h-full min-h-48 flex-col items-center justify-center gap-2 p-6 text-center text-sm text-muted-foreground">
              <ShieldCheck className="size-8" />
              {t("logs.runtime.noLineSelected")}
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
