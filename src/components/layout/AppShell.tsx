import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Outlet, useLocation } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { GlobalSearchDialog } from "./GlobalSearchDialog";
import { UpdateCenterDialog } from "@/components/central/UpdateCenterDialog";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useTargetStore } from "@/stores/targetStore";
import { useSkillStore } from "@/stores/skillStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { isTauriRuntime, listen, showMainWindowWhenReady } from "@/lib/ipc";

type MigrationProgressPayload =
  | { phase: "started" }
  | { phase: "completed"; copied: number; skipped: number; failed: number }
  | { phase: "failed"; error: string };

function disposeListener(unlisten?: (() => void | Promise<void>) | null) {
  if (!unlisten) {
    return;
  }

  void Promise.resolve(unlisten()).catch(() => undefined);
}

/**
 * Top-level app shell: TopBar + icon sidebar + scrollable main content area.
 * Triggers the initial platform scan on mount.
 */
export function AppShell() {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const mainRef = useRef<HTMLElement | null>(null);
  const { pathname } = useLocation();
  const { t } = useTranslation();

  const initialize = usePlatformStore((s) => s.initialize);
  const rescan = usePlatformStore((s) => s.rescan);
  const resetPlatformForTargetChange = usePlatformStore((s) => s.resetForTargetChange);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);
  const resetCentralForTargetChange = useCentralSkillsStore((s) => s.resetForTargetChange);
  const resetSkillsForTargetChange = useSkillStore((s) => s.resetForTargetChange);
  const resetMarketplaceForTargetChange = useMarketplaceStore((s) => s.resetForTargetChange);
  const loadTargets = useTargetStore((s) => s.loadTargets);
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);
  const [hasLoadedTargets, setHasLoadedTargets] = useState(false);
  const lastTargetIdRef = useRef<string | null>(null);
  const isInitialTargetLoadRef = useRef(true);
  const startupActionsRef = useRef({ initialize, loadTargets });
  const handleGlobalRescan = useCallback(async () => {
    await rescan();
    await loadCentralSkills();
  }, [loadCentralSkills, rescan]);

  useEffect(() => {
    void showMainWindowWhenReady().catch(() => undefined);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const { initialize: initialInitialize, loadTargets: initialLoadTargets } =
        startupActionsRef.current;
      await initialLoadTargets().catch(() => undefined);
      if (cancelled) return;
      setHasLoadedTargets(true);
      await initialInitialize().catch(() => undefined);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!activeTargetId) return;

    if (isInitialTargetLoadRef.current) {
      lastTargetIdRef.current = activeTargetId;
      if (hasLoadedTargets) {
        isInitialTargetLoadRef.current = false;
      }
      return;
    }

    if (lastTargetIdRef.current === null) {
      lastTargetIdRef.current = activeTargetId;
      return;
    }

    if (lastTargetIdRef.current === activeTargetId) return;

    lastTargetIdRef.current = activeTargetId;
    resetPlatformForTargetChange();
    resetCentralForTargetChange();
    resetSkillsForTargetChange();
    resetMarketplaceForTargetChange();
    void handleGlobalRescan();
  }, [
    activeTargetId,
    handleGlobalRescan,
    hasLoadedTargets,
    resetCentralForTargetChange,
    resetMarketplaceForTargetChange,
    resetPlatformForTargetChange,
    resetSkillsForTargetChange,
  ]);

  useEffect(() => {
    if (!mainRef.current) return;
    mainRef.current.scrollTop = 0;
  }, [pathname]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void | Promise<void>) | undefined;

    void listen<MigrationProgressPayload>("system://migration-progress", (event) => {
      if (disposed) return;
      const payload = event.payload;
      switch (payload.phase) {
        case "completed":
          if (payload.copied > 0 || payload.failed > 0) {
            toast.success(
              t("central.migrationCompleted", {
                copied: payload.copied,
                skipped: payload.skipped,
                failed: payload.failed,
              })
            );
          }
          break;
        case "failed":
          toast.error(t("central.migrationFailed", { error: payload.error }));
          break;
        default:
          break;
      }
    }).then((cleanup) => {
      if (disposed) {
        disposeListener(cleanup);
        return;
      }
      unlisten = cleanup;
    }).catch(() => undefined);

    return () => {
      disposed = true;
      disposeListener(unlisten);
    };
  }, [t]);

  function handleAction(action: string) {
    switch (action) {
      case "rescan":
        void handleGlobalRescan();
        break;
    }
  }

  return (
    <div className="flex flex-col h-screen bg-background text-foreground overflow-hidden">
      <TopBar onSearchClick={() => setIsSearchOpen(true)} />
      <div className="flex flex-1 min-h-0">
        <Sidebar />
        <main ref={mainRef} className="flex-1 min-h-0 min-w-0 overflow-hidden">
          <Outlet />
        </main>
      </div>
      <GlobalSearchDialog
        open={isSearchOpen}
        onOpenChange={setIsSearchOpen}
        onAction={handleAction}
      />
      <UpdateCenterDialog />
    </div>
  );
}
