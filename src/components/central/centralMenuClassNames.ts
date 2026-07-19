import { cn } from "@/lib/utils";

/**
 * Shared menu className builders for the Central skills toolbar and the
 * `SkillImportLauncher`. Kept here so both `CentralSkillsShellMenus.tsx`
 * (component file) and `SkillImportLauncher.tsx` can import them without
 * tripping the `react-refresh/only-export-components` lint rule.
 */

export function menuPopupClassName(extra?: string): string {
  return cn(
    "min-w-[200px] rounded-xl bg-popover p-1 text-sm text-popover-foreground shadow-[0_0_0_1px_color-mix(in_srgb,var(--foreground)_10%,transparent),0_16px_40px_-18px_color-mix(in_srgb,var(--background)_85%,transparent)] outline-none",
    "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
    "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
    "animation-duration-100",
    extra,
  );
}

export function menuItemClassName(extra?: string | false): string {
  return cn(
    "flex min-h-8 cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none transition-[background-color,color] data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground",
    extra,
  );
}

export function menuLabelClassName(): string {
  return "px-2 pt-1 pb-0.5 text-xs uppercase tracking-wide text-muted-foreground";
}
