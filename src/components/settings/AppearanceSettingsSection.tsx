import { Globe2, Palette, SlidersHorizontal, Type, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  BODY_FONT_OPTIONS,
  CHINESE_FALLBACK_OPTIONS,
  DEFAULT_FONT_PREFERENCES,
  DISPLAY_FONT_OPTIONS,
  FONT_SCALE_OPTIONS,
  applyFontPreferences,
  loadFontPreferences,
  saveBodyChineseFallback,
  saveBodyFont,
  saveDisplayChineseFallback,
  saveDisplayFont,
  saveFontScale,
  type BodyFontKey,
  type ChineseFallbackKey,
  type DisplayFontKey,
  type FontPreferences,
} from "@/lib/displayFont";
import { cn } from "@/lib/utils";
import type { CatppuccinAccent, ThemeFlavor } from "@/stores/themeStore";

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

interface FontOption {
  key: string;
  labelKey: string;
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
  const { t, i18n } = useTranslation();
  const [prefs, setPrefs] = useState<FontPreferences>(DEFAULT_FONT_PREFERENCES);
  const hasEditedPrefs = useRef(false);
  const currentLanguage = getCurrentLanguage(i18n.language);

  useEffect(() => {
    let cancelled = false;
    void loadFontPreferences().then((loaded) => {
      if (cancelled || hasEditedPrefs.current) return;
      setPrefs((current) =>
        fontPreferencesEqual(current, loaded) ? current : loaded,
      );
      applyFontPreferences(loaded);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleDisplayKey(key: DisplayFontKey) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, display: key }));
    await saveDisplayFont(
      key,
      prefs.displayCustom,
      prefs.displayChineseFallback,
      prefs.displayChineseFallbackCustom,
    );
  }

  async function handleDisplayCustom(custom: string) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, displayCustom: custom }));
    if (prefs.display === "custom") {
      await saveDisplayFont(
        "custom",
        custom,
        prefs.displayChineseFallback,
        prefs.displayChineseFallbackCustom,
      );
    }
  }

  async function handleDisplayChineseFallback(key: ChineseFallbackKey) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, displayChineseFallback: key }));
    await saveDisplayChineseFallback(
      key,
      prefs.displayChineseFallbackCustom,
      prefs.display,
      prefs.displayCustom,
    );
  }

  async function handleDisplayChineseFallbackCustom(custom: string) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({
      ...current,
      displayChineseFallbackCustom: custom,
    }));
    if (prefs.displayChineseFallback === "custom") {
      await saveDisplayChineseFallback(
        "custom",
        custom,
        prefs.display,
        prefs.displayCustom,
      );
    }
  }

  async function handleBodyKey(key: BodyFontKey) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, body: key }));
    await saveBodyFont(
      key,
      prefs.bodyCustom,
      prefs.bodyChineseFallback,
      prefs.bodyChineseFallbackCustom,
    );
  }

  async function handleBodyCustom(custom: string) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, bodyCustom: custom }));
    if (prefs.body === "custom") {
      await saveBodyFont(
        "custom",
        custom,
        prefs.bodyChineseFallback,
        prefs.bodyChineseFallbackCustom,
      );
    }
  }

  async function handleBodyChineseFallback(key: ChineseFallbackKey) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, bodyChineseFallback: key }));
    await saveBodyChineseFallback(
      key,
      prefs.bodyChineseFallbackCustom,
      prefs.body,
      prefs.bodyCustom,
    );
  }

  async function handleBodyChineseFallbackCustom(custom: string) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({
      ...current,
      bodyChineseFallbackCustom: custom,
    }));
    if (prefs.bodyChineseFallback === "custom") {
      await saveBodyChineseFallback(
        "custom",
        custom,
        prefs.body,
        prefs.bodyCustom,
      );
    }
  }

  async function handleScale(value: number) {
    hasEditedPrefs.current = true;
    setPrefs((current) => ({ ...current, scale: value }));
    await saveFontScale(value);
  }

  return (
    <div
      className="divide-y divide-border/70 border-y border-border/70"
      data-settings-section="appearance"
    >
      <SettingGroup
        icon={Palette}
        title={t("settings.appearance.themeGroup")}
        description={t("settings.appearance.themeGroupDesc")}
      >
        <div className="space-y-5">
          <ControlField label={t("settings.flavor")}>
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
              {flavorOrder.map((item) => {
                const active = flavor === item;
                return (
                  <button
                    key={item}
                    type="button"
                    aria-pressed={active}
                    onClick={() => onSetFlavor(item)}
                    className={cn(
                      "focus-ring flex min-h-10 items-center gap-2 rounded-lg px-3 text-left text-sm transition-[scale,background-color,color] active:scale-[0.96]",
                      active
                        ? "bg-primary/12 text-foreground"
                        : "bg-muted/35 text-muted-foreground hover:bg-muted/65 hover:text-foreground",
                    )}
                  >
                    <span
                      className="size-2.5 shrink-0 rounded-full"
                      style={{ backgroundColor: flavorColors[item] }}
                      aria-hidden="true"
                    />
                    <span className="truncate">{t(`settings.${item}`)}</span>
                  </button>
                );
              })}
            </div>
          </ControlField>

          <ControlField label={t("settings.accentColor")}>
            <div
              className="flex flex-wrap gap-1.5"
              role="radiogroup"
              aria-label={t("settings.accentColor")}
            >
              {accentNames.map((name) => {
                const selected = accent === name;
                return (
                  <button
                    key={name}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    aria-label={t(`settings.accent.${name}`)}
                    onClick={() => onSetAccent(name)}
                    className={cn(
                      "focus-ring relative grid size-10 place-items-center rounded-full transition-[scale,box-shadow] active:scale-[0.96]",
                      selected
                        ? "shadow-[0_0_0_2px_var(--background),0_0_0_4px_var(--ring)]"
                        : "hover:shadow-[0_0_0_2px_var(--background),0_0_0_3px_var(--border)]",
                    )}
                  >
                    <span
                      className="size-6 rounded-full"
                      style={{ backgroundColor: `var(${ctpVarMap[name]})` }}
                      aria-hidden="true"
                    />
                  </button>
                );
              })}
            </div>
          </ControlField>
        </div>
      </SettingGroup>

      <SettingGroup
        icon={Globe2}
        title={t("settings.appearance.languageGroup")}
        description={t("settings.appearance.languageGroupDesc")}
      >
        <SegmentedControl
          options={[
            { value: "zh", label: t("settings.chinese") },
            { value: "en", label: t("settings.english") },
          ]}
          value={currentLanguage}
          onChange={(value) => void i18n.changeLanguage(value)}
        />
      </SettingGroup>

      <SettingGroup
        icon={Type}
        title={t("settings.appearance.typographyGroup")}
        description={t("settings.appearance.typographyGroupDesc")}
      >
        <div className="divide-y divide-border/70">
          <FontRoleControl
            groupLabel={t("settings.appearance.displayFontOptionsLabel")}
            title={t("settings.appearance.displayFont")}
            primary={prefs.display}
            primaryOptions={DISPLAY_FONT_OPTIONS}
            primaryOptionNamespace="displayFontOption"
            primaryCustom={prefs.displayCustom}
            primaryCustomLabel={t("settings.appearance.displayCustomLabel")}
            chineseFallback={prefs.displayChineseFallback}
            chineseFallbackLabel={t(
              "settings.appearance.displayChineseFallback",
            )}
            chineseFallbackCustom={prefs.displayChineseFallbackCustom}
            chineseFallbackCustomLabel={t(
              "settings.appearance.displayChineseFallbackCustomLabel",
            )}
            specimenClassName="font-display"
            onPrimaryChange={(value) =>
              void handleDisplayKey(value as DisplayFontKey)
            }
            onPrimaryCustomChange={(value) => void handleDisplayCustom(value)}
            onChineseFallbackChange={(value) =>
              void handleDisplayChineseFallback(value)
            }
            onChineseFallbackCustomChange={(value) =>
              void handleDisplayChineseFallbackCustom(value)
            }
          />
          <FontRoleControl
            groupLabel={t("settings.appearance.bodyFontOptionsLabel")}
            title={t("settings.appearance.bodyFont")}
            primary={prefs.body}
            primaryOptions={BODY_FONT_OPTIONS}
            primaryOptionNamespace="bodyFontOption"
            primaryCustom={prefs.bodyCustom}
            primaryCustomLabel={t("settings.appearance.bodyCustomLabel")}
            chineseFallback={prefs.bodyChineseFallback}
            chineseFallbackLabel={t(
              "settings.appearance.bodyChineseFallback",
            )}
            chineseFallbackCustom={prefs.bodyChineseFallbackCustom}
            chineseFallbackCustomLabel={t(
              "settings.appearance.bodyChineseFallbackCustomLabel",
            )}
            specimenClassName="font-body"
            onPrimaryChange={(value) => void handleBodyKey(value as BodyFontKey)}
            onPrimaryCustomChange={(value) => void handleBodyCustom(value)}
            onChineseFallbackChange={(value) =>
              void handleBodyChineseFallback(value)
            }
            onChineseFallbackCustomChange={(value) =>
              void handleBodyChineseFallbackCustom(value)
            }
          />
        </div>
      </SettingGroup>

      <SettingGroup
        icon={SlidersHorizontal}
        title={t("settings.appearance.densityGroup")}
        description={t("settings.appearance.densityGroupDesc")}
      >
        <div className="space-y-4">
          <SegmentedControl
            options={FONT_SCALE_OPTIONS.map((option) => ({
              value: String(option.value),
              label: t(`settings.appearance.${option.labelKey}`),
            }))}
            value={String(prefs.scale)}
            onChange={(value) => void handleScale(Number(value))}
          />
          <div className="flex min-h-16 items-center justify-between gap-4 rounded-lg bg-muted/35 px-4 py-3 ring-1 ring-border/60">
            <span
              className="font-display font-semibold tabular-nums"
              style={{ fontSize: `${prefs.scale}rem` }}
            >
              {t("settings.appearance.scaleSpecimen")}
            </span>
            <span className="text-sm tabular-nums text-muted-foreground">
              {Math.round(prefs.scale * 100)}%
            </span>
          </div>
        </div>
      </SettingGroup>
    </div>
  );
}

function SettingGroup({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="grid gap-4 py-6 md:grid-cols-[minmax(11rem,0.34fr)_minmax(0,0.66fr)] md:gap-8">
      <div className="flex items-start gap-2.5">
        <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        <div className="min-w-0">
          <h2 className="text-balance font-heading text-sm font-semibold leading-5">
            {title}
          </h2>
          <p className="mt-1 text-pretty text-xs leading-5 text-muted-foreground">
            {description}
          </p>
        </div>
      </div>
      <div className="min-w-0">{children}</div>
    </section>
  );
}

function ControlField({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="text-sm font-medium text-foreground">{label}</div>
      {children}
    </div>
  );
}

function SegmentedControl({
  options,
  value,
  onChange,
}: {
  options: readonly { value: string; label: string }[];
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="inline-flex min-h-10 max-w-full flex-wrap gap-1 rounded-lg bg-muted/45 p-1">
      {options.map((option) => {
        const active = value === option.value;
        return (
          <Button
            key={option.value}
            type="button"
            size="sm"
            variant="ghost"
            aria-pressed={active}
            onClick={() => onChange(option.value)}
            className={cn(
              "min-h-8 px-3",
              active
                ? "bg-background text-foreground shadow-sm hover:bg-background"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {option.label}
          </Button>
        );
      })}
    </div>
  );
}

function FontRoleControl({
  groupLabel,
  title,
  primary,
  primaryOptions,
  primaryOptionNamespace,
  primaryCustom,
  primaryCustomLabel,
  chineseFallback,
  chineseFallbackLabel,
  chineseFallbackCustom,
  chineseFallbackCustomLabel,
  specimenClassName,
  onPrimaryChange,
  onPrimaryCustomChange,
  onChineseFallbackChange,
  onChineseFallbackCustomChange,
}: {
  groupLabel: string;
  title: string;
  primary: string;
  primaryOptions: readonly FontOption[];
  primaryOptionNamespace: "displayFontOption" | "bodyFontOption";
  primaryCustom: string;
  primaryCustomLabel: string;
  chineseFallback: ChineseFallbackKey;
  chineseFallbackLabel: string;
  chineseFallbackCustom: string;
  chineseFallbackCustomLabel: string;
  specimenClassName: string;
  onPrimaryChange: (value: string) => void;
  onPrimaryCustomChange: (value: string) => void;
  onChineseFallbackChange: (value: ChineseFallbackKey) => void;
  onChineseFallbackCustomChange: (value: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-4 py-5 first:pt-0 last:pb-0" role="group" aria-label={groupLabel}>
      <div className="grid gap-3 sm:grid-cols-[minmax(7rem,0.3fr)_minmax(0,0.7fr)] sm:items-start">
        <div className="text-sm font-medium leading-10">{title}</div>
        <div className="space-y-3">
          <label className="block space-y-1.5">
            <span className="text-xs text-muted-foreground">
              {t("settings.appearance.primaryFont")}
            </span>
            <select
              value={primary}
              onChange={(event) => onPrimaryChange(event.target.value)}
              aria-label={title}
              className="focus-ring h-10 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground"
            >
              {primaryOptions.map((option) => (
                <option key={option.key} value={option.key}>
                  {t(
                    `settings.appearance.${primaryOptionNamespace}.${option.labelKey}`,
                  )}
                </option>
              ))}
            </select>
          </label>
          {primary === "custom" ? (
            <Input
              className="h-10"
              placeholder={t("settings.appearance.customPlaceholder")}
              value={primaryCustom}
              onChange={(event) => onPrimaryCustomChange(event.target.value)}
              aria-label={primaryCustomLabel}
            />
          ) : null}
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-[minmax(7rem,0.3fr)_minmax(0,0.7fr)] sm:items-start">
        <div className="text-sm font-medium leading-10">{chineseFallbackLabel}</div>
        <div className="space-y-3">
          <select
            value={chineseFallback}
            onChange={(event) =>
              onChineseFallbackChange(event.target.value as ChineseFallbackKey)
            }
            aria-label={chineseFallbackLabel}
            className="focus-ring h-10 w-full rounded-lg border border-input bg-background px-3 text-sm text-foreground"
          >
            {CHINESE_FALLBACK_OPTIONS.map((option) => (
              <option key={option.key} value={option.key}>
                {t(
                  `settings.appearance.chineseFallbackOption.${option.labelKey}`,
                )}
              </option>
            ))}
          </select>
          {chineseFallback === "custom" ? (
            <Input
              className="h-10"
              placeholder={t("settings.appearance.customPlaceholder")}
              value={chineseFallbackCustom}
              onChange={(event) =>
                onChineseFallbackCustomChange(event.target.value)
              }
              aria-label={chineseFallbackCustomLabel}
            />
          ) : null}
        </div>
      </div>

      <div
        className={cn(
          "min-h-16 rounded-lg bg-muted/35 px-4 py-3 ring-1 ring-border/60",
          specimenClassName,
        )}
      >
        <div className="text-base font-semibold">
          {t("settings.appearance.mixedSpecimen")}
        </div>
        <div className="mt-1 text-xs text-muted-foreground">Aa 0123</div>
      </div>
    </div>
  );
}

function getCurrentLanguage(language: string): "zh" | "en" {
  return language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

function fontPreferencesEqual(left: FontPreferences, right: FontPreferences) {
  return (
    left.display === right.display &&
    left.displayCustom === right.displayCustom &&
    left.displayChineseFallback === right.displayChineseFallback &&
    left.displayChineseFallbackCustom === right.displayChineseFallbackCustom &&
    left.body === right.body &&
    left.bodyCustom === right.bodyCustom &&
    left.bodyChineseFallback === right.bodyChineseFallback &&
    left.bodyChineseFallbackCustom === right.bodyChineseFallbackCustom &&
    Math.abs(left.scale - right.scale) < 1e-6
  );
}
