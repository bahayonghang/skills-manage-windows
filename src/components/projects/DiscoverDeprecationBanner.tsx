import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Info, X } from "lucide-react";

import { invoke, isTauriRuntime } from "@/lib/tauri";

const SETTING_KEY = "discover_deprecation_dismissed";

export function DiscoverDeprecationBanner() {
  const { t } = useTranslation();
  const [hidden, setHidden] = useState(true);

  useEffect(() => {
    let cancelled = false;
    if (!isTauriRuntime()) {
      setHidden(true);
      return;
    }
    void (async () => {
      try {
        const value = await invoke<string | null>("get_setting", {
          key: SETTING_KEY,
        });
        if (cancelled) return;
        setHidden(value === "true");
      } catch {
        if (!cancelled) setHidden(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (hidden) return null;

  const dismiss = async () => {
    setHidden(true);
    if (!isTauriRuntime()) return;
    try {
      await invoke("set_setting", { key: SETTING_KEY, value: "true" });
    } catch {
      // best-effort; the in-memory hide already happened
    }
  };

  return (
    <div className="flex items-start gap-2 border-b border-warning/40 bg-warning/10 px-6 py-2 text-sm text-warning-foreground">
      <Info className="mt-0.5 size-4 shrink-0" />
      <div className="flex-1">
        <p className="font-medium">{t("projects.discoverDeprecatedTitle")}</p>
        <p className="text-xs opacity-90">
          {t("projects.discoverDeprecatedBody")}
        </p>
      </div>
      <button
        type="button"
        onClick={() => void dismiss()}
        className="ml-2 rounded px-2 py-0.5 text-xs hover:bg-warning/20"
        aria-label={t("projects.discoverDeprecatedDismiss")}
      >
        {t("projects.discoverDeprecatedDismiss")}
        <X className="ml-1 inline size-3" />
      </button>
    </div>
  );
}
