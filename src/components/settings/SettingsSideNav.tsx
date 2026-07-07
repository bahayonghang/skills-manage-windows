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
        "shrink-0 border-border/70 bg-card/40",
        "lg:w-56 lg:border-r lg:bg-transparent xl:w-60 2xl:w-64",
      )}
    >
      <div className="sticky top-0 space-y-1 p-2 lg:top-4 lg:p-4">
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
                "group flex min-h-10 w-full items-center gap-3 rounded-xl border px-3 py-2 text-left text-sm transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]",
                active
                  ? "border-primary/35 bg-primary/10 text-foreground shadow-[inset_0_1px_0_color-mix(in_oklch,white_18%,transparent)]"
                  : "border-transparent text-muted-foreground hover:border-border hover:bg-muted/55 hover:text-foreground",
              )}
            >
              <span
                className={cn(
                  "grid size-8 shrink-0 place-items-center rounded-lg border transition-colors",
                  active
                    ? "border-primary/40 bg-primary/15 text-primary"
                    : "border-border/70 bg-background/60 text-muted-foreground group-hover:text-foreground",
                )}
                aria-hidden="true"
              >
                <Icon className="size-4" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">
                  {t(page.titleKey)}
                </span>
                <span className="hidden truncate text-xs text-muted-foreground 2xl:block">
                  {t(page.descriptionKey)}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
