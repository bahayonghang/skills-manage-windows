import type { ReactNode } from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import {
  AlertTriangle,
  Bot,
  Check,
  Download,
  Eye,
  FileJson,
  X as XIcon,
} from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatBackendError } from "@/lib/backendError";
import { cn } from "@/lib/utils";
import type {
  AiTagJob,
  CentralSkillUpdateJob,
  SkillportStatePortabilityJob,
} from "@/types";

/**
 * TaskCenterDrawer：右侧抽屉，聚合三类异步任务。
 *
 * - AI 标签（aiTagJob）
 * - 中央更新检查 / 应用（updateJob）
 * - portability 导入 / 导出（portabilityJob）
 *
 * 每行：图标 + 标题 + 状态徽章 + 当前项 + 1.5px 进度条 + 统计 + 取消按钮。
 * 完成 / 失败 / 取消的任务也在列表中展示，供回看；点击关闭整体抽屉收纳。
 */
export interface TaskCenterDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  t: TFunction;
  aiTagJob: AiTagJob;
  updateJob: CentralSkillUpdateJob;
  portabilityJob: SkillportStatePortabilityJob;
  onCancelAiTag: () => void;
  onCancelUpdate: () => void;
  onCancelPortability: () => void;
  onViewAiReviews: () => void;
}

export function TaskCenterDrawer({
  open,
  onOpenChange,
  t,
  aiTagJob,
  updateJob,
  portabilityJob,
  onCancelAiTag,
  onCancelUpdate,
  onCancelPortability,
  onViewAiReviews,
}: TaskCenterDrawerProps) {
  const titleId = "central-task-center-drawer-title";

  const hasAi = aiTagJob.status !== "idle";
  const hasUpdate = updateJob.status !== "idle";
  const hasPortability = portabilityJob.status !== "idle";
  const hasAny = hasAi || hasUpdate || hasPortability;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal keepMounted={false}>
        <DialogOverlay className="bg-overlay" />
        <DialogPrimitive.Popup
          data-testid="central-task-center-drawer"
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          className="fixed inset-y-0 right-0 z-50 flex h-full w-screen flex-col bg-background shadow-2xl ring-1 ring-border outline-none sm:w-[min(440px,96vw)]"
        >
          <div className="flex shrink-0 items-start justify-between gap-3 border-b border-border p-4">
            <div className="min-w-0 space-y-1">
              <DialogTitle id={titleId}>
                {t("central.taskCenterTitle")}
              </DialogTitle>
              <p className="text-sm text-muted-foreground">
                {t("central.taskCenterDesc")}
              </p>
            </div>
            <DialogClose
              render={
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("common.close")}
                />
              }
            >
              <XIcon />
            </DialogClose>
          </div>
          <div className="flex-1 space-y-3 overflow-y-auto p-4">
            {!hasAny && (
              <p
                data-testid="central-task-center-empty"
                className="text-sm text-muted-foreground"
              >
                {t("central.taskCenterEmpty")}
              </p>
            )}
            {hasAi && (
              <AiTagTaskRow
                t={t}
                job={aiTagJob}
                onCancel={onCancelAiTag}
                onViewReviews={onViewAiReviews}
              />
            )}
            {hasUpdate && (
              <UpdateTaskRow t={t} job={updateJob} onCancel={onCancelUpdate} />
            )}
            {hasPortability && (
              <PortabilityTaskRow
                t={t}
                job={portabilityJob}
                onCancel={onCancelPortability}
              />
            )}
          </div>
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}

// ─── 通用任务行 ───────────────────────────────────────────────────────

interface TaskRowProps {
  testId: string;
  icon: ReactNode;
  title: string;
  current?: string;
  statusBadge?: ReactNode;
  ratio: number;
  hasFailed: boolean;
  isCompleted: boolean;
  stats: ReactNode;
  error?: string | null;
  actions?: ReactNode;
}

function TaskRow({
  testId,
  icon,
  title,
  current,
  statusBadge,
  ratio,
  hasFailed,
  isCompleted,
  stats,
  error,
  actions,
}: TaskRowProps) {
  return (
    <div
      data-testid={testId}
      className="rounded-lg border border-border bg-card p-3 shadow-sm"
    >
      <div className="flex items-start gap-3">
        <span
          className={cn(
            "grid size-7 shrink-0 place-items-center rounded-full ring-1",
            hasFailed
              ? "bg-destructive/10 text-destructive ring-destructive/20"
              : isCompleted
                ? "bg-success/10 text-success-foreground ring-success/20"
                : "bg-primary/10 text-primary ring-primary/20",
          )}
        >
          {icon}
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm font-medium text-foreground">
            <span className="truncate">{title}</span>
            {statusBadge}
          </div>
          {current && (
            <div className="mt-0.5 truncate text-xs text-muted-foreground">
              {current}
            </div>
          )}
        </div>
      </div>
      <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
        <div
          className={cn(
            "h-full w-full origin-left rounded-full transition-transform duration-300 ease-out",
            hasFailed ? "bg-destructive" : "bg-primary",
          )}
          style={{ transform: `scaleX(${ratio})` }}
        />
      </div>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs">
        <div className="flex flex-wrap items-center gap-2 text-muted-foreground">
          {stats}
        </div>
        {actions && (
          <div className="flex flex-wrap items-center gap-2">{actions}</div>
        )}
      </div>
      {error && (
        <p
          className={cn(
            "mt-2 text-xs",
            hasFailed ? "text-destructive" : "text-warning-foreground",
          )}
        >
          {error}
        </p>
      )}
    </div>
  );
}

// ─── 状态徽章 ────────────────────────────────────────────────────────

function StatusBadge({
  tone,
  label,
}: {
  tone: "running" | "ok" | "warn" | "fail";
  label: string;
}) {
  return (
    <span
      className={cn(
        "rounded-full px-2 py-0.5 text-[10px] font-medium",
        tone === "running" &&
          "bg-primary/10 text-primary ring-1 ring-primary/20",
        tone === "ok" &&
          "bg-success/10 text-success-foreground ring-1 ring-success/20",
        tone === "warn" &&
          "bg-warning/10 text-warning-foreground ring-1 ring-warning/20",
        tone === "fail" &&
          "bg-destructive/10 text-destructive ring-1 ring-destructive/20",
      )}
    >
      {label}
    </span>
  );
}

function renderStatusBadge(
  status: string,
  t: TFunction,
): { node: ReactNode; tone: "running" | "ok" | "warn" | "fail" } | null {
  if (status === "running") {
    return {
      node: (
        <StatusBadge
          tone="running"
          label={t("central.taskCenterStatusRunning")}
        />
      ),
      tone: "running",
    };
  }
  if (status === "cancelling") {
    return {
      node: (
        <StatusBadge
          tone="warn"
          label={t("central.taskCenterStatusCancelling")}
        />
      ),
      tone: "warn",
    };
  }
  if (status === "completed") {
    return {
      node: (
        <StatusBadge tone="ok" label={t("central.taskCenterStatusCompleted")} />
      ),
      tone: "ok",
    };
  }
  if (status === "failed") {
    return {
      node: (
        <StatusBadge tone="fail" label={t("central.taskCenterStatusFailed")} />
      ),
      tone: "fail",
    };
  }
  if (status === "cancelled") {
    return {
      node: (
        <StatusBadge
          tone="warn"
          label={t("central.taskCenterStatusCancelled")}
        />
      ),
      tone: "warn",
    };
  }
  return null;
}

// ─── AI Tag 行 ──────────────────────────────────────────────────────

function AiTagTaskRow({
  t,
  job,
  onCancel,
  onViewReviews,
}: {
  t: TFunction;
  job: AiTagJob;
  onCancel: () => void;
  onViewReviews: () => void;
}) {
  const ratio = job.total > 0 ? Math.min(1, job.completed / job.total) : 0;
  const isRunning = job.status === "running";
  const isCompleted = job.status === "completed";
  const hasFailed = job.status === "failed";
  const statusBadge = renderStatusBadge(job.status, t)?.node;
  const current =
    isRunning && job.currentSkillName
      ? t("central.aiTagCurrent", { name: job.currentSkillName })
      : undefined;

  return (
    <TaskRow
      testId="central-task-row-ai-tag"
      icon={<Bot className="size-3.5" />}
      title={t("central.aiTagProgressTitle")}
      current={current}
      statusBadge={statusBadge}
      ratio={ratio}
      hasFailed={hasFailed}
      isCompleted={isCompleted}
      stats={
        <>
          <span>
            {t("central.aiTagCompleted", {
              completed: job.completed,
              total: job.total,
            })}
          </span>
          <span>{t("central.aiTagSucceeded", { count: job.succeeded })}</span>
          <span>{t("central.aiTagFailed", { count: job.failed })}</span>
          <span>
            {t("central.aiTagLowConfidence", { count: job.lowConfidenceCount })}
          </span>
        </>
      }
      error={job.error ? formatBackendError(job.error, t) : null}
      actions={
        <>
          {isCompleted && (
            <Button
              type="button"
              size="sm"
              variant="ghost"
              onClick={onViewReviews}
              data-testid="task-row-ai-tag-view-reviews"
            >
              <Eye className="size-3.5" />
              {t("central.viewFailuresAndReviews")}
            </Button>
          )}
          {isRunning && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={onCancel}
              data-testid="task-row-ai-tag-cancel"
            >
              <XIcon className="size-3.5" />
              {t("central.cancelAiTagJob")}
            </Button>
          )}
        </>
      }
    />
  );
}

// ─── Update 行 ──────────────────────────────────────────────────────

function UpdateTaskRow({
  t,
  job,
  onCancel,
}: {
  t: TFunction;
  job: CentralSkillUpdateJob;
  onCancel: () => void;
}) {
  const ratio = job.total > 0 ? Math.min(1, job.completed / job.total) : 0;
  const isRunning = job.status === "running";
  const isCancelling = job.status === "cancelling";
  const isCompleted = job.status === "completed";
  const hasFailed = job.status === "failed";
  const statusBadge = renderStatusBadge(job.status, t)?.node;
  const icon = hasFailed ? (
    <AlertTriangle className="size-3.5" />
  ) : isCompleted ? (
    <Check className="size-3.5" />
  ) : (
    <Download className="size-3.5" />
  );
  const current =
    isRunning && job.currentSkillName
      ? t("central.updateCurrent", { name: job.currentSkillName })
      : undefined;

  return (
    <TaskRow
      testId="central-task-row-update"
      icon={icon}
      title={t("central.updateProgressTitle")}
      current={current}
      statusBadge={statusBadge}
      ratio={ratio}
      hasFailed={hasFailed}
      isCompleted={isCompleted}
      stats={
        <>
          <span>
            {t("central.updateCompleted", {
              completed: job.completed,
              total: job.total,
            })}
          </span>
          <span>{t("central.updateSucceeded", { count: job.succeeded })}</span>
          <span>{t("central.updateSkipped", { count: job.skipped })}</span>
          <span
            className={cn(
              job.failed > 0 &&
                "rounded-full bg-destructive/10 px-1.5 text-destructive",
            )}
          >
            {t("central.updateFailed", { count: job.failed })}
          </span>
        </>
      }
      error={job.error ?? null}
      actions={
        (isRunning || isCancelling) && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onCancel}
            disabled={isCancelling}
            data-testid="task-row-update-cancel"
          >
            <XIcon className="size-3.5" />
            {isCancelling
              ? t("central.cancellingUpdateJob")
              : t("central.cancelUpdateJob")}
          </Button>
        )
      }
    />
  );
}

// ─── Portability 行 ─────────────────────────────────────────────────

function PortabilityTaskRow({
  t,
  job,
  onCancel,
}: {
  t: TFunction;
  job: SkillportStatePortabilityJob;
  onCancel: () => void;
}) {
  const ratio = job.total > 0 ? Math.min(1, job.completed / job.total) : 0;
  const isRunning = job.status === "running";
  const isCancelling = job.status === "cancelling";
  const isCompleted = job.status === "completed";
  const hasFailed = job.status === "failed";
  const statusBadge = renderStatusBadge(job.status, t)?.node;
  const phaseLabel = job.phase
    ? t(`central.portabilityPhase.${job.phase}`)
    : "";
  const currentText = [job.message ?? phaseLabel, job.currentItem]
    .filter((part) => Boolean(part))
    .join(" · ");

  return (
    <TaskRow
      testId="central-task-row-portability"
      icon={<FileJson className="size-3.5" />}
      title={t("central.portabilityProgressTitle")}
      current={currentText || undefined}
      statusBadge={statusBadge}
      ratio={ratio}
      hasFailed={hasFailed}
      isCompleted={isCompleted}
      stats={
        <span>
          {t("central.portabilityProgressCompleted", {
            completed: job.completed,
            total: job.total,
          })}
        </span>
      }
      error={job.error ?? null}
      actions={
        (isRunning || isCancelling) && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onCancel}
            disabled={isCancelling}
            data-testid="task-row-portability-cancel"
          >
            <XIcon className="size-3.5" />
            {isCancelling
              ? t("central.portabilityCancelling")
              : t("central.portabilityCancel")}
          </Button>
        )
      }
    />
  );
}
