import { SKILLS_CLI_DRAWER_BAND_MIN_PX } from "@/pages/skillsCliViewModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
} from "@/types";

export const SKILLS_CLI_DRAWER_FIXED_WIDTH_PX = 460;

export type SkillsCliDocState =
  | { status: "idle" }
  | { status: "loading"; skillName: string; requestId: string }
  | { status: "ready"; skillName: string; content: string; byteSize: number }
  | { status: "empty"; skillName: string; byteSize: 0 }
  | { status: "error"; skillName: string; errorCode: string };

export type SkillsCliDetailRowAction = "link" | "unlink" | null;
export type SkillsCliDetailReasonKind =
  | "direct_copy"
  | "conflict"
  | "unavailable"
  | null;
export type SkillsCliDetailAggregate = "linkAll" | "unlinkAll" | "disabled";
export type SkillsCliDrawerWidthMode = "fullWidth" | "fixed460";

export interface SkillsCliDetailPlacementRow {
  agentId: string;
  displayName: string;
  targetPath: string;
  state: SkillsCliPlacementState;
  associated: boolean;
  switchChecked: boolean;
  switchDisabled: boolean;
  action: SkillsCliDetailRowAction;
  reasonCode: string | null;
  reasonKind: SkillsCliDetailReasonKind;
}

export function isAssociatedPlacement(state: SkillsCliPlacementState): boolean {
  switch (state) {
    case "managed_link":
    case "direct_copy":
      return true;
    case "missing":
    case "conflict":
    case "unavailable":
      return false;
    default: {
      const _exhaustive: never = state;
      return _exhaustive;
    }
  }
}

function toDetailRow(placement: SkillsCliPlacement): SkillsCliDetailPlacementRow {
  const base = {
    agentId: placement.agentId,
    displayName: placement.displayName,
    targetPath: placement.targetPath,
    state: placement.state,
    reasonCode: placement.reasonCode,
  };
  switch (placement.state) {
    case "managed_link":
      return {
        ...base,
        associated: true,
        switchChecked: true,
        switchDisabled: false,
        action: "unlink",
        reasonKind: null,
      };
    case "missing":
      return {
        ...base,
        associated: false,
        switchChecked: false,
        switchDisabled: false,
        action: "link",
        reasonKind: null,
      };
    case "direct_copy":
      return {
        ...base,
        associated: true,
        switchChecked: true,
        switchDisabled: true,
        action: null,
        reasonKind: "direct_copy",
      };
    case "conflict":
      return {
        ...base,
        associated: false,
        switchChecked: false,
        switchDisabled: true,
        action: null,
        reasonKind: "conflict",
      };
    case "unavailable":
      return {
        ...base,
        associated: false,
        switchChecked: false,
        switchDisabled: true,
        action: null,
        reasonKind: "unavailable",
      };
    default: {
      const _exhaustive: never = placement.state;
      return _exhaustive;
    }
  }
}

export function buildSkillsCliDetailRows(
  skill: SkillsCliGlobalSkill,
  targets: readonly SkillsCliInstallTarget[],
): SkillsCliDetailPlacementRow[] {
  const byAgent = new Map(
    skill.placements.map((placement) => [placement.agentId, placement]),
  );
  const rows: SkillsCliDetailPlacementRow[] = [];
  for (const target of targets) {
    const placement = byAgent.get(target.id);
    if (!placement) {
      continue;
    }
    rows.push(toDetailRow(placement));
  }
  return rows;
}

export function summarizeDetailPlacements(
  rows: readonly SkillsCliDetailPlacementRow[],
  targets: readonly SkillsCliInstallTarget[],
): {
  associatedCount: number;
  enabledCount: number;
  missingAgentIds: string[];
  managedLinkAgentIds: string[];
  aggregate: SkillsCliDetailAggregate;
} {
  const associatedCount = rows.filter((row) => row.associated).length;
  const enabledCount = targets.filter((target) => target.isEnabled).length;
  const missingAgentIds = rows
    .filter((row) => row.state === "missing")
    .map((row) => row.agentId);
  const managedLinkAgentIds = rows
    .filter((row) => row.state === "managed_link")
    .map((row) => row.agentId);
  const aggregate: SkillsCliDetailAggregate =
    missingAgentIds.length > 0
      ? "linkAll"
      : managedLinkAgentIds.length > 0
        ? "unlinkAll"
        : "disabled";
  return {
    associatedCount,
    enabledCount,
    missingAgentIds,
    managedLinkAgentIds,
    aggregate,
  };
}

export function skillsCliDrawerPanelWidth(contentWidthPx: number | null): {
  mode: SkillsCliDrawerWidthMode;
  cssWidth: string;
} {
  if (contentWidthPx == null || contentWidthPx <= 0) {
    return { mode: "fullWidth", cssWidth: "100%" };
  }
  if (contentWidthPx < SKILLS_CLI_DRAWER_BAND_MIN_PX) {
    return { mode: "fullWidth", cssWidth: `${contentWidthPx}px` };
  }
  return {
    mode: "fixed460",
    cssWidth: `${SKILLS_CLI_DRAWER_FIXED_WIDTH_PX}px`,
  };
}

export function folderHashPrefix(hash: string | null | undefined): string | null {
  const trimmed = hash?.trim() ?? "";
  if (!trimmed) {
    return null;
  }
  return trimmed.slice(0, 7);
}

export function skillLocalTimestamp(
  skill: Pick<SkillsCliGlobalSkill, "updatedAt" | "installedAt">,
): string | null {
  const value = skill.updatedAt?.trim() || skill.installedAt?.trim() || "";
  return value || null;
}

export function detailFocusForEntry(
  kind: "detail" | "manageLinks",
): "links" | null {
  return kind === "manageLinks" ? "links" : null;
}

export function consumeDetailFocus(_focus: "links" | null): null {
  return null;
}

export type SkillsCliDocResponse =
  | { ok: true; content: string; byteSize: number }
  | { ok: false; errorCode: string };

export function applySkillDocResponse(
  current: SkillsCliDocState,
  requestId: string,
  skillName: string,
  result: SkillsCliDocResponse,
): SkillsCliDocState {
  if (
    current.status !== "loading" ||
    current.requestId !== requestId ||
    current.skillName !== skillName
  ) {
    return current;
  }
  if (!result.ok) {
    return { status: "error", skillName, errorCode: result.errorCode };
  }
  if (result.byteSize === 0) {
    return { status: "empty", skillName, byteSize: 0 };
  }
  return {
    status: "ready",
    skillName,
    content: result.content,
    byteSize: result.byteSize,
  };
}

export function visibleSkillDocState(
  skillName: string | null | undefined,
  docState: SkillsCliDocState,
): SkillsCliDocState {
  if (!skillName) {
    return { status: "idle" };
  }
  switch (docState.status) {
    case "idle":
      return { status: "loading", skillName, requestId: "pending" };
    case "loading":
    case "ready":
    case "empty":
    case "error":
      if (docState.skillName !== skillName) {
        return { status: "loading", skillName, requestId: "pending" };
      }
      return docState;
    default: {
      const _exhaustive: never = docState;
      return _exhaustive;
    }
  }
}
