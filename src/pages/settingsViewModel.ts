import type { TFunction } from "i18next";

import type { CatppuccinAccent, ThemeFlavor } from "@/stores/themeStore";
import { THEME_FLAVORS } from "@/stores/themeStore";
import type { AiSettings } from "@/stores/settingsStore";
import { deriveHomeDir, joinPathForDisplay } from "@/lib/path";
import {
  getPlatformCategoryKey,
  sortPlatformVisibilityAgents,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";
import {
  createPlatformTargetGroups,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import { matchesPlatformVisibilityQuery } from "@/components/settings/platformVisibilityUtils";
import type { AiProvider } from "@/data/aiProviders";
import type { AgentWithStatus, TargetSummary } from "@/types";
import type { SshTargetFormState } from "@/components/settings/RemoteTargetsSettingsSection";

const DB_PATH_FALLBACK = "~/.skillsmanage/db.sqlite";

export const REPO_URL = "https://github.com/bahayonghang/skills-manage-windows";

export const EMPTY_SSH_TARGET_FORM: SshTargetFormState = {
  label: "",
  host: "",
  username: "",
  port: "22",
  authMethod: "key",
  keyPath: "",
  password: "",
};

export const FLAVOR_COLORS: Record<ThemeFlavor, string> = {
  mocha: "#b4befe",
  macchiato: "#b7bdf8",
  frappe: "#babbf1",
  latte: "#7287fd",
  "claude-light": "#cc785c",
  "claude-dark": "#cc785c",
};

export const CTP_VAR_MAP: Record<CatppuccinAccent, string> = {
  rosewater: "--ctp-rosewater",
  flamingo: "--ctp-flamingo",
  pink: "--ctp-pink",
  mauve: "--ctp-mauve",
  red: "--ctp-red",
  maroon: "--ctp-maroon",
  peach: "--ctp-peach",
  yellow: "--ctp-yellow",
  green: "--ctp-green",
  teal: "--ctp-teal",
  sky: "--ctp-sky",
  sapphire: "--ctp-sapphire",
  blue: "--ctp-blue",
  lavender: "--ctp-lavender",
};

export const FLAVOR_ORDER: ThemeFlavor[] = THEME_FLAVORS;

export interface PlatformVisibilityGroupViewModel {
  agents: PlatformTarget[];
  category: "coding" | "lobster";
  description: string;
  enabledCount: number;
  groupVisible: boolean;
  title: string;
  totalCount: number;
}

export function resolveSettingsDbPath(
  agents: AgentWithStatus[],
  scanDirectories: Array<{ path: string }>
) {
  const candidates = [
    agents.find((agent) => agent.id === "central")?.global_skills_dir,
    ...scanDirectories.map((dir) => dir.path),
    ...agents.map((agent) => agent.global_skills_dir),
  ].filter((candidate): candidate is string => Boolean(candidate));

  const homeDir = candidates
    .map((candidate) => deriveHomeDir(candidate))
    .find((candidate): candidate is string => Boolean(candidate));

  return homeDir
    ? joinPathForDisplay(homeDir, ".skillsmanage/db.sqlite")
    : DB_PATH_FALLBACK;
}

export function getCustomAgents(agents: AgentWithStatus[]) {
  return agents.filter((agent) => !agent.is_builtin);
}

export function getNormalizedPlatformVisibilityQuery(query: string) {
  return query.trim().toLowerCase();
}

export function isPlatformVisibilitySearchActive(normalizedQuery: string) {
  return normalizedQuery.length > 0;
}

export function getPlatformVisibilityGroups({
  agents,
  categoryVisibility,
  normalizedQuery,
  t,
}: {
  agents: AgentWithStatus[];
  categoryVisibility: PlatformCategoryVisibility;
  normalizedQuery: string;
  t: TFunction;
}): PlatformVisibilityGroupViewModel[] {
  const allPlatformAgents = agents.filter((agent) => agent.id !== "central");
  const searchActive = isPlatformVisibilitySearchActive(normalizedQuery);
  const groupConfigs = [
    {
      category: "coding" as const,
      title: t("sidebar.categoryCoding"),
      description: t("settings.platformGroupCodingDesc"),
      groupVisible: categoryVisibility.coding,
    },
    {
      category: "lobster" as const,
      title: t("sidebar.categoryLobster"),
      description: t("settings.platformGroupLobsterDesc"),
      groupVisible: categoryVisibility.lobster,
    },
  ];

  return groupConfigs
    .map((group) => {
      const groupAgents = sortPlatformVisibilityAgents(
        allPlatformAgents.filter(
          (agent) => getPlatformCategoryKey(agent.category) === group.category
        )
      );
      const groupedAgents = createPlatformTargetGroups(groupAgents, agents);
      const matchingAgents = groupedAgents.filter((agent) =>
        matchesPlatformVisibilityQuery(agent, normalizedQuery)
      );

      return {
        ...group,
        enabledCount: groupAgents.filter((agent) => agent.is_enabled).length,
        totalCount: groupAgents.length,
        agents: matchingAgents,
      };
    })
    .filter((group) => !searchActive || group.agents.length > 0);
}

export function getAiProviderViewModel(
  aiSettings: AiSettings,
  providers: AiProvider[]
) {
  const currentProvider = providers.find(
    (provider) => provider.id === aiSettings.provider
  );
  const resolvedUrl =
    aiSettings.provider === "custom"
      ? aiSettings.customUrl
      : currentProvider?.endpoints[aiSettings.region] ?? "";

  return {
    aiProvider: aiSettings.provider,
    aiRegion: aiSettings.region,
    aiModel: aiSettings.model,
    aiCustomUrl: aiSettings.customUrl,
    currentProvider,
    resolvedUrl,
  };
}

export function getNextAiProviderPatch(
  id: string,
  providers: AiProvider[],
  aiSettings: AiSettings
): Partial<AiSettings> {
  const provider = providers.find((item) => item.id === id);
  return {
    provider: id,
    model: provider?.defaultModel ?? aiSettings.model,
    region:
      provider && !provider.regions.includes(aiSettings.region)
        ? provider.regions[0]
        : aiSettings.region,
  };
}

export function targetToSshTargetForm(target: TargetSummary): SshTargetFormState {
  return {
    label: target.label,
    host: target.host ?? "",
    username: target.username ?? "",
    port: String(target.port ?? 22),
    authMethod: target.authMethod ?? "key",
    keyPath: target.keyPath ?? "",
    password: "",
  };
}

export function sshTargetPayload(
  form: SshTargetFormState,
  includeEmptyPassword: boolean
) {
  const port = Number(form.port.trim() || "22");
  const authMethod = form.authMethod;
  const password = form.password.trim();

  return {
    label: form.label.trim(),
    host: form.host.trim(),
    username: form.username.trim(),
    port: Number.isFinite(port) ? port : 22,
    authMethod,
    keyPath: authMethod === "key" ? form.keyPath.trim() : null,
    password:
      authMethod === "password" && (includeEmptyPassword || password)
        ? form.password
        : null,
  };
}
