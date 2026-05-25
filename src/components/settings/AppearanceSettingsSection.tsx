import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Droplets, Globe, Palette, Type } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";
import i18n from "@/i18n";
import type { CatppuccinAccent, ThemeFlavor } from "@/stores/themeStore";
import {
  BODY_FONT_OPTIONS,
  DEFAULT_FONT_PREFERENCES,
  DISPLAY_FONT_OPTIONS,
  FONT_SCALE_OPTIONS,
  applyFontPreferences,
  loadFontPreferences,
  saveBodyFont,
  saveDisplayFont,
  saveFontScale,
  type BodyFontKey,
  type DisplayFontKey,
  type FontPreferences,
} from "@/lib/displayFont";
import { cn } from "@/lib/utils";

interface AppearanceSettingsSectionProps {
  accent: CatppuccinAccent;
  accentNames: CatppuccinAccent[];
  ctpVarMap: Record<CatppuccinAccent, string>;
  flavor: ThemeFlavor;
  flavorColors: Record<ThemeFlavor, string>;
  flavorOrder: ThemeFlavor[];
  onSetAccent: (accent: CatppuccinAccent) => void;
  onSetFlavor: (flavor: ThemeFlavor) => void;
}

export function AppearanceSettingsSection({
  accent,
  accentNames,
  ctpVarMap,
  flavor,
  flavorColors,
  flavorOrder,
  onSetAccent,
  onSetFlavor,
}: AppearanceSettingsSectionProps) {
  const { t } = useTranslation();
  const [prefs, setPrefs] = useState<FontPreferences>(DEFAULT_FONT_PREFERENCES);

  useEffect(() => {
    let cancelled = false;
    void loadFontPreferences().then((loaded) => {
      if (cancelled) return;
      setPrefs(loaded);
      applyFontPreferences(loaded);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleDisplayKey(key: DisplayFontKey) {
    const next = { ...prefs, display: key };
    setPrefs(next);
    await saveDisplayFont(key, prefs.displayCustom);
  }

  async function handleDisplayCustom(custom: string) {
    const next = { ...prefs, displayCustom: custom };
    setPrefs(next);
    if (prefs.display === "custom") {
      await saveDisplayFont("custom", custom);
    }
  }

  async function handleBodyKey(key: BodyFontKey) {
    const next = { ...prefs, body: key };
    setPrefs(next);
    await saveBodyFont(key, prefs.bodyCustom);
  }

  async function handleBodyCustom(custom: string) {
    const next = { ...prefs, bodyCustom: custom };
    setPrefs(next);
    if (prefs.body === "custom") {
      await saveBodyFont("custom", custom);
    }
  }

  async function handleScale(value: number) {
    const next = { ...prefs, scale: value };
    setPrefs(next);
    await saveFontScale(value);
  }

  return (
    <SettingsCollapsibleCard
      sectionId="appearance"
      title={t("settings.appearance.title")}
      description={t("settings.appearance.description")}
      icon={<Type className="size-4 text-muted-foreground shrink-0" />}
    >
      <div className="space-y-5">
        <div className="grid gap-4 lg:grid-cols-2">
          <div className="rounded-xl border border-border/80 bg-background/70 p-4">
            <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <Palette className="size-3.5" />
              {t("settings.flavor")}
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {flavorOrder.map((item) => (
                <Button
                  key={item}
                  variant={flavor === item ? "default" : "outline"}
                  size="sm"
                  onClick={() => onSetFlavor(item)}
                  aria-pressed={flavor === item}
                >
                  <span
                    className="mr-1.5 inline-block size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: flavorColors[item] }}
                  />
                  {t(`settings.${item}`)}
                </Button>
              ))}
            </div>
          </div>

          <div className="rounded-xl border border-border/80 bg-background/70 p-4">
            <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <Globe className="size-3.5" />
              {t("settings.language")}
            </div>
            <div className="mt-3 flex gap-2">
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

        <div className="rounded-xl border border-border/80 bg-background/70 p-4">
          <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            <Droplets className="size-3.5" />
            {t("settings.accentColor")}
          </div>
          <div
            className="mt-3 flex flex-wrap gap-1.5"
            role="radiogroup"
            aria-label={t("settings.accentColor")}
          >
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
                  className={cn(
                    "relative size-8 rounded-full transition-colors active:scale-95 cursor-pointer md:size-6",
                    isActive
                      ? "ring-2 ring-ring ring-offset-2 ring-offset-background scale-110"
                      : "ring-1 ring-border hover:scale-105 hover:ring-2 hover:ring-ring/50"
                  )}
                  style={{ backgroundColor: `var(${ctpVar})` }}
                />
              );
            })}
          </div>
        </div>

        {/* Display font */}
        <div>
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("settings.appearance.displayFont")}
          </div>
          <div className="mt-2 grid max-w-2xl grid-cols-2 gap-2 sm:grid-cols-3 xl:max-w-xl">
            {DISPLAY_FONT_OPTIONS.map((option) => {
              const active = prefs.display === option.key;
              return (
                <Button
                  key={option.key}
                  variant="outline"
                  size="sm"
                  className={cn(
                    "max-w-44 justify-start",
                    active &&
                      "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]"
                  )}
                  onClick={() => void handleDisplayKey(option.key)}
                  aria-pressed={active}
                >
                  <span
                    aria-hidden="true"
                    className={cn(
                      "mr-2 inline-block text-base leading-none",
                      previewClassFor(option.key),
                    )}
                  >
                    {option.sample}
                  </span>
                  <span className="truncate text-xs">
                    {t(`settings.appearance.displayFontOption.${option.labelKey}`)}
                  </span>
                </Button>
              );
            })}
          </div>
          {prefs.display === "custom" && (
            <Input
              className="mt-2"
              placeholder={t("settings.appearance.customPlaceholder")}
              value={prefs.displayCustom}
              onChange={(event) => {
                void handleDisplayCustom(event.target.value);
              }}
              aria-label={t("settings.appearance.displayCustomLabel")}
            />
          )}
        </div>

        {/* Body font */}
        <div>
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("settings.appearance.bodyFont")}
          </div>
          <div className="mt-2 grid max-w-2xl grid-cols-2 gap-2 sm:grid-cols-3 xl:max-w-xl">
            {BODY_FONT_OPTIONS.map((option) => {
              const active = prefs.body === option.key;
              return (
                <Button
                  key={option.key}
                  variant="outline"
                  size="sm"
                  className={cn(
                    "max-w-44 justify-start",
                    active &&
                      "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]"
                  )}
                  onClick={() => void handleBodyKey(option.key)}
                  aria-pressed={active}
                >
                  <span className="truncate text-xs">
                    {t(`settings.appearance.bodyFontOption.${option.labelKey}`)}
                  </span>
                </Button>
              );
            })}
          </div>
          {prefs.body === "custom" && (
            <Input
              className="mt-2"
              placeholder={t("settings.appearance.customPlaceholder")}
              value={prefs.bodyCustom}
              onChange={(event) => {
                void handleBodyCustom(event.target.value);
              }}
              aria-label={t("settings.appearance.bodyCustomLabel")}
            />
          )}
        </div>

        {/* Font scale */}
        <div>
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("settings.appearance.scale")}
          </div>
          <div className="mt-2 flex max-w-xl flex-wrap gap-2">
            {FONT_SCALE_OPTIONS.map((option) => {
              const active = Math.abs(prefs.scale - option.value) < 1e-3;
              return (
                <Button
                  key={option.value}
                  variant="outline"
                  size="sm"
                  className={cn(
                    active &&
                      "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]"
                  )}
                  onClick={() => void handleScale(option.value)}
                  aria-pressed={active}
                >
                  {t(`settings.appearance.${option.labelKey}`)}
                </Button>
              );
            })}
          </div>
        </div>

        {/* Live preview */}
        <div className="rounded-xl border border-[color:var(--settings-section-accent-border)] bg-[linear-gradient(135deg,var(--settings-section-accent-soft),var(--settings-section-accent-faint))] px-4 py-4 shadow-inner">
          <div className="text-[0.65rem] font-semibold uppercase tracking-[0.18em] text-[color:var(--settings-section-accent)]">
            {t("settings.appearance.preview")}
          </div>
          <div className="mt-2 font-display text-3xl font-semibold tracking-tight">
            {t("settings.appearance.previewTitle")}
          </div>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            {t("settings.appearance.previewBody")}
          </p>
        </div>
      </div>
    </SettingsCollapsibleCard>
  );
}

function previewClassFor(key: DisplayFontKey): string {
  switch (key) {
    case "serif":
      return "[font-family:'Instrument_Serif',Georgia,serif]";
    case "inter":
      return "[font-family:'Inter_Variable',sans-serif]";
    case "jetbrains":
      return "[font-family:'JetBrains_Mono_Variable',monospace]";
    case "system":
      return "[font-family:ui-sans-serif,system-ui]";
    case "custom":
      return "italic";
    case "geist":
    default:
      return "[font-family:'Geist_Variable',sans-serif]";
  }
}
