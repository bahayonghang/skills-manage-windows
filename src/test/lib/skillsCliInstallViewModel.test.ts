import { describe, expect, it } from "vitest";

import {
  buildInstallCommandPreview,
  buildPlatformInstalledCounts,
  createInitialInstallSession,
  defaultSelectedPlatformIds,
  defaultUninstalledSkillNames,
  installDialogReducer,
  installStepStatus,
  joinInstallPlatformTargets,
  mapSelectedCliAgents,
  mapSelectedSkillportAgentIds,
  nextRecentSources,
  parseRecentSourcesSetting,
  quoteInstallPreviewToken,
  uniqueStable,
} from "@/pages/skillsCliInstallViewModel";
import type {
  AgentWithStatus,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
} from "@/types";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: "cursor",
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "claude-code",
    displayName: "Claude Code",
    iconName: null,
    cliAgent: "claude",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: "amp",
    cliAgent: "amp",
    isEnabled: false,
    defaultSelected: false,
  },
];

function placement(
  agentId: string,
  state: SkillsCliPlacement["state"],
): SkillsCliPlacement {
  return {
    agentId,
    displayName: agentId,
    targetPath: `/tmp/${agentId}/skills/x`,
    state,
    managedLinkKind: state === "managed_link" ? "windows_junction" : null,
    reasonCode: null,
    installOrigin: null,
  };
}

function skill(
  name: string,
  placements: SkillsCliPlacement[],
): SkillsCliGlobalSkill {
  return {
    name,
    path: `/tmp/${name}`,
    installKind: "canonical",
    scope: "global",
    agents: [],
    source: "owner/repo",
    sourceUrl: null,
    sourceType: "github",
    sourceTypeBucket: "github",
    canonicalPath: `/tmp/${name}`,
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements,
  };
}

describe("installDialogReducer session correlation", () => {
  it("increments sessionId on open and ignores stale preview settle", () => {
    const opened = installDialogReducer(
      createInitialInstallSession(0),
      { type: "opened" },
    );
    expect(opened.sessionId).toBe(1);
    expect(opened.step).toBe("source");

    const pending = installDialogReducer(opened, {
      type: "previewStarted",
      source: "owner/repo",
      requestId: 7,
    });
    expect(pending.step).toBe("source");
    expect(pending.pendingPreviewId).toBe(7);

    const closed = installDialogReducer(pending, { type: "closed" });
    const reopened = installDialogReducer(closed, { type: "opened" });
    expect(reopened.sessionId).toBe(2);
    expect(reopened.step).toBe("source");
    expect(reopened.sourceInput).toBe("");
    expect(reopened.resolvedPreview).toBeNull();

    const stale = installDialogReducer(reopened, {
      type: "previewSucceeded",
      sessionId: 1,
      requestId: 7,
      result: { source: "owner/repo", skills: ["demo-skill"] },
      defaultSkillNames: ["demo-skill"],
      defaultPlatformIds: ["cursor"],
    });
    expect(stale).toBe(reopened);
    expect(stale.step).toBe("source");
    expect(stale.resolvedPreview).toBeNull();
  });

  it("advances only the matching preview and uses the normalized source", () => {
    let state = installDialogReducer(createInitialInstallSession(0), {
      type: "opened",
    });
    state = installDialogReducer(state, {
      type: "previewStarted",
      source: " Owner/Repo ",
      requestId: 1,
    });
    state = installDialogReducer(state, {
      type: "previewSucceeded",
      sessionId: state.sessionId,
      requestId: 1,
      result: { source: "owner/repo", skills: ["demo-skill", "helper-skill"] },
      defaultSkillNames: ["helper-skill"],
      defaultPlatformIds: ["cursor", "claude-code"],
    });
    expect(state.step).toBe("skills");
    expect(state.sourceInput).toBe("owner/repo");
    expect(state.resolvedPreview?.source).toBe("owner/repo");
    expect([...state.selectedSkillNames]).toEqual(["helper-skill"]);
    expect([...state.selectedPlatformIds]).toEqual(["cursor", "claude-code"]);
  });

  it("keeps step 1 and clears preview on a matching failure", () => {
    let state = installDialogReducer(createInitialInstallSession(0), {
      type: "opened",
    });
    state = installDialogReducer(state, {
      type: "previewStarted",
      source: "owner/repo",
      requestId: 3,
    });
    state = installDialogReducer(state, {
      type: "previewFailed",
      sessionId: state.sessionId,
      requestId: 3,
      message: "The skill source is not allowed.",
    });
    expect(state.step).toBe("source");
    expect(state.sourceInput).toBe("owner/repo");
    expect(state.resolvedPreview).toBeNull();
    expect(state.submitError).toBe("The skill source is not allowed.");
    expect(state.pendingPreviewId).toBeNull();
  });
});

describe("skill and platform selection helpers", () => {
  it("defaults to uninstalled skills and defaultSelected platforms", () => {
    expect(
      defaultUninstalledSkillNames(
        ["demo-skill", "helper-skill", "extra"],
        new Set(["demo-skill"]),
      ),
    ).toEqual(["helper-skill", "extra"]);
    expect(defaultSelectedPlatformIds(targets)).toEqual([
      "cursor",
      "claude-code",
    ]);
  });

  it("maps selected platform ids to skillport ids and unique cliAgents in target order", () => {
    const selected = new Set(["amp", "cursor", "claude-code"]);
    expect(mapSelectedSkillportAgentIds(targets, selected)).toEqual([
      "cursor",
      "claude-code",
      "amp",
    ]);
    expect(mapSelectedCliAgents(targets, selected)).toEqual([
      "cursor",
      "claude",
      "amp",
    ]);
    expect(
      mapSelectedCliAgents(
        [
          ...targets,
          {
            id: "claude-desktop",
            displayName: "Claude Desktop",
            iconName: null,
            cliAgent: "claude",
            isEnabled: true,
            defaultSelected: false,
          },
        ],
        new Set(["claude-code", "claude-desktop"]),
      ),
    ).toEqual(["claude"]);
  });

  it("joins CLI targets onto platform agents without grouping universal ids", () => {
    const agents: AgentWithStatus[] = [
      {
        id: "cursor",
        display_name: "Cursor App",
        category: "coding",
        global_skills_dir: "~/.cursor/skills",
        icon_name: "cursor",
        is_detected: true,
        is_builtin: true,
        is_enabled: true,
      },
    ];
    const joined = joinInstallPlatformTargets(targets, agents);
    expect(joined.map((target) => target.id)).toEqual([
      "cursor",
      "claude-code",
      "amp",
    ]);
    expect(joined[0]?.display_name).toBe("Cursor");
    expect(joined[1]?.id).toBe("claude-code");
  });

  it("counts only managed_link and direct_copy placements per platform id", () => {
    const skills = [
      skill("one", [
        placement("cursor", "managed_link"),
        placement("amp", "missing"),
      ]),
      skill("two", [
        placement("cursor", "direct_copy"),
        placement("amp", "conflict"),
      ]),
      skill("three", [placement("claude-code", "unavailable")]),
    ];
    expect(buildPlatformInstalledCounts(skills, targets)).toEqual({
      cursor: 2,
      "claude-code": 0,
      amp: 0,
    });
  });
});

describe("buildInstallCommandPreview", () => {
  it("renders npx tokens in build_add_global_argv order with repeated flags", () => {
    const preview = buildInstallCommandPreview({
      npmSpec: "skills@1.5.23",
      source: "owner/repo",
      skillNames: ["demo-skill", "helper-skill", "demo-skill"],
      cliAgents: ["cursor", "claude", "cursor"],
    });
    expect(preview).toBe(
      "npx skills@1.5.23 add owner/repo -s demo-skill -s helper-skill -g -a cursor -a claude -y",
    );
    expect(preview).not.toContain("--force");
    expect(preview).not.toContain("--keep-links");
    expect(preview).not.toMatch(/-a\s+\S+,/);
    expect(preview).not.toMatch(/-s\s+\S+,/);
  });

  it("returns empty preview when a required selection is missing", () => {
    expect(
      buildInstallCommandPreview({
        npmSpec: "skills@1.5.23",
        source: "owner/repo",
        skillNames: [],
        cliAgents: ["cursor"],
      }),
    ).toBe("");
  });

  it("quotes unsafe tokens for display only", () => {
    expect(quoteInstallPreviewToken("owner/repo")).toBe("owner/repo");
    expect(quoteInstallPreviewToken('a"b')).toBe('"a\\"b"');
    expect(uniqueStable(["b", "a", "b"])).toEqual(["b", "a"]);
  });
});

describe("install step status and recent helpers", () => {
  it("marks source/skills/platforms as current, completed, or pending", () => {
    expect(installStepStatus("skills", "source")).toBe("completed");
    expect(installStepStatus("skills", "skills")).toBe("current");
    expect(installStepStatus("skills", "platforms")).toBe("pending");
  });

  it("parses recent sources fail-closed and prepends unique items", () => {
    expect(parseRecentSourcesSetting(null)).toEqual({ ok: true, sources: [] });
    expect(parseRecentSourcesSetting("[]")).toEqual({ ok: true, sources: [] });
    expect(
      parseRecentSourcesSetting('["owner/repo","https://github.com/a/b"]'),
    ).toEqual({
      ok: true,
      sources: ["owner/repo", "https://github.com/a/b"],
    });
    expect(parseRecentSourcesSetting("{not json")).toEqual({ ok: false });
    expect(parseRecentSourcesSetting('{"source":"owner/repo"}')).toEqual({
      ok: false,
    });
    expect(parseRecentSourcesSetting('[" owner/repo"]')).toEqual({ ok: false });
    expect(
      parseRecentSourcesSetting(
        '["https://user:token@github.com/owner/repo"]',
      ),
    ).toEqual({ ok: false });
    expect(parseRecentSourcesSetting('["owner/repo","owner/repo"]')).toEqual({
      ok: false,
    });
    expect(
      nextRecentSources(["b", "c"], "a", 8),
    ).toEqual(["a", "b", "c"]);
    expect(nextRecentSources(["a", "b"], "b", 8)).toEqual(["b", "a"]);
    expect(
      nextRecentSources(["1", "2", "3", "4", "5", "6", "7", "8"], "9", 8),
    ).toEqual(["9", "1", "2", "3", "4", "5", "6", "7"]);
  });
});
