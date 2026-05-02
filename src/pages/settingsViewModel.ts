import type { CatppuccinAccent, ThemeFlavor } from "@/stores/themeStore";
import { THEME_FLAVORS } from "@/stores/themeStore";
import { deriveHomeDir, joinPathForDisplay } from "@/lib/path";
import type { AgentWithStatus } from "@/types";
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
