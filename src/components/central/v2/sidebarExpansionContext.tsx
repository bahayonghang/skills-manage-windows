import { createContext } from "react";

export interface SidebarExpansionSignal {
  expanded: boolean;
  token: number;
}

export const SidebarExpansionContext = createContext<SidebarExpansionSignal | null>(null);
