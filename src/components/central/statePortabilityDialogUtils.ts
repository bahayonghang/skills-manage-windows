import type { TFunction } from "i18next";

import type {
  SkillportStateExportedTarget,
  SkillportStateSkillPreview,
  TargetSummary,
} from "@/types";

export type JsonViewMode = "raw" | "pretty";
export interface ExportSummary {
  githubSources: number;
  centralSkills: number;
  unrestorableSkills: number;
}

export const EMPTY_SUMMARY: ExportSummary = {
  githubSources: 0,
  centralSkills: 0,
  unrestorableSkills: 0,
};

export function parseExportSummary(json: string): ExportSummary {
  const parsed = JSON.parse(json) as {
    githubSources?: unknown[];
    centralSkills?: unknown[];
    unrestorableSkills?: unknown[];
  };
  return {
    githubSources: parsed.githubSources?.length ?? 0,
    centralSkills: parsed.centralSkills?.length ?? 0,
    unrestorableSkills: parsed.unrestorableSkills?.length ?? 0,
  };
}

export function parseOriginTarget(
  json: string,
): SkillportStateExportedTarget | null {
  if (!json.trim()) return null;
  try {
    const parsed = JSON.parse(json) as {
      exportedFrom?: {
        target?: {
          id?: unknown;
          kind?: unknown;
          label?: unknown;
        } | null;
      };
    };
    const target = parsed.exportedFrom?.target;
    if (
      !target ||
      typeof target.id !== "string" ||
      typeof target.kind !== "string" ||
      typeof target.label !== "string"
    ) {
      return null;
    }
    return { id: target.id, kind: target.kind, label: target.label };
  } catch {
    return null;
  }
}

function targetKindLabel(kind: string, t: TFunction) {
  if (kind === "local") return t("targets.local");
  if (kind === "wsl") return t("targets.kind.wsl");
  if (kind === "ssh") return t("targets.kind.ssh");
  return kind;
}

export function targetDisplayLabel(
  target: Pick<TargetSummary, "kind" | "label"> | SkillportStateExportedTarget,
  t: TFunction,
) {
  if (target.kind === "local") return t("targets.local");
  return t("central.portabilityTargetBadge", {
    kind: targetKindLabel(target.kind, t),
    label: target.label,
  });
}

export function prettifyJson(json: string) {
  return JSON.stringify(JSON.parse(json), null, 2);
}

export function defaultExportFileName() {
  const date = new Date().toISOString().slice(0, 10);
  return `skillport-state-${date}.json`;
}

export function statusTone(status: SkillportStateSkillPreview["status"]) {
  if (status === "ready")
    return "border-success/40 bg-success/10 text-success-foreground";
  if (status === "conflict")
    return "border-warning/40 bg-warning/10 text-warning-foreground";
  if (status === "missing")
    return "border-destructive/40 bg-destructive/10 text-destructive";
  if (status === "duplicate_skipped")
    return "border-info/40 bg-info/10 text-info-foreground";
  return "border-muted-foreground/30 bg-muted text-muted-foreground";
}

export function isManifestPreviewError(error: unknown) {
  const message = String(error);
  return (
    message.includes("Invalid SkillPort state JSON:") ||
    message.includes("Unsupported SkillPort state export kind") ||
    message.includes("Unsupported SkillPort state export version:")
  );
}

export function conflictKey(
  skill: Pick<SkillportStateSkillPreview, "id" | "sourcePath">,
) {
  return `${skill.id}\u001f${skill.sourcePath ?? ""}`;
}
