import type { LogsListDensity } from "@/components/logs/LogsListRow";

export const LOG_DENSITY_STORAGE_KEY = "skillport.logs.density";

export function loadLogDensity(): LogsListDensity {
  if (typeof window === "undefined") return "comfortable";
  const stored = window.localStorage.getItem(LOG_DENSITY_STORAGE_KEY);
  return stored === "compact" ? "compact" : "comfortable";
}

export function downloadOperationLogs(payload: string) {
  const blob = new Blob([payload], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "");

  link.href = url;
  link.download = `skillport-operation-logs-${timestamp}.json`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

export function toDateInput(value?: string): string {
  return value?.slice(0, 10) ?? "";
}

export function fromStartDateInput(value: string): string | undefined {
  return value ? `${value}T00:00:00Z` : undefined;
}

export function fromEndDateInput(value: string): string | undefined {
  return value ? `${value}T23:59:59Z` : undefined;
}
