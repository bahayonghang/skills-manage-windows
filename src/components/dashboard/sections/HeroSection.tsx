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
    <section className="surface-glass relative overflow-hidden rounded-3xl px-6 py-7 sm:px-8 sm:py-9">
      <span
        aria-hidden="true"
        className="dashboard-hero-glow pointer-events-none absolute -right-24 -top-16 h-64 w-64 rounded-full opacity-60 blur-3xl"
      />
      <div className="relative flex max-w-3xl flex-col gap-5">
        <div className="inline-flex items-center gap-2 self-start rounded-full border border-border/70 bg-card/60 px-2.5 py-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
          <span
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[0.65rem] font-semibold uppercase tracking-wide",
              pillTone,
            )}
          >
            <span className="size-1.5 rounded-full bg-current" />
            {scanStateLabel}
          </span>
          <span>{t("dashboard.hero.eyebrow")}</span>
        </div>
        <h1 className="font-display text-balance text-4xl font-semibold leading-[1.05] tracking-tight sm:text-5xl">
          {t("dashboard.hero.title")}
        </h1>
        <p className="max-w-2xl text-pretty text-sm leading-7 text-muted-foreground sm:text-base sm:leading-7">
          {t("dashboard.hero.description")}
        </p>
        <div className="flex flex-wrap gap-2 pt-1">
          <Button onClick={() => onNavigate("/central")}>
            <ArrowUpRight className="size-4" />
            {t("dashboard.hero.ctaBrowse")}
          </Button>
          {showReviewCta && (
            <Button
              variant="outline"
              onClick={() => onNavigate("/central?filter=uncategorized")}
            >
              {t("dashboard.hero.ctaReview", { count: uncategorizedCount })}
              <ChevronRight className="size-4" />
            </Button>
          )}
          <Button
            variant="outline"
            data-testid="dashboard-action-marketplace"
            onClick={() => onNavigate("/marketplace")}
          >
            <Store className="size-4" />
            {t("dashboard.hero.ctaMarketplace")}
          </Button>
          <Button
            variant="outline"
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
