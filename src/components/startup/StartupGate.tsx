import { useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { StartupRecoveryView } from "./StartupRecoveryView";
import { showMainWindowWhenReady } from "@/lib/ipc";
import { useStartupStore } from "@/stores/startupStore";

function StartupLoadingView() {
  const { t } = useTranslation();

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-6 text-foreground">
      <div role="status" aria-live="polite" className="w-full max-w-lg">
        <div className="mb-5 flex items-center gap-2" aria-hidden="true">
          <span className="h-8 w-1.5 animate-pulse rounded-full bg-primary" />
          <span className="h-11 w-1.5 animate-pulse rounded-full bg-primary/70 [animation-delay:120ms]" />
          <span className="h-8 w-1.5 animate-pulse rounded-full bg-primary/40 [animation-delay:240ms]" />
        </div>
        <p className="text-sm font-medium text-foreground">{t("app.name")}</p>
        <p className="mt-1 text-sm text-muted-foreground">{t("startup.loading")}</p>
      </div>
    </main>
  );
}

export function StartupGate({ children }: { children: ReactNode }) {
  const status = useStartupStore((state) => state.status);
  const loadStatus = useStartupStore((state) => state.loadStatus);

  useEffect(() => {
    let cancelled = false;
    void loadStatus().finally(() => {
      if (!cancelled) void showMainWindowWhenReady().catch(() => undefined);
    });
    return () => {
      cancelled = true;
    };
  }, [loadStatus]);

  if (status?.phase === "ready") return children;
  if (status === null || status.phase === "checking") return <StartupLoadingView />;
  return <StartupRecoveryView status={status} />;
}
