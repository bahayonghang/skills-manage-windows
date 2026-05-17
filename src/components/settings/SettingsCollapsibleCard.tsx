import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

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
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-2">
            {icon}
            <div className="min-w-0">
              <CardTitle>{title}</CardTitle>
              {description ? (
                <CardDescription className="mt-1">
                  {description}
                </CardDescription>
              ) : null}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {action}
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="px-2"
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
        <CardContent id={contentId}>{children}</CardContent>
      )}
    </Card>
  );
}

export { SETTINGS_SECTION_COLLAPSED_STORAGE_KEY };
