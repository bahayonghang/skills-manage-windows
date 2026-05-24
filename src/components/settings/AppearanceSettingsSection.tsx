import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Type } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";
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

export function AppearanceSettingsSection() {
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
