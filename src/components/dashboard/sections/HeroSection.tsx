import { useTranslation } from "react-i18next";
import { ArrowUpRight, ChevronRight, Store, UploadCloud } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface HeroSectionProps {
  onNavigate: (path: string) => void;
  uncategorizedCount: number;
  scanStateLabel: string;
  scanState: string;
  quickMigratePath: string;
  quickMigrateDescription: string;
}

export function HeroSection({
  onNavigate,
  uncategorizedCount,
  scanStateLabel,
  scanState,
  quickMigratePath,
  quickMigrateDescription,
}: HeroSectionProps) {
  const { t } = useTranslation();
  const showReviewCta = uncategorizedCount > 0;
  const pillTone =
    scanState === "error"
      ? "border-destructive/30 bg-destructive/10 text-destructive"
      : scanState === "refreshing"
        ? "border-primary/35 bg-primary/15 text-primary-text"
        : "border-success/35 bg-success/10 text-success-foreground";

  return (
    <section className="surface-glass relative overflow-hidden rounded-2xl px-6 py-6 sm:px-8 sm:py-8">
      <span
        aria-hidden="true"
        className="dashboard-hero-glow pointer-events-none absolute -right-24 -top-16 h-56 w-56 rounded-full opacity-45 blur-3xl"
      />
      <div className="relative flex max-w-[46rem] flex-col gap-4">
        <div className="inline-flex items-center gap-2 self-start rounded-full border border-border/70 bg-card/60 px-2.5 py-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
          <span
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[0.68rem] font-semibold uppercase tracking-wide",
              pillTone,
            )}
          >
            <span className="size-1.5 rounded-full bg-current" />
            {scanStateLabel}
          </span>
          <span>{t("dashboard.hero.eyebrow")}</span>
        </div>
        <h1 className="font-display text-balance text-[2.35rem] font-semibold leading-[1.08] sm:text-[3rem] xl:text-[3.25rem]">
          {t("dashboard.hero.title")}
        </h1>
        <p className="max-w-[42rem] text-pretty text-sm leading-6 text-muted-foreground sm:text-[0.95rem] sm:leading-7">
          {t("dashboard.hero.description")}
        </p>
        <div className="flex flex-wrap gap-2 pt-1">
          <Button
            className="h-10 px-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            onClick={() => onNavigate("/central")}
          >
            <ArrowUpRight className="size-4" />
            {t("dashboard.hero.ctaBrowse")}
          </Button>
          {showReviewCta && (
            <Button
              variant="outline"
              className="h-10 px-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
              onClick={() => onNavigate("/central?filter=uncategorized")}
            >
              {t("dashboard.hero.ctaReview", { count: uncategorizedCount })}
              <ChevronRight className="size-4" />
            </Button>
          )}
          <Button
            variant="outline"
            className="h-10 px-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            data-testid="dashboard-action-marketplace"
            onClick={() => onNavigate("/marketplace")}
          >
            <Store className="size-4" />
            {t("dashboard.hero.ctaMarketplace")}
          </Button>
          <Button
            variant="outline"
            className="h-10 px-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
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
