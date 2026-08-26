import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type {
  AgentWithStatus,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

export const SKILLS_CLI_RECENT_SOURCES_SETTING_KEY = "skills_cli.recent_sources";
export const SKILLS_CLI_RECENT_SOURCES_MAX = 8;
export const SKILLS_CLI_RECENT_SOURCES_MAX_ITEM = 2048;
export const SKILLS_CLI_RECENT_SOURCES_PARSE_ERROR =
  "setting_value_invalid:The setting value is invalid.";

export const INSTALL_STEPS = ["source", "skills", "platforms"] as const;
export type InstallStep = (typeof INSTALL_STEPS)[number];
export type InstallStepStatus = "current" | "completed" | "pending";

export interface InstallDialogSession {
  sessionId: number;
  step: InstallStep;
  sourceInput: string;
  resolvedPreview: SkillsCliSourcePreview | null;
  selectedSkillNames: ReadonlySet<string>;
  selectedPlatformIds: ReadonlySet<string>;
  submitError: string | null;
  pendingPreviewId: number | null;
}

export type InstallDialogAction =
  | { type: "opened" }
  | { type: "closed" }
  | { type: "sourceChanged"; value: string }
  | { type: "previewStarted"; source: string; requestId: number }
  | {
      type: "previewSucceeded";
      sessionId: number;
      requestId: number;
      result: SkillsCliSourcePreview;
      defaultSkillNames: readonly string[];
      defaultPlatformIds: readonly string[];
    }
  | {
      type: "previewFailed";
      sessionId: number;
      requestId: number;
      message: string;
    }
  | { type: "toggleSkill"; name: string }
  | { type: "selectAllSkills" }
  | { type: "clearSkills" }
  | { type: "setPlatformIds"; ids: readonly string[] }
  | { type: "continueToPlatforms" }
  | { type: "goBack" }
  | { type: "installStarted" }
  | { type: "installFailed"; message: string }
  | { type: "clearError" };

export type ParsedRecentSources =
  | { ok: true; sources: string[] }
  | { ok: false };

const SAFE_PREVIEW_TOKEN = /^[A-Za-z0-9_@./:+-]+$/;
const RECENT_SOURCE_FORBIDDEN = /[ &|^%!<>"']/;

export function createInitialInstallSession(
  sessionId = 0,
): InstallDialogSession {
  return {
    sessionId,
    step: "source",
    sourceInput: "",
    resolvedPreview: null,
    selectedSkillNames: new Set(),
    selectedPlatformIds: new Set(),
    submitError: null,
    pendingPreviewId: null,
  };
}

export function installDialogReducer(
  state: InstallDialogSession,
  action: InstallDialogAction,
): InstallDialogSession {
  switch (action.type) {
    case "opened":
      return createInitialInstallSession(state.sessionId + 1);
    case "closed":
      return {
        ...createInitialInstallSession(state.sessionId),
      };
    case "sourceChanged":
      return { ...state, sourceInput: action.value };
    case "previewStarted":
      return {
        ...state,
        step: "source",
        sourceInput: action.source,
        resolvedPreview: null,
        selectedSkillNames: new Set(),
        submitError: null,
        pendingPreviewId: action.requestId,
      };
    case "previewSucceeded":
      if (!isMatchingPreviewSettle(state, action.sessionId, action.requestId)) {
        return state;
      }
      return {
        ...state,
        step: "skills",
        sourceInput: action.result.source,
        resolvedPreview: action.result,
        selectedSkillNames: new Set(action.defaultSkillNames),
        selectedPlatformIds: new Set(action.defaultPlatformIds),
        submitError: null,
        pendingPreviewId: null,
      };
    case "previewFailed":
      if (!isMatchingPreviewSettle(state, action.sessionId, action.requestId)) {
        return state;
      }
      return {
        ...state,
        step: "source",
        resolvedPreview: null,
        selectedSkillNames: new Set(),
        submitError: action.message,
        pendingPreviewId: null,
      };
    case "toggleSkill": {
      if (!state.resolvedPreview?.skills.includes(action.name)) {
        return state;
      }
      const selectedSkillNames = new Set(state.selectedSkillNames);
      if (selectedSkillNames.has(action.name)) {
        selectedSkillNames.delete(action.name);
      } else {
        selectedSkillNames.add(action.name);
      }
      return { ...state, selectedSkillNames };
    }
    case "selectAllSkills":
      if (!state.resolvedPreview) {
        return state;
      }
      return {
        ...state,
        selectedSkillNames: new Set(state.resolvedPreview.skills),
      };
    case "clearSkills":
      return { ...state, selectedSkillNames: new Set() };
    case "setPlatformIds":
      return { ...state, selectedPlatformIds: new Set(action.ids) };
    case "continueToPlatforms":
      if (state.step !== "skills" || state.selectedSkillNames.size === 0) {
        return state;
      }
      return { ...state, step: "platforms", submitError: null };
    case "goBack":
      if (state.step === "platforms") {
        return { ...state, step: "skills", submitError: null };
      }
      if (state.step === "skills") {
        return {
          ...state,
          step: "source",
          resolvedPreview: null,
          selectedSkillNames: new Set(),
          submitError: null,
        };
      }
      return state;
    case "installStarted":
      return { ...state, submitError: null };
    case "installFailed":
      return { ...state, submitError: action.message };
    case "clearError":
      return { ...state, submitError: null };
    default: {
      const _exhaustive: never = action;
      return _exhaustive;
    }
  }
}

export function isMatchingPreviewSettle(
  state: InstallDialogSession,
  sessionId: number,
  requestId: number,
): boolean {
  return (
    state.sessionId === sessionId && state.pendingPreviewId === requestId
  );
}

export function installStepStatus(
  current: InstallStep,
  item: InstallStep,
): InstallStepStatus {
  const currentIndex = INSTALL_STEPS.indexOf(current);
  const itemIndex = INSTALL_STEPS.indexOf(item);
  if (itemIndex === currentIndex) {
    return "current";
  }
  if (itemIndex < currentIndex) {
    return "completed";
  }
  return "pending";
}

export function uniqueStable<T>(items: readonly T[]): T[] {
  const seen = new Set<T>();
  const output: T[] = [];
  for (const item of items) {
    if (seen.has(item)) {
      continue;
    }
    seen.add(item);
    output.push(item);
  }
  return output;
}

export function defaultUninstalledSkillNames(
  previewSkills: readonly string[],
  installedNames: ReadonlySet<string>,
): string[] {
  return previewSkills.filter((name) => !installedNames.has(name));
}

export function defaultSelectedPlatformIds(
  targets: readonly SkillsCliInstallTarget[],
): string[] {
  return targets
    .filter((target) => target.defaultSelected)
    .map((target) => target.id);
}

export function orderedSelectedSkills(
  previewSkills: readonly string[],
  selected: ReadonlySet<string>,
): string[] {
  return previewSkills.filter((name) => selected.has(name));
}

export function orderedSelectedTargets(
  targets: readonly SkillsCliInstallTarget[],
  selected: ReadonlySet<string>,
): SkillsCliInstallTarget[] {
  return targets.filter((target) => selected.has(target.id));
}

export function mapSelectedSkillportAgentIds(
  targets: readonly SkillsCliInstallTarget[],
  selectedPlatformIds: ReadonlySet<string>,
): string[] {
  return uniqueStable(
    orderedSelectedTargets(targets, selectedPlatformIds).map(
      (target) => target.id,
    ),
  );
}

export function mapSelectedCliAgents(
  targets: readonly SkillsCliInstallTarget[],
  selectedPlatformIds: ReadonlySet<string>,
): string[] {
  return uniqueStable(
    orderedSelectedTargets(targets, selectedPlatformIds).map(
      (target) => target.cliAgent,
    ),
  );
}

export function quoteInstallPreviewToken(token: string): string {
  if (token.length === 0) {
    return '""';
  }
  if (SAFE_PREVIEW_TOKEN.test(token)) {
    return token;
  }
  return `"${token.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

export function buildInstallCommandPreview(input: {
  npmSpec: string;
  source: string;
  skillNames: readonly string[];
  cliAgents: readonly string[];
}): string {
  const skillNames = uniqueStable(input.skillNames);
  const cliAgents = uniqueStable(input.cliAgents);
  if (
    input.npmSpec.trim() === "" ||
    input.source.trim() === "" ||
    skillNames.length === 0 ||
    cliAgents.length === 0
  ) {
    return "";
  }
  const tokens = ["npx", input.npmSpec, "add", input.source];
  for (const name of skillNames) {
    tokens.push("-s", name);
  }
  tokens.push("-g");
  for (const agent of cliAgents) {
    tokens.push("-a", agent);
  }
  tokens.push("-y");
  return tokens.map(quoteInstallPreviewToken).join(" ");
}

export function countAssociatedPlacements(
  skills: readonly SkillsCliGlobalSkill[],
  agentId: string,
): number {
  return skills.reduce((count, skill) => {
    const associated = skill.placements.some(
      (placement) =>
        placement.agentId === agentId &&
        (placement.state === "managed_link" ||
          placement.state === "direct_copy"),
    );
    return associated ? count + 1 : count;
  }, 0);
}

export function buildPlatformInstalledCounts(
  skills: readonly SkillsCliGlobalSkill[],
  targets: readonly SkillsCliInstallTarget[],
): Record<string, number> {
  return Object.fromEntries(
    targets.map((target) => [
      target.id,
      countAssociatedPlacements(skills, target.id),
    ]),
  );
}

export function cliTargetToPlatformTarget(
  target: SkillsCliInstallTarget,
): AgentWithStatus {
  return {
    id: target.id,
    display_name: target.displayName,
    category: "coding",
    global_skills_dir: "",
    icon_name: target.iconName ?? undefined,
    is_detected: true,
    is_builtin: true,
    is_enabled: target.isEnabled,
  };
}

export function joinInstallPlatformTargets(
  targets: readonly SkillsCliInstallTarget[],
  agents: readonly AgentWithStatus[],
): PlatformTarget[] {
  const byId = new Map(agents.map((agent) => [agent.id, agent]));
  return targets.map((target) => {
    const agent = byId.get(target.id);
    if (!agent) {
      return cliTargetToPlatformTarget(target);
    }
    return {
      ...agent,
      display_name: target.displayName,
      icon_name: target.iconName ?? agent.icon_name,
      is_enabled: target.isEnabled,
    };
  });
}

export function nextRecentSources(
  current: readonly string[],
  source: string,
  max = SKILLS_CLI_RECENT_SOURCES_MAX,
): string[] {
  const trimmed = source.trim();
  if (trimmed === "") {
    return [...current];
  }
  return [trimmed, ...current.filter((item) => item !== trimmed)].slice(0, max);
}

export function isPlausibleRecentSource(value: string): boolean {
  if (value.length === 0 || value.length > SKILLS_CLI_RECENT_SOURCES_MAX_ITEM) {
    return false;
  }
  if (value.trim() !== value) {
    return false;
  }
  for (const char of value) {
    const code = char.charCodeAt(0);
    if (code < 32 || code === 127) {
      return false;
    }
  }
  if (RECENT_SOURCE_FORBIDDEN.test(value)) {
    return false;
  }
  if (value.includes("://") && /\/\/[^/]*@/.test(value)) {
    return false;
  }
  if (value.includes("?") || value.includes("#")) {
    return false;
  }
  return true;
}

export function parseRecentSourcesSetting(
  raw: string | null,
): ParsedRecentSources {
  if (raw == null) {
    return { ok: true, sources: [] };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { ok: false };
  }
  if (!Array.isArray(parsed) || parsed.length > SKILLS_CLI_RECENT_SOURCES_MAX) {
    return { ok: false };
  }
  if (!parsed.every((item) => typeof item === "string")) {
    return { ok: false };
  }
  const sources = parsed as string[];
  if (new Set(sources).size !== sources.length) {
    return { ok: false };
  }
  if (!sources.every(isPlausibleRecentSource)) {
    return { ok: false };
  }
  return { ok: true, sources };
}
