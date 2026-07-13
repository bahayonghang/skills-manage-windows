import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import {
  SETTINGS_PAGES,
  getSettingsPagePath,
  type SettingsPageId,
} from "@/components/settings/settingsPages";
import { cn } from "@/lib/utils";

interface SettingsSideNavProps {
  activePageId: SettingsPageId;
}

export function SettingsSideNav({ activePageId }: SettingsSideNavProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <nav
      aria-label={t("settings.pages.navAriaLabel")}
      className={cn(
        "shrink-0 border-b border-border/70 bg-background/95",
        "lg:w-52 lg:border-r lg:border-b-0 xl:w-56",
      )}
    >
      <div className="scrollbar-subtle flex gap-1 overflow-x-auto p-2 lg:sticky lg:top-0 lg:block lg:space-y-1 lg:overflow-visible lg:p-3">
        {SETTINGS_PAGES.map((page) => {
          const active = page.id === activePageId;
          const Icon = page.icon;

          return (
            <button
              key={page.id}
              type="button"
              aria-current={active ? "page" : undefined}
              data-settings-page-nav={page.id}
              onClick={() => navigate(getSettingsPagePath(page.id))}
              className={cn(
                "group flex min-h-10 shrink-0 items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-[scale,background-color,color] active:scale-[0.96] lg:w-full",
                active
                  ? "bg-primary/12 text-foreground"
                  : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
              )}
            >
              <Icon
                className={cn(
                  "size-4 shrink-0 transition-colors",
                  active ? "text-primary" : "group-hover:text-foreground",
                )}
                aria-hidden="true"
              />
              <span className="truncate font-medium">{t(page.titleKey)}</span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
