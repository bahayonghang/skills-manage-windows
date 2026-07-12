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
import { cn } from "@/lib/utils";

const SETTINGS_SECTION_COLLAPSED_STORAGE_KEY = "settings.sectionCollapsed.v1";

type CollapsedSectionMap = Record<string, boolean>;

interface SettingsSectionProps {
  sectionId: string;
  title: string;
  description?: string;
  icon?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}

function readCollapsedSections(): CollapsedSectionMap {
  if (typeof window === "undefined") return {};

  try {
    const raw = window.localStorage.getItem(SETTINGS_SECTION_COLLAPSED_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return {};
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
  if (typeof window === "undefined") return;

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

function SectionIcon({ icon }: { icon: ReactNode }) {
  if (!icon) return null;

  const renderedIcon = isValidElement<{ className?: string }>(icon)
    ? cloneElement(icon as ReactElement<{ className?: string }>, {
        className: cn(icon.props.className, "size-4"),
      })
    : icon;

  return (
    <span
      className="mt-0.5 flex size-5 shrink-0 items-center justify-center text-muted-foreground"
      data-settings-section-icon=""
      aria-hidden="true"
    >
      {renderedIcon}
    </span>
  );
}

export function SettingsSection({
  sectionId,
  title,
  description,
  icon,
  action,
  children,
}: SettingsSectionProps) {
  const { t } = useTranslation();
  const contentId = useMemo(() => `settings-section-${sectionId}-content`, [sectionId]);
  const [isCollapsed, setIsCollapsed] = useState(
    () => readCollapsedSections()[sectionId] === true
  );
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
    <section
      className="border-b border-border/70 pb-6 last:border-b-0 last:pb-0"
      data-settings-section={sectionId}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 items-start gap-2.5">
          <SectionIcon icon={icon} />
          <div className="min-w-0">
            <h3 className="text-balance font-heading text-base font-semibold leading-6">
              {title}
            </h3>
            {description ? (
              <p className="mt-0.5 max-w-3xl text-pretty text-sm leading-6 text-muted-foreground">
                {description}
              </p>
            ) : null}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {action}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-10 text-muted-foreground hover:text-foreground"
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
      {isCollapsed ? (
        <div id={contentId} hidden />
      ) : (
        <div id={contentId} className="mt-5">
          {children}
        </div>
      )}
    </section>
  );
}

export { SETTINGS_SECTION_COLLAPSED_STORAGE_KEY };
