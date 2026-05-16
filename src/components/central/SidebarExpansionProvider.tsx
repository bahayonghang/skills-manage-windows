import type { ReactNode } from "react";

import {
  SidebarExpansionContext,
  type SidebarExpansionSignal,
} from "@/components/central/sidebarExpansionContext";

export function SidebarExpansionProvider({
  signal,
  children,
}: {
  signal: SidebarExpansionSignal | null;
  children: ReactNode;
}) {
  return (
    <SidebarExpansionContext.Provider value={signal}>
      {children}
    </SidebarExpansionContext.Provider>
  );
}

