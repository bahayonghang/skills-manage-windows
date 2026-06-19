import { Server } from "lucide-react";

import { cn } from "@/lib/utils";
import type { JsonViewMode } from "@/components/central/statePortabilityDialogUtils";

export function TargetBoundaryBadge({
  label,
  targetLabel,
}: {
  label: string;
  targetLabel: string;
}) {
  return (
    <div
      className="mt-2 inline-flex max-w-full items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-1.5 text-xs"
      data-testid="central-portability-active-target"
    >
      <Server className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span className="truncate font-medium text-foreground">
        {targetLabel}
      </span>
    </div>
  );
}

export function JsonViewToggle({
  value,
  onChange,
  prettyDisabled,
  rawLabel,
  prettyLabel,
}: {
  value: JsonViewMode;
  onChange: (value: JsonViewMode) => void;
  prettyDisabled: boolean;
  rawLabel: string;
  prettyLabel: string;
}) {
  return (
    <div className="inline-flex rounded-md border border-border bg-muted/40 p-1">
      <button
        type="button"
        data-testid="central-portability-raw-json"
        className={cn(
          "rounded-sm px-3 py-1.5 text-xs",
          value === "raw" ? "bg-background shadow-sm" : "text-muted-foreground",
        )}
        onClick={() => onChange("raw")}
      >
        {rawLabel}
      </button>
      <button
        type="button"
        data-testid="central-portability-pretty-json"
        className={cn(
          "rounded-sm px-3 py-1.5 text-xs",
          value === "pretty"
            ? "bg-background shadow-sm"
            : "text-muted-foreground",
        )}
        onClick={() => onChange("pretty")}
        disabled={prettyDisabled}
      >
        {prettyLabel}
      </button>
    </div>
  );
}

export function SummaryTile({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-border bg-muted/30 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-2xl font-semibold">{value}</div>
    </div>
  );
}
