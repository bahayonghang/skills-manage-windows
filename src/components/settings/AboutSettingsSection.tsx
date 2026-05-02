import { Database, Droplets, Globe, Info, Palette } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import i18n from "@/i18n";
import type { CatppuccinAccent, ThemeFlavor } from "@/stores/themeStore";

interface AboutSettingsSectionProps {
  accent: CatppuccinAccent;
  appVersion: string;
  dbPathDisplay: string;
  flavor: ThemeFlavor;
  repoUrl: string;
  accentNames: CatppuccinAccent[];
  flavorColors: Record<ThemeFlavor, string>;
  flavorOrder: ThemeFlavor[];
  ctpVarMap: Record<CatppuccinAccent, string>;
  onSetAccent: (accent: CatppuccinAccent) => void;
  onSetFlavor: (flavor: ThemeFlavor) => void;
}

export function AboutSettingsSection({
  accent,
  appVersion,
  dbPathDisplay,
  flavor,
  repoUrl,
  accentNames,
  flavorColors,
  flavorOrder,
  ctpVarMap,
  onSetAccent,
  onSetFlavor,
}: AboutSettingsSectionProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("settings.about")}</CardTitle>
      </CardHeader>
      <CardContent>
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
          <div className="flex items-center gap-3">
            <Palette className="size-4 text-muted-foreground shrink-0" />
            <div className="flex-1">
              <div className="text-xs text-muted-foreground mb-1.5">{t("settings.flavor")}</div>
              <div className="flex gap-2">
                {flavorOrder.map((item) => (
                  <Button
                    key={item}
                    variant={flavor === item ? "default" : "outline"}
                    size="sm"
                    onClick={() => onSetFlavor(item)}
                    aria-pressed={flavor === item}
                  >
                    <span
                      className="inline-block size-2 rounded-full mr-1.5 shrink-0"
                      style={{ backgroundColor: flavorColors[item] }}
                    />
                    {t(`settings.${item}`)}
                  </Button>
                ))}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Droplets className="size-4 text-muted-foreground shrink-0" />
            <div className="flex-1">
              <div className="text-xs text-muted-foreground mb-1.5">{t("settings.accentColor")}</div>
              <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label={t("settings.accentColor")}>
                {accentNames.map((name) => {
                  const ctpVar = ctpVarMap[name];
                  const isActive = accent === name;
                  return (
                    <button
                      key={name}
                      type="button"
                      role="radio"
                      aria-checked={isActive}
                      aria-label={t(`settings.accent.${name}`)}
                      title={t(`settings.accent.${name}`)}
                      onClick={() => onSetAccent(name)}
                      className={`relative size-8 rounded-full transition-colors cursor-pointer md:size-6
                        ${isActive
                          ? "ring-2 ring-ring ring-offset-2 ring-offset-background scale-110"
                          : "ring-1 ring-border hover:scale-105 hover:ring-2 hover:ring-ring/50"
                        }`}
                      style={{ backgroundColor: `var(${ctpVar})` }}
                    />
                  );
                })}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Globe className="size-4 text-muted-foreground shrink-0" />
            <div className="flex-1">
              <div className="text-xs text-muted-foreground mb-1.5">{t("settings.language")}</div>
              <div className="flex gap-2">
                <Button
                  variant={i18n.language === "zh" ? "default" : "outline"}
                  size="sm"
                  onClick={() => i18n.changeLanguage("zh")}
                  aria-pressed={i18n.language === "zh"}
                >
                  {t("settings.chinese")}
                </Button>
                <Button
                  variant={i18n.language === "en" ? "default" : "outline"}
                  size="sm"
                  onClick={() => i18n.changeLanguage("en")}
                  aria-pressed={i18n.language === "en"}
                >
                  {t("settings.english")}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
