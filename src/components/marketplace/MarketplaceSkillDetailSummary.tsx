import { Download, ExternalLink, FileText, Star } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { MarketplaceSkillDetail } from "./marketplaceSkillDetailTypes";

interface MarketplaceSkillDetailSummaryProps {
  skill: MarketplaceSkillDetail;
  downloadUrl?: string | null;
}

export function MarketplaceSkillDetailSummary({
  downloadUrl,
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
          {downloadUrl ?? skill.downloadUrl}
        </div>
        {skill.remoteKind === "skills_sh" ? (
          <div className="mt-2 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
            <span className="inline-flex items-center gap-1 rounded-full bg-muted/50 px-2 py-0.5">
              <Download className="size-3" />
              {t("marketplace.skillsShInstalls", { count: skill.installs ?? 0 })}
            </span>
            {typeof skill.stars === "number" ? (
              <span className="inline-flex items-center gap-1 rounded-full bg-muted/50 px-2 py-0.5">
                <Star className="size-3" />
                {t("marketplace.skillsShStars", { count: skill.stars })}
              </span>
            ) : null}
          </div>
        ) : null}
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
          href={downloadUrl ?? skill.downloadUrl}
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
