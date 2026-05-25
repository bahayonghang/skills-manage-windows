import { Database, Globe, Info } from "lucide-react";
import { useTranslation } from "react-i18next";

import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";

interface AboutSettingsSectionProps {
  appVersion: string;
  dbPathDisplay: string;
  repoUrl: string;
}

export function AboutSettingsSection({
  appVersion,
  dbPathDisplay,
  repoUrl,
}: AboutSettingsSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsCollapsibleCard
      sectionId="about"
      title={t("settings.about")}
      icon={<Info className="size-5 shrink-0 text-muted-foreground" />}
    >
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <Info className="size-4 text-muted-foreground shrink-0" />
            <div>
              <div className="text-xs text-muted-foreground">{t("settings.appVersion")}</div>
              <div className="text-sm font-medium">SkillPort v{appVersion}</div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Database className="size-4 text-muted-foreground shrink-0" />
            <div>
              <div className="text-xs text-muted-foreground">{t("settings.dbPath")}</div>
              <div className="text-sm font-medium font-mono">{dbPathDisplay}</div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Globe className="size-4 text-muted-foreground shrink-0" />
            <div>
              <div className="text-xs text-muted-foreground">{t("settings.repoUrl")}</div>
              <a
                href={repoUrl}
                target="_blank"
                rel="noreferrer"
                className="text-sm font-medium text-primary hover:underline break-all"
              >
                {repoUrl}
              </a>
            </div>
          </div>
        </div>
    </SettingsCollapsibleCard>
  );
}
