import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowUpRight, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ActivityHeatmap } from "@/components/usage/ActivityHeatmap";
import { UsageMatchStatus } from "@/components/usage/SkillUsageTable";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import { projectShort } from "@/lib/timeAgo";
import type { SkillUsageDetail } from "@/types/usage";

interface SkillUsageDetailPanelProps {
  detail: SkillUsageDetail | null;
  loading: boolean;
  onClose: () => void;
}

export function SkillUsageDetailPanel({
  detail,
  loading,
  onClose,
}: SkillUsageDetailPanelProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();

  if (loading || !detail) {
    return (
      <div
        data-testid="usage-detail-loading"
        className="min-h-[22rem] animate-pulse px-4 py-4"
      >
        <div className="h-5 w-2/5 rounded bg-muted" />
        <div className="mt-5 grid grid-cols-2 gap-3">
          <div className="h-14 rounded bg-muted/70" />
          <div className="h-14 rounded bg-muted/70" />
        </div>
        <div className="mt-5 h-32 rounded bg-muted/50" />
      </div>
    );
  }

  return (
    <div data-testid="usage-detail-panel" className="min-h-[22rem]">
      <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-semibold text-foreground">
            {detail.skill}
          </h3>
          <UsageMatchStatus status={detail.matchStatus} className="mt-0.5" />
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {detail.resolvedSkillId && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() =>
                navigate(`/skill/${encodeURIComponent(detail.resolvedSkillId!)}`)
              }
            >
              <ArrowUpRight className="size-3.5" />
              {t("skillUsage.openSkill")}
            </Button>
          )}
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            onClick={onClose}
            title={t("skillUsage.closeDetail")}
            aria-label={t("skillUsage.closeDetail")}
          >
            <X className="size-4" />
          </Button>
        </div>
      </div>

      <dl className="grid grid-cols-2 divide-x divide-border border-b border-border sm:grid-cols-4">
        <DetailMetric label={t("skillUsage.kpi.calls")} value={detail.count} />
        <DetailMetric label={t("skillUsage.kpi.sessions")} value={detail.sessions} />
        <DetailMetric
          label={t("skillUsage.detail.firstUsed")}
          value={
            detail.firstUsedMs > 0
              ? new Intl.DateTimeFormat(i18n.language || undefined, {
                  dateStyle: "medium",
                }).format(detail.firstUsedMs)
              : "—"
          }
        />
        <DetailMetric
          label={t("skillUsage.detail.lastUsed")}
          value={
            detail.lastUsedMs > 0
              ? formatUsageRelativeTime(detail.lastUsedMs, i18n.language)
              : "—"
          }
        />
      </dl>

      <div className="grid gap-4 px-4 py-4">
        <div>
          <div className="mb-2 flex items-center justify-between gap-3">
            <h4 className="text-xs font-medium text-foreground">
              {t("skillUsage.detail.projects")}
            </h4>
            <span
              className="text-xs tabular-nums text-muted-foreground"
              title={t("skillUsage.staticEstimateTooltip", {
                bytes: detail.staticByteCount?.toLocaleString() ?? "—",
              })}
            >
              {t("skillUsage.detail.staticEstimate")}: {" "}
              {detail.staticTokenEstimate === null
                ? t("skillUsage.unavailable")
                : `~${detail.staticTokenEstimate.toLocaleString()}`}
            </span>
          </div>
          <ul className="divide-y divide-border/60 border-y border-border/70">
            {detail.byProject.map((project) => (
              <li
                key={project.project}
                className="grid grid-cols-[minmax(0,1fr)_4rem_4rem] gap-2 py-2 text-xs"
              >
                <span className="truncate text-foreground">
                  {projectShort(project.project)}
                </span>
                <span className="text-right tabular-nums text-muted-foreground">
                  {project.count} {t("skillUsage.callsShort")}
                </span>
                <span className="text-right tabular-nums text-muted-foreground">
                  {project.sessions} {t("skillUsage.sessionsShort")}
                </span>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h4 className="mb-2 text-xs font-medium text-foreground">
            {t("skillUsage.detail.activity")}
          </h4>
          <ActivityHeatmap days={detail.weekly} compact />
        </div>
      </div>
    </div>
  );
}

function DetailMetric({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="min-w-0 px-3 py-2.5">
      <dt className="truncate text-ui-meta text-muted-foreground">{label}</dt>
      <dd className="mt-0.5 truncate text-sm font-medium tabular-nums text-foreground">
        {typeof value === "number" ? value.toLocaleString() : value}
      </dd>
    </div>
  );
}
