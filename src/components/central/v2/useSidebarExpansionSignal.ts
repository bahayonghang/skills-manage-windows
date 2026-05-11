import { useContext } from "react";

import { SidebarExpansionContext } from "@/components/central/v2/sidebarExpansionContext";

export function useSidebarExpansionSignal() {
  return useContext(SidebarExpansionContext);
}

