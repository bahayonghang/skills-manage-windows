import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  ALargeSmall,
  Check,
  Droplets,
  Globe2,
  Palette,
  SlidersHorizontal,
  Sparkles,
  Type,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";
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

type FontPreviewKey = DisplayFontKey | BodyFontKey;

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
      if (cancelled) return;
      if (hasEditedPrefs.current) return;
      setPrefs((current) =>
        fontPreferencesEqual(current, loaded) ? current : loaded,
      );
      applyFontPreferences(loaded);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const displayLabel = t(
    `settings.appearance.displayFontOption.${getDisplayOption(prefs.display).labelKey}`,
  );
  const bodyLabel = t(
    `settings.appearance.bodyFontOption.${getBodyOption(prefs.body).labelKey}`,
  );
  const scaleOption = getScaleOption(prefs.scale);
  const scaleLabel = t(`settings.appearance.${scaleOption.labelKey}`);
  const accentLabel = t(`settings.accent.${accent}`);
  const flavorLabel = t(`settings.${flavor}`);
  const languageLabel = t(
    currentLanguage === "zh" ? "settings.chinese" : "settings.english",
  );
  const scalePercent = `${Math.round(prefs.scale * 100)}%`;
  const previewScaleStyle = useMemo(
    () => ({ fontSize: `${prefs.scale}rem` }),
    [prefs.scale],
  );

  async function handleDisplayKey(key: DisplayFontKey) {
    hasEditedPrefs.current = true;
    const next = { ...prefs, display: key };
    setPrefs(next);
    await saveDisplayFont(key, prefs.displayCustom);
  }

  async function handleDisplayCustom(custom: string) {
    hasEditedPrefs.current = true;
    const next = { ...prefs, displayCustom: custom };
    setPrefs(next);
    if (prefs.display === "custom") {
      await saveDisplayFont("custom", custom);
    }
  }

  async function handleBodyKey(key: BodyFontKey) {
    hasEditedPrefs.current = true;
    const next = { ...prefs, body: key };
    setPrefs(next);
    await saveBodyFont(key, prefs.bodyCustom);
  }

  async function handleBodyCustom(custom: string) {
    hasEditedPrefs.current = true;
    const next = { ...prefs, bodyCustom: custom };
    setPrefs(next);
    if (prefs.body === "custom") {
      await saveBodyFont("custom", custom);
    }
  }

  async function handleScale(value: number) {
    hasEditedPrefs.current = true;
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
      <div className="space-y-4">
        <section className="relative overflow-hidden rounded-xl border border-[color:var(--settings-section-accent-border)] bg-[radial-gradient(circle_at_12%_10%,var(--settings-section-accent-soft),transparent_32rem),linear-gradient(135deg,var(--card),var(--background))] p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] sm:p-5 xl:p-6">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 bg-[linear-gradient(90deg,var(--settings-section-accent-faint)_1px,transparent_1px),linear-gradient(0deg,var(--settings-section-accent-faint)_1px,transparent_1px)] [background-size:2.4rem_2.4rem] opacity-30 [mask-image:linear-gradient(135deg,black,transparent_72%)]"
          />
          <div className="relative grid gap-5 xl:grid-cols-[1.12fr_0.88fr] xl:items-stretch">
            <div className="flex min-w-0 flex-col justify-between gap-5">
              <div>
                <div className="inline-flex items-center gap-2 rounded-full border border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] px-3 py-1 text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-[color:var(--settings-section-accent-text)]">
                  <Sparkles className="size-3.5" aria-hidden="true" />
                  {t("settings.appearance.labEyebrow")}
                </div>
                <h3 className="mt-4 max-w-3xl font-display text-3xl font-semibold leading-[1.03] tracking-[-0.055em] text-foreground sm:text-4xl xl:text-5xl">
                  {t("settings.appearance.labTitle")}
                </h3>
                <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
                  {t("settings.appearance.labSubtitle")}
                </p>
              </div>

              <div className="grid gap-2 sm:grid-cols-2 2xl:grid-cols-4">
                <StatusChip label={t("settings.flavor")} value={flavorLabel} />
                <StatusChip
                  label={t("settings.language")}
                  value={languageLabel}
                />
                <StatusChip
                  label={t("settings.accentColor")}
                  value={accentLabel}
                />
                <StatusChip
                  label={t("settings.appearance.scale")}
                  value={scaleLabel}
                />
              </div>
            </div>

            <div className="min-w-0 rounded-lg bg-card p-3 shadow-sm ring-1 ring-border sm:p-4">
              <div className="flex items-center justify-between gap-3 border-b border-border/70 pb-3">
                <div className="flex items-center gap-1.5" aria-hidden="true">
                  <span className="size-2.5 rounded-full bg-[color:var(--settings-section-accent)]" />
                  <span className="size-2.5 rounded-full bg-muted" />
                  <span className="size-2.5 rounded-full bg-muted/60" />
                </div>
                <div className="truncate text-[0.66rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                  {t("settings.appearance.previewSurface")}
                </div>
              </div>

              <div className="grid gap-3 pt-4 sm:grid-cols-[0.42fr_1fr]">
                <div className="hidden min-w-0 rounded-md bg-card/80 p-3 ring-1 ring-border/70 sm:block">
                  <div className="mb-3 h-2 w-12 rounded-full bg-[color:var(--settings-section-accent)]" />
                  <div className="space-y-2" aria-hidden="true">
                    <span className="block h-2 rounded-full bg-muted" />
                    <span className="block h-2 w-4/5 rounded-full bg-muted/80" />
                    <span className="block h-2 w-3/5 rounded-full bg-muted/60" />
                  </div>
                </div>

                <div className="min-w-0 rounded-md bg-[linear-gradient(145deg,var(--settings-section-accent-soft),var(--card))] p-4 ring-1 ring-[color:var(--settings-section-accent-border)]">
                  <div className="text-[0.64rem] font-semibold uppercase tracking-[0.2em] text-[color:var(--settings-section-accent-text)]">
                    {t("settings.appearance.previewKicker")}
                  </div>
                  <div
                    className="mt-2 font-display text-2xl font-semibold leading-none tracking-[-0.045em] sm:text-3xl"
                    style={previewScaleStyle}
                  >
                    {t("settings.appearance.previewTitle")}
                  </div>
                  <p className="mt-3 font-body text-sm leading-6 text-muted-foreground">
                    {t("settings.appearance.previewBody")}
                  </p>
                  <div className="mt-4 grid grid-cols-3 gap-2">
                    <PreviewMetric
                      value="128"
                      label={t("settings.appearance.previewMetricSkills")}
                    />
                    <PreviewMetric
                      value="14"
                      label={t("settings.appearance.previewMetricChecks")}
                    />
                    <PreviewMetric
                      value="32"
                      label={t("settings.appearance.previewMetricMs")}
                    />
                  </div>
                </div>
              </div>

              <div className="mt-3 grid gap-2 text-xs sm:grid-cols-3">
                <SpecimenStrip
                  label={t("settings.appearance.displayFont")}
                  value={displayLabel}
                  previewClassName={fontPreviewClassFor(prefs.display)}
                />
                <SpecimenStrip
                  label={t("settings.appearance.bodyFont")}
                  value={bodyLabel}
                  previewClassName={fontPreviewClassFor(prefs.body)}
                />
                <SpecimenStrip
                  label={t("settings.appearance.scale")}
                  value={scalePercent}
                  previewClassName="font-display tabular-nums"
                />
              </div>
            </div>
          </div>
        </section>

        <ControlGroup
          icon={<Palette className="size-4" />}
          title={t("settings.appearance.surfaceGroup")}
          description={t("settings.appearance.surfaceGroupDesc")}
        >
          <div className="grid gap-3 lg:grid-cols-[1.05fr_0.75fr_1.2fr]">
            <div className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80">
              <ControlLabel
                icon={<Palette className="size-3.5" />}
                label={t("settings.flavor")}
              />
              <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-2 2xl:grid-cols-3">
                {flavorOrder.map((item) => {
                  const active = flavor === item;
                  return (
                    <Button
                      key={item}
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-9 justify-start gap-2 rounded-lg px-2.5 text-xs active:scale-[0.96]",
                        active &&
                          "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]",
                      )}
                      onClick={() => onSetFlavor(item)}
                      aria-pressed={active}
                    >
                      <span
                        className="inline-block size-2.5 shrink-0 rounded-full ring-1 ring-background/70"
                        style={{ backgroundColor: flavorColors[item] }}
                      />
                      <span className="truncate">{t(`settings.${item}`)}</span>
                    </Button>
                  );
                })}
              </div>
            </div>

            <div className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80">
              <ControlLabel
                icon={<Globe2 className="size-3.5" />}
                label={t("settings.language")}
              />
              <div className="mt-3 grid gap-2">
                {(["zh", "en"] as const).map((lang) => {
                  const active = currentLanguage === lang;
                  return (
                    <Button
                      key={lang}
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-10 justify-between rounded-lg px-3 text-xs active:scale-[0.96]",
                        active &&
                          "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]",
                      )}
                      onClick={() => {
                        void i18n.changeLanguage(lang);
                      }}
                      aria-pressed={active}
                    >
                      <span>
                        {t(
                          lang === "zh"
                            ? "settings.chinese"
                            : "settings.english",
                        )}
                      </span>
                      {active ? (
                        <Check className="size-3.5" aria-hidden="true" />
                      ) : null}
                    </Button>
                  );
                })}
              </div>
            </div>

            <div className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80">
              <div className="flex items-center justify-between gap-3">
                <ControlLabel
                  icon={<Droplets className="size-3.5" />}
                  label={t("settings.accentColor")}
                />
                <span className="truncate rounded-full border border-border/80 bg-card px-2 py-1 text-[0.68rem] text-muted-foreground">
                  {t("settings.appearance.selected", { value: accentLabel })}
                </span>
              </div>
              <div
                className="mt-3 grid grid-cols-7 gap-1.5 sm:grid-cols-[repeat(14,minmax(0,1fr))] lg:grid-cols-7 2xl:grid-cols-[repeat(14,minmax(0,1fr))]"
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
                        "relative size-7 rounded-xl transition active:scale-[0.96] cursor-pointer",
                        isActive
                          ? "ring-2 ring-ring ring-offset-2 ring-offset-background scale-105 shadow-[0_0_20px_var(--settings-section-accent-soft)]"
                          : "ring-1 ring-border hover:scale-105 hover:ring-2 hover:ring-ring/50",
                      )}
                      style={{ backgroundColor: `var(${ctpVar})` }}
                    >
                      {isActive ? (
                        <span className="absolute inset-1 rounded-lg border border-background/60" />
                      ) : null}
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        </ControlGroup>

        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.35fr)_minmax(18rem,0.65fr)]">
          <ControlGroup
            icon={<ALargeSmall className="size-4" />}
            title={t("settings.appearance.typographyGroup")}
            description={t("settings.appearance.typographyGroupDesc")}
          >
            <div className="grid gap-4 2xl:grid-cols-2">
              <div
                className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80"
                role="group"
                aria-label={t("settings.appearance.displayFontOptionsLabel")}
              >
                <ControlLabel
                  icon={<Type className="size-3.5" />}
                  label={t("settings.appearance.displayFont")}
                />
                <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 2xl:grid-cols-2">
                  {DISPLAY_FONT_OPTIONS.map((option) => {
                    const label = t(
                      `settings.appearance.displayFontOption.${option.labelKey}`,
                    );
                    const active = prefs.display === option.key;
                    return (
                      <TypeSpecimenTile
                        key={option.key}
                        active={active}
                        title={label}
                        sample={option.sample}
                        detail={t("settings.appearance.displaySample")}
                        previewClassName={fontPreviewClassFor(option.key)}
                        onClick={() => void handleDisplayKey(option.key)}
                      />
                    );
                  })}
                </div>
                {prefs.display === "custom" && (
                  <Input
                    className="mt-3"
                    placeholder={t("settings.appearance.customPlaceholder")}
                    value={prefs.displayCustom}
                    onChange={(event) => {
                      void handleDisplayCustom(event.target.value);
                    }}
                    aria-label={t("settings.appearance.displayCustomLabel")}
                  />
                )}
              </div>

              <div
                className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80"
                role="group"
                aria-label={t("settings.appearance.bodyFontOptionsLabel")}
              >
                <ControlLabel
                  icon={<Type className="size-3.5" />}
                  label={t("settings.appearance.bodyFont")}
                />
                <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 2xl:grid-cols-2">
                  {BODY_FONT_OPTIONS.map((option) => {
                    const label = t(
                      `settings.appearance.bodyFontOption.${option.labelKey}`,
                    );
                    const active = prefs.body === option.key;
                    return (
                      <TypeSpecimenTile
                        key={option.key}
                        active={active}
                        title={label}
                        sample={option.key === "jetbrains" ? "01" : "Aa"}
                        detail={t("settings.appearance.bodySample")}
                        previewClassName={fontPreviewClassFor(option.key)}
                        onClick={() => void handleBodyKey(option.key)}
                      />
                    );
                  })}
                </div>
                {prefs.body === "custom" && (
                  <Input
                    className="mt-3"
                    placeholder={t("settings.appearance.customPlaceholder")}
                    value={prefs.bodyCustom}
                    onChange={(event) => {
                      void handleBodyCustom(event.target.value);
                    }}
                    aria-label={t("settings.appearance.bodyCustomLabel")}
                  />
                )}
              </div>
            </div>
          </ControlGroup>

          <ControlGroup
            icon={<SlidersHorizontal className="size-4" />}
            title={t("settings.appearance.densityGroup")}
            description={t("settings.appearance.densityGroupDesc")}
          >
            <div className="rounded-lg bg-background/70 p-3 ring-1 ring-border/80">
              <ControlLabel
                icon={<SlidersHorizontal className="size-3.5" />}
                label={t("settings.appearance.scale")}
              />
              <div
                className="mt-3 grid gap-2"
                role="group"
                aria-label={t("settings.appearance.scale")}
              >
                {FONT_SCALE_OPTIONS.map((option) => {
                  const active = Math.abs(prefs.scale - option.value) < 1e-3;
                  const label = t(`settings.appearance.${option.labelKey}`);
                  return (
                    <Button
                      key={option.value}
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-11 justify-between rounded-lg px-3 text-xs active:scale-[0.96]",
                        active &&
                          "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-foreground shadow-sm hover:bg-[color:var(--settings-section-accent-soft)]",
                      )}
                      onClick={() => void handleScale(option.value)}
                      aria-pressed={active}
                    >
                      <span>{label}</span>
                      <span className="font-display tabular-nums text-muted-foreground">
                        {Math.round(option.value * 100)}%
                      </span>
                    </Button>
                  );
                })}
              </div>
              <div className="mt-3 rounded-md border border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] p-3">
                <div className="text-[0.68rem] font-semibold uppercase tracking-[0.16em] text-[color:var(--settings-section-accent-text)]">
                  {t("settings.appearance.scaleSpecimen")}
                </div>
                <div className="mt-2 font-display text-3xl font-semibold tabular-nums tracking-[-0.06em]">
                  {scalePercent}
                </div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  {t("settings.appearance.scaleSample", {
                    scale: scalePercent,
                  })}
                </p>
              </div>
            </div>
          </ControlGroup>
        </div>
      </div>
    </SettingsCollapsibleCard>
  );
}

function ControlGroup({
  icon,
  title,
  description,
  children,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl bg-card/70 p-3 shadow-sm ring-1 ring-border/80 sm:p-4">
      <div className="mb-3 flex items-start gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-xl border border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-[color:var(--settings-section-accent-text)]">
          {icon}
        </span>
        <div className="min-w-0">
          <h4 className="text-sm font-semibold tracking-tight text-foreground">
            {title}
          </h4>
          <p className="mt-0.5 text-xs leading-5 text-muted-foreground">
            {description}
          </p>
        </div>
      </div>
      {children}
    </section>
  );
}

function ControlLabel({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
      {icon}
      <span>{label}</span>
    </div>
  );
}

function StatusChip({ label, value }: { label: string; value: string }) {
  return (
    <div
      className="min-w-0 rounded-lg bg-background/70 px-3 py-2 ring-1 ring-border/80"
      aria-label={`${label} ${value}`}
    >
      <div className="truncate text-[0.62rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
        {label}
      </div>
      <div className="mt-0.5 truncate text-sm font-semibold text-foreground">
        {value}
      </div>
    </div>
  );
}

function PreviewMetric({ value, label }: { value: string; label: string }) {
  return (
    <div className="rounded-sm border border-border/70 bg-background/60 px-2.5 py-2">
      <div className="font-display text-lg font-semibold leading-none tabular-nums tracking-[-0.05em]">
        {value}
      </div>
      <div className="mt-1 truncate text-[0.6rem] uppercase tracking-[0.14em] text-muted-foreground">
        {label}
      </div>
    </div>
  );
}

function SpecimenStrip({
  label,
  value,
  previewClassName,
}: {
  label: string;
  value: string;
  previewClassName: string;
}) {
  return (
    <div
      className="min-w-0 rounded-md border border-border/70 bg-card/70 px-3 py-2"
      aria-label={`${label} ${value}`}
    >
      <div className="truncate text-[0.6rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {label}
      </div>
      <div
        className={cn(
          "mt-1 truncate text-sm font-semibold text-foreground",
          previewClassName,
        )}
      >
        {value}
      </div>
    </div>
  );
}

function TypeSpecimenTile({
  active,
  title,
  sample,
  detail,
  previewClassName,
  onClick,
}: {
  active: boolean;
  title: string;
  sample: string;
  detail: string;
  previewClassName: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={cn(
        "min-h-24 rounded-md border border-border/80 bg-card/70 p-3 text-left transition active:scale-[0.96]",
        "hover:border-[color:var(--settings-section-accent-border)] hover:bg-[color:var(--settings-section-accent-faint)]",
        active &&
          "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] shadow-sm ring-1 ring-[color:var(--settings-section-accent-border)]",
      )}
      onClick={onClick}
      aria-pressed={active}
    >
      <div className="flex items-start justify-between gap-2">
        <span
          className={cn(
            "text-2xl font-semibold leading-none tracking-[-0.06em]",
            previewClassName,
          )}
        >
          {sample}
        </span>
        {active ? (
          <span className="grid size-5 shrink-0 place-items-center rounded-full bg-[color:var(--settings-section-accent)] text-primary-foreground">
            <Check className="size-3" aria-hidden="true" />
          </span>
        ) : null}
      </div>
      <div className="mt-3 truncate text-xs font-semibold text-foreground">
        {title}
      </div>
      <div className="mt-1 line-clamp-2 text-[0.68rem] leading-4 text-muted-foreground">
        {detail}
      </div>
    </button>
  );
}

function getCurrentLanguage(language: string): "zh" | "en" {
  return language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

function getDisplayOption(key: DisplayFontKey) {
  return (
    DISPLAY_FONT_OPTIONS.find((option) => option.key === key) ??
    DISPLAY_FONT_OPTIONS[0]
  );
}

function getBodyOption(key: BodyFontKey) {
  return (
    BODY_FONT_OPTIONS.find((option) => option.key === key) ??
    BODY_FONT_OPTIONS[0]
  );
}

function getScaleOption(value: number) {
  return (
    FONT_SCALE_OPTIONS.find(
      (option) => Math.abs(option.value - value) < 1e-3,
    ) ?? FONT_SCALE_OPTIONS[1]
  );
}

function fontPreviewClassFor(key: FontPreviewKey): string {
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

function fontPreferencesEqual(left: FontPreferences, right: FontPreferences) {
  return (
    left.display === right.display &&
    left.displayCustom === right.displayCustom &&
    left.body === right.body &&
    left.bodyCustom === right.bodyCustom &&
    Math.abs(left.scale - right.scale) < 1e-6
  );
}
