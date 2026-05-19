import type { TargetSummary } from "@/types";

export function isLocalTarget(target: Pick<TargetSummary, "kind"> | null | undefined) {
  return target?.kind === "local";
}

export function isSshTarget(target: Pick<TargetSummary, "kind"> | null | undefined) {
  return target?.kind === "ssh";
}

export function isWslTarget(target: Pick<TargetSummary, "kind"> | null | undefined) {
  return target?.kind === "wsl";
}

export function isRemoteLikeTarget(
  target: Pick<TargetSummary, "kind"> | null | undefined
) {
  return Boolean(target && target.kind !== "local");
}

export function requiresSshPasswordRepair(
  target:
    | Pick<TargetSummary, "kind" | "authMethod">
    | null
    | undefined
) {
  return target?.kind === "ssh" && target.authMethod === "password";
}
