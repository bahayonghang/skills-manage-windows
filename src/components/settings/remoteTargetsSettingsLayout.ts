import { isWslTarget, requiresSshPasswordRepair } from "@/lib/targetKind";
import type { TargetSummary } from "@/types";

export const TARGET_ROW_ACTION_CLASS = "min-w-fit flex-1 sm:flex-none";
export const TARGET_FORM_GRID_CLASS =
  "grid gap-3 md:grid-cols-2 xl:grid-cols-6";
export const TARGET_EDIT_GRID_CLASS =
  "grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-3 md:grid-cols-2 xl:grid-cols-6";
export const WSL_FORM_GRID_CLASS = "grid gap-3 md:grid-cols-2 xl:grid-cols-4";

type Translate = (key: string, options?: Record<string, unknown>) => string;

export function sshCredentialStatus(target: TargetSummary) {
  if (!requiresSshPasswordRepair(target)) return null;
  return (
    target.credentialStatus ?? (target.hasStoredPassword ? "stored" : "missing")
  );
}

export function sshCredentialStatusClass(status: string | null) {
  switch (status) {
    case "stored":
      return "border-primary/30 bg-primary/10 text-primary";
    case "session":
      return "border-warning/40 bg-warning/10 text-warning-foreground";
    case "unreadable":
      return "border-destructive/40 bg-destructive/10 text-destructive";
    default:
      return "border-muted-foreground/30 bg-muted text-muted-foreground";
  }
}

export function targetTitle(target: TargetSummary, t: Translate) {
  if (target.kind === "local") return t("targets.local");
  return target.label;
}

export function targetKindLabel(target: TargetSummary, t: Translate) {
  if (target.kind === "local") return t("targets.local");
  if (target.kind === "wsl") return t("targets.kind.wsl");
  return t("targets.kind.ssh");
}

export function targetDescription(target: TargetSummary, t: Translate) {
  if (target.kind === "local") return t("targets.localDescription");

  if (isWslTarget(target)) {
    return [
      t("targets.wslDistributionValue", {
        distribution: target.distribution || t("common.unknown"),
      }),
      target.remoteOs || t("common.unknown"),
      target.remoteHome || "",
    ]
      .filter(Boolean)
      .join(" • ");
  }

  const auth =
    target.authMethod === "password"
      ? t("targets.authPassword")
      : t("targets.authKey");
  const host = `${target.username ?? ""}@${target.host ?? ""}:${target.port ?? 22}`;
  return [
    auth,
    host,
    target.remoteOs || t("common.unknown"),
    target.remoteHome || "",
  ]
    .filter(Boolean)
    .join(" • ");
}
