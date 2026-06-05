import { cn } from "@/lib/utils";

export type JsonViewMode = "raw" | "pretty";

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
