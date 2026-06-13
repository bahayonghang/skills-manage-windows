import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import {
  getSettingsSectionTheme,
  getSettingsSectionThemeStyle,
} from "@/components/settings/settingsSectionTheme";
import { cn } from "@/lib/utils";

export interface TocEntry {
  /** anchor id 必须与 SettingsView 中的 <section id="..."> 完全一致 */
  id: string;
  labelKey: string;
}

interface SettingsTableOfContentsProps {
  entries: readonly TocEntry[];
}

export function SettingsTableOfContents({ entries }: SettingsTableOfContentsProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [activeId, setActiveId] = useState<string | null>(
    location.hash ? location.hash.slice(1) : entries[0]?.id ?? null,
  );

  /* hash → scrollIntoView：进入页面或外部跳转时定位目标 section */
  useEffect(() => {
    if (!location.hash) return;
    const id = location.hash.slice(1);
    setActiveId(id);
    const node = document.getElementById(id);
    if (node) {
      node.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, [location.hash]);

  /* IntersectionObserver：滚动时高亮当前 section */
  useEffect(() => {
    if (typeof window === "undefined" || typeof IntersectionObserver === "undefined") {
      return;
    }
    const observer = new IntersectionObserver(
      (records) => {
        const visible = records
          .filter((entry) => entry.isIntersecting)
          .sort((a, b) => b.intersectionRatio - a.intersectionRatio);
        if (visible[0]) {
          setActiveId(visible[0].target.id);
        }
      },
      {
        rootMargin: "-25% 0px -55% 0px",
        threshold: [0, 0.25, 0.5, 1],
      },
    );

    const nodes = entries
      .map((entry) => document.getElementById(entry.id))
      .filter((node): node is HTMLElement => node !== null);

    nodes.forEach((node) => observer.observe(node));
    return () => observer.disconnect();
  }, [entries]);

  function handleClick(id: string) {
    setActiveId(id);
    navigate(`${location.pathname}#${id}`, { replace: true });
    const node = document.getElementById(id);
    if (node) {
      node.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }

  return (
    <nav
      aria-label={t("settings.toc.ariaLabel")}
      className="sticky top-0 z-20 -mx-6 mb-4 border-b border-border bg-background/85 px-6 py-2 backdrop-blur"
    >
      <ul className="flex flex-wrap gap-1">
        {entries.map((entry) => {
          const active = activeId === entry.id;
          const theme = getSettingsSectionTheme(entry.id);
          return (
            <li key={entry.id}>
              <button
                type="button"
                onClick={() => handleClick(entry.id)}
                aria-current={active ? "true" : undefined}
                data-settings-section-anchor={entry.id}
                data-settings-section-tone={theme.tone}
                style={getSettingsSectionThemeStyle(entry.id)}
                className={cn(
                  "rounded-full border px-2.5 py-1 text-xs font-medium transition-colors",
                  active
                    ? "border-[color:var(--settings-section-accent-border)] bg-[color:var(--settings-section-accent-soft)] text-[color:var(--settings-section-accent-text)] shadow-[0_0_0_1px_var(--settings-section-accent-faint)]"
                    : "border-transparent text-muted-foreground hover:border-[color:var(--settings-section-accent-border)] hover:bg-[color:var(--settings-section-accent-faint)] hover:text-foreground",
                )}
              >
                {t(`settings.toc.entries.${entry.labelKey}`)}
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
