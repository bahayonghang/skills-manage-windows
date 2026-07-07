import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  Circle,
  Loader2,
  ShieldCheck,
  X as XIcon,
  XCircle,
} from "lucide-react";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import type {
  AiTagProgressItem,
  AiTagRateProfile,
} from "@/lib/centralAiTagDashboard";
import { cn } from "@/lib/utils";
import type { AiTagItemStatus, AiTagJob, SkillAiTagReview } from "@/types";

interface CentralSkillAiTagPanelProps {
  aiTagJob: AiTagJob;
  aiTagProgressItems: AiTagProgressItem[];
  aiTagRateProfile: AiTagRateProfile;
  aiTagReviews: SkillAiTagReview[];
  aiTaggingAvailable: boolean;
  selectedSkillCount: number;
  sortedSkillCount: number;
  t: TFunction;
  onCancelAiTag: () => void;
  onOpenReviewTab: () => void;
}

export function CentralSkillAiTagPanel({
  aiTagJob,
  aiTagProgressItems,
  aiTagRateProfile,
  aiTagReviews,
  aiTaggingAvailable,
  selectedSkillCount,
  sortedSkillCount,
  t,
  onCancelAiTag,
  onOpenReviewTab,
}: CentralSkillAiTagPanelProps) {
  const isRunning = aiTagJob.status === "running";
  const isSettled =
    aiTagJob.status === "completed" ||
    aiTagJob.status === "failed" ||
    aiTagJob.status === "cancelled";

  return (
    <div className="space-y-3">
      <AiTagRateCard profile={aiTagRateProfile} t={t} />
      {!aiTaggingAvailable && (
        <div className="rounded-xl bg-muted/20 p-3 text-xs text-foreground/75 ring-1 ring-border/90">
          {t("central.aiTaggingNeedsConfig")}
        </div>
      )}
      {isRunning ? (
        <RunningAiTagPanel
          job={aiTagJob}
          items={aiTagProgressItems}
          t={t}
          onCancel={onCancelAiTag}
        />
      ) : isSettled ? (
        <SettledAiTagPanel
          job={aiTagJob}
          items={aiTagProgressItems}
          reviewCount={aiTagReviews.length}
          t={t}
          onOpenReviewTab={onOpenReviewTab}
        />
      ) : (
        <IdleAiTagPanel
          selectedSkillCount={selectedSkillCount}
          sortedSkillCount={sortedSkillCount}
          t={t}
        />
      )}
    </div>
  );
}

function IdleAiTagPanel({
  selectedSkillCount,
  sortedSkillCount,
  t,
}: {
  selectedSkillCount: number;
  sortedSkillCount: number;
  t: TFunction;
}) {
  return (
    <>
      <DashboardCard
        icon={<Bot className="size-3.5 text-primary" />}
        title={t("central.aiScopeTitle")}
      >
        <p className="leading-relaxed text-foreground/75">
          {t("central.aiScopeDesc", {
            selected: selectedSkillCount,
            total: sortedSkillCount,
          })}
        </p>
      </DashboardCard>
      <DashboardCard
        icon={<ShieldCheck className="size-3.5 text-success-foreground" />}
        title={t("central.aiTagReadyTitle")}
      >
        <p className="leading-relaxed text-foreground/75">
          {t("central.aiPreview", { count: selectedSkillCount })}
        </p>
      </DashboardCard>
    </>
  );
}

function RunningAiTagPanel({
  job,
  items,
  t,
  onCancel,
}: {
  job: AiTagJob;
  items: AiTagProgressItem[];
  t: TFunction;
  onCancel: () => void;
}) {
  const runningItems = items.filter((item) => item.status === "running");
  const queuedItems = items.filter((item) => item.status === "queued");
  const activeItems =
    runningItems.length > 0
      ? runningItems
      : job.currentSkillName
        ? [
            {
              skillId: "current",
              skillName: job.currentSkillName,
              status: "running" as const,
            },
          ]
        : [];
  const percent = getProgressPercent(job);

  return (
    <DashboardCard
      icon={
        <Loader2 className="size-3.5 animate-spin text-primary motion-reduce:animate-none" />
      }
      title={t("central.aiTagCockpitRunningTitle")}
      testId="ai-tag-running-cockpit"
    >
      <div className="space-y-3">
        <div className="flex items-end justify-between gap-3">
          <div>
            <div className="text-2xl font-semibold tabular-nums text-foreground">
              {percent}%
            </div>
            <div className="text-[11px] text-muted-foreground">
              {t("central.aiTagCompleted", {
                completed: job.completed,
                total: job.total,
              })}
            </div>
          </div>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onCancel}
            data-testid="ai-tag-panel-cancel"
            aria-label={t("central.cancelAiTagJob")}
          >
            <XIcon className="size-3.5" />
            {t("central.cancelAiTagJob")}
          </Button>
        </div>
        <ProgressBar
          percent={percent}
          label={t("central.aiTagProgressAriaLabel", { percent })}
        />
        <AiTagStats job={job} t={t} />
        {activeItems.length > 0 && (
          <ItemCluster
            title={t("central.aiTagCurrentTasksTitle")}
            items={activeItems}
            t={t}
            emphasized
          />
        )}
        {queuedItems.length > 0 && (
          <ItemCluster
            title={t("central.aiTagQueuePreviewTitle", {
              count: queuedItems.length,
            })}
            items={queuedItems.slice(0, 5)}
            t={t}
          />
        )}
      </div>
    </DashboardCard>
  );
}

function SettledAiTagPanel({
  job,
  items,
  reviewCount,
  t,
  onOpenReviewTab,
}: {
  job: AiTagJob;
  items: AiTagProgressItem[];
  reviewCount: number;
  t: TFunction;
  onOpenReviewTab: () => void;
}) {
  const percent = getProgressPercent(job);
  const hasReviewEntry = reviewCount > 0 || job.lowConfidenceCount > 0;
  const message = getSettledMessage(job, t);

  return (
    <DashboardCard
      icon={getSettledIcon(job.status)}
      title={t("central.aiTagResultTitle")}
      testId="ai-tag-result-cockpit"
    >
      <div className="space-y-3">
        <ProgressBar
          percent={percent}
          label={t("central.aiTagProgressAriaLabel", { percent })}
        />
        <AiTagStats job={job} t={t} />
        {message && (
          <p
            className={cn(
              "rounded-lg border p-2.5 text-xs leading-relaxed",
              job.status === "failed"
                ? "border-destructive/30 bg-destructive/10 text-destructive"
                : "border-warning/30 bg-warning/10 text-warning-foreground",
            )}
          >
            {message}
          </p>
        )}
        {items.length > 0 && (
          <ItemCluster
            title={t("central.aiTagItemStatusTitle")}
            items={items.slice(0, 6)}
            t={t}
          />
        )}
        {hasReviewEntry && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onOpenReviewTab}
            data-testid="ai-tag-panel-open-review"
          >
            <AlertTriangle className="size-3.5" />
            {t("central.aiTagOpenReviewTab", { count: reviewCount })}
          </Button>
        )}
      </div>
    </DashboardCard>
  );
}

function AiTagRateCard({
  profile,
  t,
}: {
  profile: AiTagRateProfile;
  t: TFunction;
}) {
  const riskReasons = [
    profile.concurrency > 1
      ? t("central.aiTagRateRiskConcurrency", {
          count: profile.concurrency,
        })
      : null,
    profile.intervalMs < 4000
      ? t("central.aiTagRateRiskInterval", { interval: profile.intervalMs })
      : null,
    !profile.stopOnRateLimit ? t("central.aiTagRateRiskStopOff") : null,
  ].filter((reason): reason is string => Boolean(reason));

  return (
    <DashboardCard
      icon={<ShieldCheck className="size-3.5 text-primary" />}
      title={t("central.aiTagRateProfileTitle")}
    >
      <div className="grid grid-cols-3 gap-2 text-center text-[11px]">
        <RateMetric
          label={t("settings.aiTagConcurrencyLabel")}
          value={profile.concurrency}
        />
        <RateMetric
          label={t("settings.aiTagIntervalLabel")}
          value={`${profile.intervalMs}ms`}
        />
        <RateMetric
          label={t("settings.aiTagStopOn429Label")}
          value={formatToggle(profile.stopOnRateLimit, t)}
        />
      </div>
      {!profile.isSafeProfile && (
        <div
          className="mt-3 rounded-lg border border-warning/30 bg-warning/10 p-2.5 text-xs leading-relaxed text-warning-foreground"
          data-testid="ai-tag-rate-risk"
        >
          <div className="mb-1 flex items-center gap-1.5 font-semibold">
            <AlertTriangle className="size-3.5" />
            {t("central.aiTagRateRiskTitle")}
          </div>
          <ul className="list-inside list-disc space-y-0.5">
            {riskReasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        </div>
      )}
    </DashboardCard>
  );
}

function formatToggle(value: boolean, t: TFunction) {
  return value ? t("common.enabled") : t("common.disabled");
}

function AiTagStats({ job, t }: { job: AiTagJob; t: TFunction }) {
  return (
    <div className="grid grid-cols-4 gap-2 text-center text-[11px]">
      <RateMetric label={t("central.aiTagStatDone")} value={job.completed} />
      <RateMetric
        label={t("central.aiTagStatSucceeded")}
        value={job.succeeded}
      />
      <RateMetric label={t("central.aiTagStatFailed")} value={job.failed} />
      <RateMetric
        label={t("central.aiTagStatReview")}
        value={job.lowConfidenceCount}
      />
    </div>
  );
}

function ItemCluster({
  title,
  items,
  t,
  emphasized = false,
}: {
  title: string;
  items: AiTagProgressItem[];
  t: TFunction;
  emphasized?: boolean;
}) {
  return (
    <div className="space-y-1.5">
      <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
        {title}
      </div>
      <div className="space-y-1.5">
        {items.map((item) => (
          <div
            key={`${item.skillId}:${item.status}`}
            className={cn(
              "flex min-w-0 items-center justify-between gap-2 rounded-lg border px-2.5 py-2 text-xs",
              emphasized
                ? "border-primary/25 bg-primary/10 text-primary"
                : "border-border/80 bg-muted/10 text-foreground/75",
            )}
          >
            <span className="min-w-0 truncate font-medium">
              {item.skillName}
            </span>
            <StatusPill status={item.status} t={t} />
          </div>
        ))}
      </div>
    </div>
  );
}

function StatusPill({ status, t }: { status: AiTagItemStatus; t: TFunction }) {
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-semibold ring-1",
        status === "running" && "bg-primary/10 text-primary ring-primary/20",
        status === "queued" && "bg-muted text-muted-foreground ring-border",
        status === "succeeded" &&
          "bg-success/10 text-success-foreground ring-success/20",
        status === "failed" &&
          "bg-destructive/10 text-destructive ring-destructive/20",
        status === "cancelled" &&
          "bg-warning/10 text-warning-foreground ring-warning/20",
      )}
    >
      {status === "running" ? (
        <Loader2 className="size-2.5 animate-spin motion-reduce:animate-none" />
      ) : status === "succeeded" ? (
        <CheckCircle2 className="size-2.5" />
      ) : status === "failed" ? (
        <XCircle className="size-2.5" />
      ) : (
        <Circle className="size-2.5" />
      )}
      {t(`central.aiTagItemStatus.${status}`)}
    </span>
  );
}

function ProgressBar({ percent, label }: { percent: number; label: string }) {
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      className="h-2 overflow-hidden rounded-full bg-muted shadow-inner"
    >
      <div
        className="h-full rounded-full bg-primary shadow-[0_0_18px_rgba(139,92,246,0.35)] transition-[width] duration-300 motion-reduce:transition-none"
        style={{ width: `${percent}%` }}
      />
    </div>
  );
}

function DashboardCard({
  icon,
  title,
  children,
  testId,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
  testId?: string;
}) {
  return (
    <section
      className="rounded-xl bg-background p-3 text-xs shadow-sm ring-1 ring-border/90"
      data-testid={testId}
    >
      <div className="mb-2 flex items-center gap-1.5 font-medium text-foreground">
        {icon}
        {title}
      </div>
      {children}
    </section>
  );
}

function RateMetric({
  label,
  value,
}: {
  label: string;
  value: number | string;
}) {
  return (
    <div className="rounded-lg border border-border/70 bg-muted/10 px-2 py-1.5">
      <div className="font-semibold tabular-nums text-foreground">{value}</div>
      <div className="mt-0.5 text-muted-foreground">{label}</div>
    </div>
  );
}

function getProgressPercent(job: AiTagJob): number {
  return job.total > 0 ? Math.round((job.completed / job.total) * 100) : 0;
}

function getSettledIcon(status: AiTagJob["status"]) {
  if (status === "failed") {
    return <XCircle className="size-3.5 text-destructive" />;
  }
  if (status === "cancelled") {
    return <AlertTriangle className="size-3.5 text-warning-foreground" />;
  }
  return <CheckCircle2 className="size-3.5 text-success-foreground" />;
}

function getSettledMessage(job: AiTagJob, t: TFunction): string | null {
  if (job.error) {
    const formatted = formatBackendError(job.error, t);
    if (/429|rate limit/i.test(formatted)) {
      return t("central.aiTagRateLimitDetected");
    }
    return formatted;
  }
  if (job.status === "cancelled") {
    return t("central.aiTagCancelledSummary");
  }
  if (job.failed > 0) {
    return t("central.aiTagFailedSummary", { count: job.failed });
  }
  return null;
}
