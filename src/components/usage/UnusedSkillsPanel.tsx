import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { UsageMatchStatus } from "@/components/usage/SkillUsageTable";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import { statusFillClass } from "@/lib/statusTone";
import { cn } from "@/lib/utils";
import type {
  UnusedSkillEntry,
  UnusedSkillsReport,
  UnusedSkillStatus,
} from "@/types/usage";

type StatusFilter = "all" | UnusedSkillStatus;
type ThresholdDays = 30 | 60 | 90;
type SortMode = "idle" | "size" | "name";

const DAY_MS = 86_400_000;
const THRESHOLD_OPTIONS: ThresholdDays[] = [30, 60, 90];

interface UnusedSkillsPanelProps {
  report: UnusedSkillsReport | null;
  loading?: boolean;
  error?: string | null;
  selectedSkill?: string | null;
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  className?: string;
}

interface VisibleEntry {
  entry: UnusedSkillEntry;
  /** 按当前阈值芯片在视图本地重分类后的状态 */
  status: UnusedSkillStatus;
}

/**
 * 未使用技能面板。store 只按最严阈值（30 天）取一次报告；这里的 30/60/90 切换
 * 是视图本地重分类（callCount === 0 → never_used，否则比较 Date.now() - lastUsedMs
 * 与选中阈值），排序与状态过滤同样不进 store、不改后端请求。
 */
export function UnusedSkillsPanel({
  report,
  loading = false,
  error = null,
  selectedSkill,
  onSelect,
  className,
}: UnusedSkillsPanelProps) {
  const { t, i18n } = useTranslation();
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [thresholdDays, setThresholdDays] = useState<ThresholdDays>(90);
  const [sortMode, setSortMode] = useState<SortMode>("idle");

  const { central, platformByAgent, thresholdTotal } = useMemo(() => {
    const nowMs = Date.now();
    const classify = (entries: UnusedSkillEntry[]): VisibleEntry[] =>
      entries.flatMap((entry) => {
        const status = classifyAtThreshold(entry, thresholdDays, nowMs);
        return status === null ? [] : [{ entry, status }];
      });
    const centralAll = classify(report?.central ?? []);
    const platformAll = classify(report?.platforms ?? []);
    const byAgent = new Map<string, VisibleEntry[]>();
    for (const item of platformAll) {
      for (const agent of item.entry.agents) {
        const list = byAgent.get(agent);
        if (list) list.push(item);
        else byAgent.set(agent, [item]);
      }
    }
    const matchesFilter = (item: VisibleEntry) =>
      statusFilter === "all" || item.status === statusFilter;
    const sortItems = (items: VisibleEntry[]) =>
      sortUnusedEntries(items, sortMode, nowMs);
    return {
      central: sortItems(centralAll.filter(matchesFilter)),
      platformByAgent: new Map(
        [...byAgent.entries()]
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([agent, items]) => [
            agent,
            sortItems(items.filter(matchesFilter)),
          ]),
      ),
      thresholdTotal: centralAll.length + platformAll.length,
    };
  }, [report, thresholdDays, statusFilter, sortMode]);

  // 同一技能装在多个平台时会在每个 agent 小节各出现一次，汇总计数按条目去重
  const visibleUnique = countUniqueVisible(central, platformByAgent);

  if (report === null && loading) {
    return (
      <div className={cn("px-3 py-3", className)}>
        <div className="animate-pulse space-y-2 motion-reduce:animate-none">
          {[0, 1, 2].map((index) => (
            <div key={index} className="h-9 rounded bg-muted/30" />
          ))}
        </div>
        <p className="mt-3 text-center text-xs text-muted-foreground">
          {t("skillUsage.unused.loading")}
        </p>
      </div>
    );
  }

  if (report === null) {
    return (
      <div className={cn("px-4 py-12 text-center text-sm", className)}>
        <p className="text-foreground">
          {error ?? t("skillUsage.unused.loading")}
        </p>
      </div>
    );
  }

  return (
    <div className={cn("flex min-h-0 flex-col", className)}>
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2">
        <span className="text-xs text-muted-foreground">
          {statusFilter === "all"
            ? t("skillUsage.unused.summary", { count: visibleUnique })
            : t("skillUsage.unused.summaryFiltered", {
                count: visibleUnique,
                total: thresholdTotal,
              })}
        </span>
        <div className="flex flex-wrap items-center gap-2">
          <ChipGroup label={t("skillUsage.unused.statusFilter.label")}>
            {(["all", "never_used", "stale"] as const).map((filter) => (
              <Chip
                key={filter}
                pressed={statusFilter === filter}
                onClick={() => setStatusFilter(filter)}
              >
                {t(`skillUsage.unused.statusFilter.${filter}`)}
              </Chip>
            ))}
          </ChipGroup>
          <ChipGroup label={t("skillUsage.unused.threshold.label")}>
            {THRESHOLD_OPTIONS.map((days) => (
              <Chip
                key={days}
                pressed={thresholdDays === days}
                onClick={() => setThresholdDays(days)}
              >
                {t("skillUsage.unused.threshold.days", { count: days })}
              </Chip>
            ))}
          </ChipGroup>
          <ChipGroup label={t("skillUsage.unused.sort.label")}>
            {(["idle", "size", "name"] as const).map((mode) => (
              <Chip
                key={mode}
                pressed={sortMode === mode}
                onClick={() => setSortMode(mode)}
              >
                {t(`skillUsage.unused.sort.${mode}`)}
              </Chip>
            ))}
          </ChipGroup>
        </div>
      </div>

      {visibleUnique === 0 ? (
        <div className="flex-1 px-4 py-12 text-center text-sm">
          <p className="text-foreground">{t("skillUsage.unused.empty")}</p>
        </div>
      ) : (
        <div className="scrollbar-subtle max-h-[26rem] overflow-y-auto">
          <UnusedSection
            title={t("skillUsage.unused.groups.central")}
            items={central}
            selectedSkill={selectedSkill}
            onSelect={onSelect}
            locale={i18n.language}
          />
          {[...platformByAgent.entries()].map(([agent, items]) =>
            items.length === 0 ? null : (
              <UnusedSection
                key={agent}
                title={t("skillUsage.unused.groups.agentSection", { agent })}
                items={items}
                selectedSkill={selectedSkill}
                onSelect={onSelect}
                locale={i18n.language}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function UnusedSection({
  title,
  items,
  selectedSkill,
  onSelect,
  locale,
}: {
  title: string;
  items: VisibleEntry[];
  selectedSkill?: string | null;
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  locale: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  if (items.length === 0) return null;
  return (
    <section>
      <h3 className="border-b border-border bg-muted/20 px-3 py-1.5 text-xs font-medium text-muted-foreground">
        {title} · {items.length}
      </h3>
      <div className="grid grid-cols-[minmax(8rem,1fr)_3.5rem_4.5rem_2rem] gap-2 border-b border-border px-3 py-2 text-ui-meta text-muted-foreground md:grid-cols-[minmax(9rem,1fr)_5.5rem_4rem_6rem_5rem_5rem_2rem]">
        <span>{t("skillUsage.unused.columns.skill")}</span>
        <span className="hidden md:block">
          {t("skillUsage.unused.columns.match")}
        </span>
        <span className="text-right">
          {t("skillUsage.unused.columns.calls")}
        </span>
        <span className="hidden text-right md:block">
          {t("skillUsage.unused.columns.estimate")}
        </span>
        <span className="text-right">
          {t("skillUsage.unused.columns.lastUsed")}
        </span>
        <span className="hidden md:block">
          {t("skillUsage.unused.columns.status")}
        </span>
        <span className="sr-only">{t("skillUsage.openSkill")}</span>
      </div>
      <ul className="divide-y divide-border/70">
        {items.map(({ entry, status }) => {
          const selected = entry.name === selectedSkill;
          return (
            <li
              key={`${entry.origin}-${entry.name}`}
              data-testid={`unused-row-${entry.origin}-${entry.name}`}
              className={cn(
                "grid grid-cols-[minmax(8rem,1fr)_3.5rem_4.5rem_2rem] items-stretch gap-2 px-3 md:grid-cols-[minmax(9rem,1fr)_5.5rem_4rem_6rem_5rem_5rem_2rem]",
                selected && "bg-primary/7",
              )}
            >
              <button
                type="button"
                aria-pressed={selected}
                onClick={(event) => onSelect(entry.name, event.currentTarget)}
                className="col-span-3 grid min-w-0 grid-cols-subgrid items-center py-2.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring md:col-span-6"
              >
                <span className="min-w-0 pr-2">
                  <span className="block truncate text-sm font-medium text-foreground">
                    {entry.name}
                  </span>
                  <span className="mt-0.5 block truncate text-ui-meta text-muted-foreground">
                    {entry.agents.length > 0
                      ? entry.agents.join(" · ")
                      : t("skillUsage.unused.noAgents")}
                  </span>
                </span>
                <UsageMatchStatus
                  status={entry.matchStatus}
                  className="hidden md:flex"
                />
                <span className="text-right text-sm font-medium tabular-nums">
                  {entry.callCount.toLocaleString()}
                </span>
                <span
                  className="hidden text-right text-xs tabular-nums text-muted-foreground md:block"
                  title={
                    entry.staticTokenEstimate === null
                      ? t("skillUsage.staticEstimateUnavailable")
                      : t("skillUsage.staticEstimateTooltip", {
                          bytes:
                            entry.staticByteCount?.toLocaleString() ?? "—",
                        })
                  }
                >
                  {entry.staticTokenEstimate === null
                    ? "—"
                    : `~${entry.staticTokenEstimate.toLocaleString()}`}
                </span>
                <span className="text-right text-xs tabular-nums text-muted-foreground">
                  {entry.lastUsedMs === null
                    ? t("skillUsage.unused.never")
                    : formatUsageRelativeTime(entry.lastUsedMs, locale)}
                </span>
                <UnusedStatusBadge status={status} className="hidden md:flex" />
              </button>
              <div className="flex items-center justify-end">
                {entry.skillId && (
                  <Button
                    type="button"
                    size="icon-sm"
                    variant="ghost"
                    title={t("skillUsage.openSkill")}
                    aria-label={t("skillUsage.openSkillNamed", {
                      skill: entry.name,
                    })}
                    onClick={() =>
                      navigate(`/skill/${encodeURIComponent(entry.skillId!)}`)
                    }
                  >
                    <ArrowUpRight className="size-3.5" />
                  </Button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** 未使用状态文本 + 语义色状态点：无色彩场景下凭文字可读。 */
function UnusedStatusBadge({
  status,
  className,
}: {
  status: UnusedSkillStatus;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <span
      className={cn(
        "flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground",
        className,
      )}
    >
      <span
        aria-hidden
        className={cn(
          "size-1.5 shrink-0 rounded-full",
          status === "never_used"
            ? statusFillClass.warning
            : statusFillClass.info,
        )}
      />
      <span className="truncate">{t(`skillUsage.unused.status.${status}`)}</span>
    </span>
  );
}

function ChipGroup({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className="inline-flex rounded-md border border-border p-0.5"
    >
      {children}
    </div>
  );
}

function Chip({
  pressed,
  onClick,
  children,
}: {
  pressed: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={cn(
        "min-h-7 rounded px-2.5 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        pressed
          ? "bg-foreground text-background"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}

/**
 * 视图本地重分类：callCount === 0 → never_used；否则 lastUsedMs 距今达到选中
 * 阈值 → stale；达不到阈值返回 null（该阈值下不算未使用，从列表排除）。
 */
function classifyAtThreshold(
  entry: UnusedSkillEntry,
  thresholdDays: ThresholdDays,
  nowMs: number,
): UnusedSkillStatus | null {
  if (entry.callCount === 0) return "never_used";
  if (entry.lastUsedMs === null) return entry.status;
  return nowMs - entry.lastUsedMs >= thresholdDays * DAY_MS ? "stale" : null;
}

function sortUnusedEntries(
  items: VisibleEntry[],
  mode: SortMode,
  nowMs: number,
): VisibleEntry[] {
  return [...items].sort((a, b) => {
    if (mode === "name") {
      return a.entry.name.localeCompare(b.entry.name);
    }
    if (mode === "size") {
      // 缺失的体积估算按 -1 处理排到末尾，绝不当作 0
      const sizeDelta =
        (b.entry.staticByteCount ?? -1) - (a.entry.staticByteCount ?? -1);
      return sizeDelta || a.entry.name.localeCompare(b.entry.name);
    }
    const idleDelta = idleMs(b.entry, nowMs) - idleMs(a.entry, nowMs);
    return idleDelta || a.entry.name.localeCompare(b.entry.name);
  });
}

/** never_used 视为无限久未使用，排在最前。 */
function idleMs(entry: UnusedSkillEntry, nowMs: number): number {
  if (entry.callCount === 0 || entry.lastUsedMs === null) {
    return Number.POSITIVE_INFINITY;
  }
  return nowMs - entry.lastUsedMs;
}

function countUniqueVisible(
  central: VisibleEntry[],
  platformByAgent: Map<string, VisibleEntry[]>,
): number {
  const seen = new Set<UnusedSkillEntry>(central.map((item) => item.entry));
  for (const items of platformByAgent.values()) {
    for (const item of items) seen.add(item.entry);
  }
  return seen.size;
}
