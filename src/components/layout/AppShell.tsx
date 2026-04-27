import { useEffect, useRef, useState } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { GlobalSearchDialog } from "./GlobalSearchDialog";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useDiscoverStore } from "@/stores/discoverStore";
import { useTargetStore } from "@/stores/targetStore";
import { useSkillStore } from "@/stores/skillStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";

/**
 * Top-level app shell: TopBar + icon sidebar + scrollable main content area.
 * Triggers the initial platform scan on mount.
 */
export function AppShell() {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const mainRef = useRef<HTMLElement | null>(null);
  const { pathname } = useLocation();

  const initialize = usePlatformStore((s) => s.initialize);
  const rescan = usePlatformStore((s) => s.rescan);
  const resetPlatformForTargetChange = usePlatformStore((s) => s.resetForTargetChange);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);
  const resetCentralForTargetChange = useCentralSkillsStore((s) => s.resetForTargetChange);
  const rescanDiscoverFromDisk = useDiscoverStore((s) => s.rescanFromDisk);
  const resetDiscoverForTargetChange = useDiscoverStore((s) => s.resetForTargetChange);
  const resetSkillsForTargetChange = useSkillStore((s) => s.resetForTargetChange);
  const resetMarketplaceForTargetChange = useMarketplaceStore((s) => s.resetForTargetChange);
  const loadTargets = useTargetStore((s) => s.loadTargets);
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);
  const [hasLoadedTargets, setHasLoadedTargets] = useState(false);
  const lastTargetIdRef = useRef<string | null>(null);
  const isInitialTargetLoadRef = useRef(true);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      await loadTargets().catch(() => undefined);
      if (cancelled) return;
      setHasLoadedTargets(true);
      await initialize().catch(() => undefined);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    resetDiscoverForTargetChange();
    resetSkillsForTargetChange();
    resetMarketplaceForTargetChange();
    void handleGlobalRescan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTargetId, hasLoadedTargets]);

  useEffect(() => {
    if (!mainRef.current) return;
    mainRef.current.scrollTop = 0;
  }, [pathname]);

  async function handleGlobalRescan() {
    await rescan();
    await Promise.allSettled([
      loadCentralSkills(),
      rescanDiscoverFromDisk(),
    ]);
  }

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
    </div>
  );
}
