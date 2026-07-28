import { cn } from "@/lib/utils";

export function cardShellClass(selected?: boolean): string {
  return cn(
    "central-skill-card-surface flex h-full flex-col rounded-xl bg-card",
    selected && "central-skill-card-selected bg-primary/[0.04]",
  );
}
