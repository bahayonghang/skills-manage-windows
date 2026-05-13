import { ExternalLink, FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { MarketplaceSkillDetail } from "./marketplaceSkillDetailTypes";

interface MarketplaceSkillDetailSummaryProps {
  skill: MarketplaceSkillDetail;
}

export function MarketplaceSkillDetailSummary({
  skill,
}: MarketplaceSkillDetailSummaryProps) {
  const { t } = useTranslation();

  return (
    <section className="grid gap-3 rounded-xl border border-border/70 bg-muted/15 p-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
      <div className="space-y-1">
        <div className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
          {t("marketplace.previewSourceLabel")}
        </div>
        <div className="text-sm font-medium text-foreground">
          {skill.sourceLabel ??
            skill.publisher ??
            t("marketplace.previewUnknownSource")}
        </div>
        <div className="break-all text-xs text-muted-foreground">
          {skill.downloadUrl}
        </div>
      </div>
      <div className="flex flex-wrap gap-2">
        {skill.sourceUrl ? (
          <a
            href={skill.sourceUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex h-9 items-center gap-2 rounded-md border border-input bg-background px-3 text-sm font-medium shadow-xs transition-[color,box-shadow] hover:bg-accent hover:text-accent-foreground"
          >
            <ExternalLink className="size-3.5" />
            <span>{t("marketplace.previewOpenSource")}</span>
          </a>
        ) : null}
        <a
          href={skill.downloadUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex h-9 items-center gap-2 rounded-md border border-input bg-background px-3 text-sm font-medium shadow-xs transition-[color,box-shadow] hover:bg-accent hover:text-accent-foreground"
        >
          <FileText className="size-3.5" />
          <span>{t("marketplace.previewOpenSkillMd")}</span>
        </a>
      </div>
    </section>
  );
}
