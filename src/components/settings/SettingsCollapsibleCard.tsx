import { ChevronDown, ChevronRight } from "lucide-react";
import {
  cloneElement,
  isValidElement,
  useEffect,
  useMemo,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  getSettingsSectionTheme,
  getSettingsSectionThemeStyle,
} from "@/components/settings/settingsSectionTheme";
import { cn } from "@/lib/utils";

const SETTINGS_SECTION_COLLAPSED_STORAGE_KEY = "settings.sectionCollapsed.v1";

type CollapsedSectionMap = Record<string, boolean>;

interface SettingsCollapsibleCardProps {
  sectionId: string;
  title: string;
  description?: string;
  icon?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}

function readCollapsedSections(): CollapsedSectionMap {
  if (typeof window === "undefined") {
    return {};
  }

  try {
    const raw = window.localStorage.getItem(SETTINGS_SECTION_COLLAPSED_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") {
      return {};
    }
    return Object.fromEntries(
      Object.entries(parsed).filter((entry): entry is [string, boolean] =>
        typeof entry[1] === "boolean"
      )
    );
  } catch {
    return {};
  }
}

function writeSectionCollapsed(sectionId: string, collapsed: boolean) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    const next = { ...readCollapsedSections(), [sectionId]: collapsed };
    window.localStorage.setItem(
      SETTINGS_SECTION_COLLAPSED_STORAGE_KEY,
      JSON.stringify(next)
    );
  } catch {
    // Local UI preference only; ignore unavailable localStorage.
  }
}

function renderSectionIcon(icon: ReactNode) {
  const renderedIcon = isValidElement<{ className?: string }>(icon)
    ? cloneElement(icon as ReactElement<{ className?: string }>, {
        className: cn(
          icon.props.className,
          "size-4 text-[color:var(--settings-section-accent-text)]"
        ),
      })
    : icon ?? (
        <span className="size-2 rounded-full bg-[color:var(--settings-section-accent)]" />
      );

  return (
    <span
      className="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-xl border border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-[color:var(--settings-section-accent-text)] shadow-sm"
      data-settings-section-icon=""
      aria-hidden="true"
    >
      {renderedIcon}
    </span>
  );
}

export function SettingsCollapsibleCard({
  sectionId,
  title,
  description,
  icon,
  action,
  children,
}: SettingsCollapsibleCardProps) {
  const { t } = useTranslation();
  const contentId = useMemo(() => `settings-section-${sectionId}-content`, [sectionId]);
  const [isCollapsed, setIsCollapsed] = useState(
    () => readCollapsedSections()[sectionId] === true
  );
  const theme = getSettingsSectionTheme(sectionId);
  const themeStyle = getSettingsSectionThemeStyle(sectionId);
  const ToggleIcon = isCollapsed ? ChevronRight : ChevronDown;

  useEffect(() => {
    setIsCollapsed(readCollapsedSections()[sectionId] === true);
  }, [sectionId]);

  function toggleCollapsed() {
    setIsCollapsed((current) => {
      const next = !current;
      writeSectionCollapsed(sectionId, next);
      return next;
    });
  }

  return (
    <Card
      className="relative ring-[color:var(--settings-section-accent-border)]"
      data-settings-section={sectionId}
      data-settings-section-tone={theme.tone}
      style={themeStyle}
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-16 bg-[linear-gradient(180deg,var(--settings-section-accent-faint),transparent)]"
      />
      <CardHeader className="relative z-10">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            {renderSectionIcon(icon)}
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <CardTitle className="text-[color:var(--settings-section-accent-text)]">
                  {title}
                </CardTitle>
                <span
                  aria-hidden="true"
                  className="h-1.5 w-1.5 rounded-full bg-[color:var(--settings-section-accent)] shadow-[0_0_0_3px_var(--settings-section-accent-soft)]"
                />
              </div>
              {description ? (
                <CardDescription className="mt-1">
                  {description}
                </CardDescription>
              ) : null}
              <span
                aria-hidden="true"
                className="mt-2 block h-1 w-10 rounded-full bg-[color:var(--settings-section-accent)] opacity-70"
              />
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {action}
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="px-2 text-[color:var(--settings-section-accent-text)] hover:bg-[color:var(--settings-section-accent-soft)]"
              aria-controls={contentId}
              aria-expanded={!isCollapsed}
              aria-label={t(
                isCollapsed ? "settings.expandSection" : "settings.collapseSection",
                { title }
              )}
              onClick={toggleCollapsed}
            >
              <ToggleIcon className="size-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      {isCollapsed ? (
        <div id={contentId} hidden />
      ) : (
        <CardContent id={contentId} className="relative z-10">
          {children}
        </CardContent>
      )}
    </Card>
  );
}

export { SETTINGS_SECTION_COLLAPSED_STORAGE_KEY };
