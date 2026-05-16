import { useContext } from "react";

import { SidebarExpansionContext } from "@/components/central/sidebarExpansionContext";

export function useSidebarExpansionSignal() {
  return useContext(SidebarExpansionContext);
}

