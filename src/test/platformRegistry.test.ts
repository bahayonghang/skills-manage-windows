import { describe, expect, it } from "vitest";

import {
  DEFAULT_ENABLED_PLATFORM_IDS,
  UNIVERSAL_AGENT_ID_ORDER,
  UNIVERSAL_INSTALL_AGENT_ORDER,
  UNIVERSAL_PLATFORM_REGISTRY,
  UNIVERSAL_PROJECT_AGENT_ID_ORDER,
} from "../lib/platformRegistry";

// 迁移前的三份字面量列表（行为基准）：id 与顺序均不能变。
const LEGACY_UNIVERSAL_AGENT_ID_ORDER = [
  "amp",
  "cline",
  "codex",
  "cursor",
  "deep-agents",
  "firebender",
  "copilot",
  "kimi-code-cli",
  "opencode",
  "warp",
];

const LEGACY_UNIVERSAL_PROJECT_AGENT_ID_ORDER = [
  "amp",
  "antigravity",
  "antigravity-cli",
  "cline",
  "codex",
  "cursor",
  "deep-agents",
  "firebender",
  "gemini-cli",
  "copilot",
  "kimi-code-cli",
  "opencode",
  "warp",
];

const LEGACY_UNIVERSAL_INSTALL_AGENT_ORDER = [
  "codex",
  "opencode",
  "antigravity-cli",
  "antigravity",
  "gemini-cli",
  "cursor",
  "amp",
];

const LEGACY_DEFAULT_ENABLED_PLATFORM_IDS = [
  "claude-code",
  "codex",
  "grok",
  "antigravity",
  "antigravity-cli",
  "opencode",
  "kiro",
];

describe("platformRegistry", () => {
  it("derives the global Universal order matching the legacy literal", () => {
    expect([...UNIVERSAL_AGENT_ID_ORDER]).toEqual(
      LEGACY_UNIVERSAL_AGENT_ID_ORDER,
    );
  });

  it("derives the project Universal order matching the legacy literal", () => {
    expect([...UNIVERSAL_PROJECT_AGENT_ID_ORDER]).toEqual(
      LEGACY_UNIVERSAL_PROJECT_AGENT_ID_ORDER,
    );
  });

  it("derives the install preference order matching the legacy literal", () => {
    expect([...UNIVERSAL_INSTALL_AGENT_ORDER]).toEqual(
      LEGACY_UNIVERSAL_INSTALL_AGENT_ORDER,
    );
  });

  it("keeps the default enabled platform ids matching the legacy literal", () => {
    expect([...DEFAULT_ENABLED_PLATFORM_IDS]).toEqual(
      LEGACY_DEFAULT_ENABLED_PLATFORM_IDS,
    );
  });

  it("has no duplicate registry ids", () => {
    const ids = UNIVERSAL_PLATFORM_REGISTRY.map((entry) => entry.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("has no duplicate installPreference values", () => {
    const preferences = UNIVERSAL_PLATFORM_REGISTRY.flatMap((entry) =>
      entry.installPreference === undefined ? [] : [entry.installPreference],
    );
    expect(new Set(preferences).size).toBe(preferences.length);
  });

  it("keeps install candidates a subset of the registry full set", () => {
    const allIds = new Set<string>(UNIVERSAL_PROJECT_AGENT_ID_ORDER);
    for (const id of UNIVERSAL_INSTALL_AGENT_ORDER) {
      expect(allIds.has(id)).toBe(true);
    }
  });

  it("marks exactly 10 registry entries as globalGroup", () => {
    expect(
      UNIVERSAL_PLATFORM_REGISTRY.filter((entry) => entry.globalGroup).length,
    ).toBe(10);
  });
});
