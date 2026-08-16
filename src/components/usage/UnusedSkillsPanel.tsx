import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowUpRight, Unlink } from "lucide-react";

import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { UsageMatchStatus } from "@/components/usage/SkillUsageTable";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import { statusChipClass, statusFillClass } from "@/lib/statusTone";
import type { StatusTone } from "@/lib/statusTone";
import { cn } from "@/lib/utils";
import { unlinkActionKey } from "@/stores/usageStore";
import type {
  UnusedAgentInstall,
  UnusedPlatformInstall,
  UnusedSkillEntry,
  UnusedSkillsReport,
  UnusedSkillStatus,
} from "@/types/usage";

type StatusFilter = "all" | UnusedSkillStatus;
type ThresholdDays = 30 | 60 | 90;
type SortMode = "idle" | "size" | "name";

const DAY_MS = 86_400_000;
const THRESHOLD_OPTIONS: ThresholdDays[] = [30, 60, 90];

/**
 * <40px 图标按钮的 40px 热区扩展（icon-control-hit-area.md 基准模式）；
 * 行操作区按钮相邻排列时配合 gap-2 保证中心距 ≥40px。
 */
const HIT_AREA_CLASS =
  "relative after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

interface UnusedSkillsPanelProps {
  report: UnusedSkillsReport | null;
  loading?: boolean;
  error?: string | null;
  selectedSkill?: string | null;
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  /** store 的 unlinkUnusedSkill（组件不直接 invoke） */
  onUnlink: (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => Promise<void>;
  /** 进行中的 unlink 动作 key 集合，驱动按钮 spinner */
  pendingUnlinkKeys?: Record<string, boolean>;
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
  onUnlink,
  pendingUnlinkKeys = {},
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
      for (const agentId of new Set(
        item.entry.installs.map((install) => install.agentId),
      )) {
        const list = byAgent.get(agentId);
        if (list) list.push(item);
        else byAgent.set(agentId, [item]);
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
            onUnlink={onUnlink}
            pendingUnlinkKeys={pendingUnlinkKeys}
            locale={i18n.language}
          />
          {[...platformByAgent.entries()].map(([agent, items]) =>
            items.length === 0 ? null : (
              <UnusedSection
                key={agent}
                title={t("skillUsage.unused.groups.agentSection", { agent })}
                sectionAgent={agent}
                items={items}
                selectedSkill={selectedSkill}
                onSelect={onSelect}
                onUnlink={onUnlink}
                pendingUnlinkKeys={pendingUnlinkKeys}
                locale={i18n.language}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

// 行/表头共用网格：State 列加宽到 6.5rem 保证徽章全文可读，操作列 6.5rem
// 稳定容纳 32px 打开按钮、gap-2 与展开后的确认按钮。
const ROW_GRID =
  "grid grid-cols-[minmax(8rem,1fr)_3.5rem_4.5rem_6.5rem] gap-2 md:grid-cols-[minmax(9rem,1fr)_5.5rem_4rem_6rem_5rem_6.5rem_6.5rem]";

function UnusedSection({
  title,
  items,
  sectionAgent,
  selectedSkill,
  onSelect,
  onUnlink,
  pendingUnlinkKeys,
  locale,
}: {
  title: string;
  items: VisibleEntry[];
  /** 平台小节的 agent id；Central 小节为 undefined */
  sectionAgent?: string;
  selectedSkill?: string | null;
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  onUnlink: (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => Promise<void>;
  pendingUnlinkKeys: Record<string, boolean>;
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
      <div
        className={cn(
          ROW_GRID,
          "border-b border-border px-3 py-2 text-ui-meta text-muted-foreground",
        )}
      >
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
        <span className="sr-only">{t("skillUsage.unused.columns.actions")}</span>
      </div>
      <ul className="divide-y divide-border/70">
        {items.map(({ entry, status }) => {
          const selected = entry.name === selectedSkill;
          const sectionInstall = sectionAgent
            ? preferredPlatformInstall(entry.installs, sectionAgent)
            : undefined;
          return (
            <li
              key={`${entry.origin}-${entry.name}`}
              data-testid={`unused-row-${entry.origin}-${entry.name}`}
              className={cn("group/row", selected && "bg-primary/7")}
            >
              <div className={cn(ROW_GRID, "items-stretch px-3")}>
                <button
                  type="button"
                  aria-pressed={selected}
                  onClick={(event) =>
                    onSelect(entry.name, event.currentTarget)
                  }
                  className="col-span-3 grid min-w-0 grid-cols-subgrid items-center py-2.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring md:col-span-6"
                >
                  <span className="min-w-0 pr-2">
                    <span className="block truncate text-sm font-medium text-foreground">
                      {entry.name}
                    </span>
                    {entry.origin === "platform" && (
                      <span className="mt-0.5 block truncate text-ui-meta text-muted-foreground">
                        {entryAgentIds(entry).length > 0
                          ? entryAgentIds(entry).join(" · ")
                          : t("skillUsage.unused.noAgents")}
                      </span>
                    )}
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
                  <UnusedStatusBadge
                    status={status}
                    className="hidden md:flex"
                  />
                </button>
                {/* 操作区：默认低透明度，行 hover / 内部聚焦时全显（focus-within
                    是 focus-visible 的等价键盘路径，容器级避免子控件不可见聚焦） */}
                <div className="flex items-center justify-end gap-2 opacity-50 transition-opacity group-hover/row:opacity-100 focus-within:opacity-100">
                  {entry.skillId && (
                    <Button
                      type="button"
                      size="icon-sm"
                      variant="ghost"
                      title={t("skillUsage.openSkill")}
                      aria-label={t("skillUsage.openSkillNamed", {
                        skill: entry.name,
                      })}
                      className={HIT_AREA_CLASS}
                      onClick={() =>
                        navigate(
                          `/skill/${encodeURIComponent(entry.skillId!)}`,
                        )
                      }
                    >
                      <ArrowUpRight className="size-3.5" />
                    </Button>
                  )}
                  {sectionInstall && (
                    <PlatformUnlinkAction
                      install={sectionInstall}
                      pending={
                        pendingUnlinkKeys[
                          unlinkActionKey(
                            sectionInstall.agentId,
                            sectionInstall.skillId,
                            sectionInstall.rowId,
                          )
                        ] ?? false
                      }
                      onUnlink={onUnlink}
                    />
                  )}
                </div>
              </div>
              {entry.origin === "central" && entry.agents.length > 0 && (
                <div className="flex flex-wrap items-center gap-2 px-3 pb-2.5">
                  {entry.agents.map((install) => (
                    <CentralAgentChip
                      key={install.agentId}
                      entry={entry}
                      install={install}
                      pending={
                        pendingUnlinkKeys[
                          unlinkActionKey(install.agentId, entry.skillId ?? "")
                        ] ?? false
                      }
                      onUnlink={onUnlink}
                    />
                  ))}
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/**
 * Central 条目的 per-agent 安装 chip：可 unlink 的附带两段式确认按钮；
 * shared-root（后端记为 linkType "native"）或 central 自身的安装禁用并给原因。
 */
function CentralAgentChip({
  entry,
  install,
  pending,
  onUnlink,
}: {
  entry: UnusedSkillEntry;
  install: UnusedAgentInstall;
  pending: boolean;
  onUnlink: (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => Promise<void>;
}) {
  const { t } = useTranslation();
  const disabledReason = install.hasPendingRecovery
    ? "disabledPendingRecovery"
    : install.agentId === "central" || install.linkType === "native"
      ? "disabledSharedRoot"
      : null;
  if (disabledReason !== null) {
    return (
      <span
        data-testid={`unlink-chip-disabled-${install.agentId}`}
        title={t(`skillUsage.unused.unlink.${disabledReason}`)}
        className="inline-flex h-8 items-center whitespace-nowrap rounded-md border border-border bg-muted/30 px-2.5 text-xs text-muted-foreground"
      >
        {install.agentId}
      </span>
    );
  }
  return (
    <span
      data-testid={`unlink-chip-${install.agentId}`}
      className="inline-flex h-8 items-center gap-1 whitespace-nowrap rounded-md border border-border bg-muted/30 pl-2.5 pr-0.5 text-xs text-muted-foreground"
    >
      {install.agentId}
      <InlineConfirmAction
        idleAriaLabel={t("skillUsage.unused.unlink.actionLabel", {
          agent: install.agentId,
        })}
        idleTitle={t("skillUsage.unused.unlink.actionLabel", {
          agent: install.agentId,
        })}
        confirmLabel={t("skillUsage.unused.unlink.confirm")}
        icon={<Unlink className="size-3.5" />}
        isLoading={pending}
        onConfirm={() => onUnlink(entry.skillId ?? "", install.agentId)}
        className={HIT_AREA_CLASS}
      />
    </span>
  );
}

/** 平台散件 install 的行内 unlink：read-only / 非 user 来源 / 缺 rowId 时禁用。 */
function PlatformUnlinkAction({
  install,
  pending,
  onUnlink,
}: {
  install: UnusedPlatformInstall;
  pending: boolean;
  onUnlink: (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => Promise<void>;
}) {
  const { t } = useTranslation();
  const disabledReason = platformUnlinkDisabledReason(install);
  return (
    <span data-testid={`unlink-action-${install.agentId}-${install.skillId}`}>
      <InlineConfirmAction
        idleAriaLabel={t("skillUsage.unused.unlink.actionLabel", {
          agent: install.agentId,
        })}
        idleTitle={
          disabledReason === null
            ? t("skillUsage.unused.unlink.actionLabel", {
                agent: install.agentId,
              })
            : t(`skillUsage.unused.unlink.${disabledReason}`)
        }
        confirmLabel={t("skillUsage.unused.unlink.confirm")}
        icon={<Unlink className="size-4" />}
        disabled={disabledReason !== null}
        isLoading={pending}
        onConfirm={() =>
          onUnlink(install.skillId, install.agentId, install.rowId)
        }
        className={HIT_AREA_CLASS}
      />
    </span>
  );
}

function platformUnlinkDisabledReason(
  install: UnusedPlatformInstall,
):
  | "disabledPendingRecovery"
  | "disabledReadOnly"
  | "disabledSourceKind"
  | "disabledNoRow"
  | null {
  if (install.hasPendingRecovery) return "disabledPendingRecovery";
  if (install.isReadOnly) return "disabledReadOnly";
  if (install.sourceKind !== "user") return "disabledSourceKind";
  if (install.rowId === null) return "disabledNoRow";
  return null;
}

function preferredPlatformInstall(
  installs: UnusedPlatformInstall[],
  agentId: string,
): UnusedPlatformInstall | undefined {
  const agentInstalls = installs.filter(
    (install) => install.agentId === agentId,
  );
  return (
    agentInstalls.find(
      (install) => platformUnlinkDisabledReason(install) === null,
    ) ?? agentInstalls[0]
  );
}

const statusBadgeTone: Record<UnusedSkillStatus, StatusTone> = {
  never_used: "warning",
  stale: "info",
};

/** 未使用状态徽章：紧凑 chip 全文可读（不截断），语义色点 + 文本保持无色可读。 */
function UnusedStatusBadge({
  status,
  className,
}: {
  status: UnusedSkillStatus;
  className?: string;
}) {
  const { t } = useTranslation();
  const tone = statusBadgeTone[status];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border px-2 py-0.5 text-xs",
        statusChipClass[tone],
        className,
      )}
    >
      <span
        aria-hidden
        className={cn("size-1.5 shrink-0 rounded-full", statusFillClass[tone])}
      />
      {t(`skillUsage.unused.status.${status}`)}
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

/** Central 条目取 agents，平台条目取 installs 的去重 agentId 列表（展示用）。 */
function entryAgentIds(entry: UnusedSkillEntry): string[] {
  if (entry.origin === "central") {
    return entry.agents.map((install) => install.agentId);
  }
  return [...new Set(entry.installs.map((install) => install.agentId))];
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
