import { useEffect, useState } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { GlobalSearchDialog } from "./GlobalSearchDialog";
import { usePlatformStore } from "@/stores/platformStore";
import { useDiscoverStore } from "@/stores/discoverStore";

/**
 * Top-level app shell: TopBar + icon sidebar + scrollable main content area.
 * Triggers the initial platform scan on mount.
 */
export function AppShell() {
  const [isSearchOpen, setIsSearchOpen] = useState(false);

  const initialize = usePlatformStore((s) => s.initialize);
  const startScan = useDiscoverStore((s) => s.startScan);

  useEffect(() => {
    void initialize().catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleAction(action: string) {
    switch (action) {
      case "rescan":
        startScan();
        break;
    }
  }

  return (
    <div className="flex flex-col h-screen bg-background text-foreground overflow-hidden">
      <TopBar onSearchClick={() => setIsSearchOpen(true)} />
      <div className="flex flex-1 min-h-0">
        <Sidebar />
        <main className="flex-1 overflow-auto min-w-0">
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
