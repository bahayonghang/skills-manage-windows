import { useTranslation } from "react-i18next";
import { ArrowUpRight, Store, UploadCloud } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface StatusHeaderProps {
  onNavigate: (path: string) => void;
  scanState: string;
  scanStateLabel: string;
  lastScanLabel: string;
  centralTotal: number;
  sourceRepositoryCount: number;
  enabledTargetsCount: number;
  visibleTargetsCount: number;
  quickMigratePath: string;
  quickMigrateDescription: string;
}

const headerControlMotion =
  "transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]";

export function StatusHeader({
  onNavigate,
  scanState,
  scanStateLabel,
  lastScanLabel,
  centralTotal,
  sourceRepositoryCount,
  enabledTargetsCount,
  visibleTargetsCount,
  quickMigratePath,
  quickMigrateDescription,
}: StatusHeaderProps) {
  const { t } = useTranslation();
  const pillTone =
    scanState === "error"
      ? "border-destructive/30 bg-destructive/10 text-destructive-text"
      : scanState === "refreshing"
        ? "border-primary/35 bg-primary/15 text-primary-text"
        : "border-success/35 bg-success/10 text-success-foreground";

  return (
    <section className="surface-glass rounded-2xl px-4 py-3 sm:px-5">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
          <span
            className={cn(
              "inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs font-semibold uppercase tracking-wide",
              pillTone,
            )}
          >
            <span className="size-1.5 rounded-full bg-current" />
            {scanStateLabel}
          </span>
          <span className="shrink-0 text-ui-meta text-muted-foreground">
            {t("dashboard.statusHeader.lastScan", { time: lastScanLabel })}
          </span>
          <span
            aria-hidden="true"
            className="hidden h-3 w-px bg-border sm:block"
          />
          <span className="min-w-0 truncate text-xs text-muted-foreground">
            {t("dashboard.statusHeader.summary", {
              central: centralTotal,
              sources: sourceRepositoryCount,
              agents: enabledTargetsCount,
              agentsTotal: visibleTargetsCount,
            })}
          </span>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <Button
            size="sm"
            className={headerControlMotion}
            onClick={() => onNavigate("/central")}
          >
            <ArrowUpRight className="size-4" />
            {t("dashboard.hero.ctaBrowse")}
          </Button>
          <Button
            size="sm"
            variant="outline"
            className={headerControlMotion}
            data-testid="dashboard-action-marketplace"
            onClick={() => onNavigate("/marketplace")}
          >
            <Store className="size-4" />
            {t("dashboard.hero.ctaMarketplace")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className={headerControlMotion}
            title={quickMigrateDescription}
            data-testid="dashboard-action-quick-migrate"
            onClick={() => onNavigate(quickMigratePath)}
          >
            <UploadCloud className="size-4" />
            {t("dashboard.hero.ctaQuickMigrate")}
          </Button>
        </div>
      </div>
    </section>
  );
}
