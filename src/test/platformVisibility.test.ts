import { describe, expect, it } from "vitest";

import type { AgentWithStatus } from "../types";
import {
  derivePlatformCategoryVisibility,
  ensureAtLeastOnePlatformCategoryVisible,
  filterVisiblePlatformAgents,
  resolvePlatformCategoryVisibility,
  sortPlatformVisibilityAgents,
} from "../lib/platformVisibility";

const baseAgent = {
  global_skills_dir: "~/.skills/",
  is_detected: true,
  is_builtin: true,
} satisfies Pick<
  AgentWithStatus,
  "global_skills_dir" | "is_detected" | "is_builtin"
>;

describe("platformVisibility helpers", () => {
  it("derives category visibility from enabled agents when no setting exists", () => {
    const agents: AgentWithStatus[] = [
      {
        ...baseAgent,
        id: "claude-code",
        display_name: "Claude Code",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "openclaw",
        display_name: "OpenClaw",
        category: "lobster",
        is_enabled: false,
      },
      {
        ...baseAgent,
        id: "central",
        display_name: "Central Skills",
        category: "central",
        is_enabled: true,
      },
    ];

    expect(derivePlatformCategoryVisibility(agents)).toEqual({
      coding: true,
      lobster: false,
    });
  });

  it("prefers saved category visibility over derived defaults", () => {
    const agents: AgentWithStatus[] = [
      {
        ...baseAgent,
        id: "claude-code",
        display_name: "Claude Code",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "openclaw",
        display_name: "OpenClaw",
        category: "lobster",
        is_enabled: false,
      },
    ];

    expect(
      resolvePlatformCategoryVisibility(
        JSON.stringify({ coding: false, lobster: true }),
        agents
      )
    ).toEqual({
      coding: false,
      lobster: true,
    });
  });

  it("falls back when saved category visibility would hide every platform group", () => {
    const agents: AgentWithStatus[] = [
      {
        ...baseAgent,
        id: "claude-code",
        display_name: "Claude Code",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "openclaw",
        display_name: "OpenClaw",
        category: "lobster",
        is_enabled: false,
      },
    ];

    expect(
      resolvePlatformCategoryVisibility(
        JSON.stringify({ coding: false, lobster: false }),
        agents
      )
    ).toEqual({
      coding: true,
      lobster: false,
    });
  });

  it("keeps the previous visible group when toggling would hide every platform group", () => {
    expect(
      ensureAtLeastOnePlatformCategoryVisible(
        { coding: false, lobster: false },
        { coding: true, lobster: false },
        [
          {
            ...baseAgent,
            id: "claude-code",
            display_name: "Claude Code",
            category: "coding",
            is_enabled: true,
          },
        ]
      )
    ).toEqual({
      coding: true,
      lobster: false,
    });
  });

  it("falls back to the default visible group when filtering an all-hidden stale setting", () => {
    const agents: AgentWithStatus[] = [
      {
        ...baseAgent,
        id: "claude-code",
        display_name: "Claude Code",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "central",
        display_name: "Central Skills",
        category: "central",
        is_enabled: true,
      },
    ];

    expect(
      filterVisiblePlatformAgents(agents, {
        coding: false,
        lobster: false,
      }).map((agent) => agent.id)
    ).toEqual(["claude-code"]);
  });

  it("sorts enabled tools first and keeps default platforms at the top", () => {
    const agents: AgentWithStatus[] = [
      {
        ...baseAgent,
        id: "cursor",
        display_name: "Cursor",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "antigravity-cli",
        display_name: "Antigravity CLI",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "opencode",
        display_name: "OpenCode",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "kiro",
        display_name: "Kiro",
        category: "coding",
        is_enabled: false,
      },
      {
        ...baseAgent,
        id: "codex",
        display_name: "Codex CLI",
        category: "coding",
        is_enabled: true,
      },
      {
        ...baseAgent,
        id: "aider",
        display_name: "Aider",
        category: "coding",
        is_enabled: false,
      },
    ];

    expect(sortPlatformVisibilityAgents(agents).map((agent) => agent.id)).toEqual([
      "codex",
      "antigravity-cli",
      "opencode",
      "cursor",
      "kiro",
      "aider",
    ]);
  });
});
