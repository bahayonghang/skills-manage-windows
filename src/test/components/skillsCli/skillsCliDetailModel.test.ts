import { describe, expect, it } from "vitest";

import en from "@/i18n/locales/en.json";
import zh from "@/i18n/locales/zh.json";
import {
  applySkillDocResponse,
  buildSkillsCliDetailRows,
  consumeDetailFocus,
  detailFocusForEntry,
  folderHashPrefix,
  skillLocalTimestamp,
  skillsCliDrawerPanelWidth,
  summarizeDetailPlacements,
  visibleSkillDocState,
  type SkillsCliDocState,
} from "@/pages/skillsCliDetailModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
} from "@/types";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "codex",
    displayName: "Codex",
    iconName: null,
    cliAgent: "codex",
    isEnabled: false,
    defaultSelected: false,
  },
];

function placement(
  agentId: string,
  state: SkillsCliPlacementState,
  displayName = agentId,
): SkillsCliPlacement {
  return {
    agentId,
    displayName,
    targetPath: `/tmp/${agentId}/skills/demo`,
    state,
    managedLinkKind: state === "managed_link" ? "windows_junction" : null,
    reasonCode:
      state === "conflict"
        ? "skills_cli.placement_conflict"
        : state === "unavailable"
          ? "skills_cli.placement_unavailable"
          : null,
    installOrigin: null,
  };
}

function skill(placements: SkillsCliPlacement[]): SkillsCliGlobalSkill {
  return {
    name: "demo",
    path: "/tmp/demo",
    installKind: "canonical",
    scope: "global",
    agents: [],
    source: "owner/repo",
    sourceUrl: null,
    sourceType: "github",
    sourceTypeBucket: "github",
    canonicalPath: "/tmp/demo",
    folderHash: "abcdef1234567890",
    installedAt: "2026-08-26T11:00:00Z",
    updatedAt: null,
    placements,
  };
}

describe("skillsCliDetailModel rows", () => {
  it("maps the five placements, preserves target order, and partitions link/unlink", () => {
    const rows = buildSkillsCliDetailRows(
      skill([
        placement("codex", "unavailable", "Codex"),
        placement("amp", "missing", "Amp"),
        placement("cursor", "managed_link", "Cursor"),
        placement("other", "conflict", "Other"),
      ]),
      targets,
    );
    expect(rows.map((row) => row.agentId)).toEqual(["cursor", "amp", "codex"]);
    expect(rows.map((row) => row.state)).toEqual([
      "managed_link",
      "missing",
      "unavailable",
    ]);
    expect(rows[0]).toMatchObject({
      associated: true,
      switchChecked: true,
      switchDisabled: false,
      action: "unlink",
    });
    expect(rows[1]).toMatchObject({
      associated: false,
      switchChecked: false,
      switchDisabled: false,
      action: "link",
    });
    expect(rows[2]).toMatchObject({
      associated: false,
      switchChecked: false,
      switchDisabled: true,
      action: null,
      reasonKind: "unavailable",
    });

    const copyConflict = buildSkillsCliDetailRows(
      skill([
        placement("cursor", "direct_copy", "Cursor"),
        placement("amp", "conflict", "Amp"),
      ]),
      targets,
    );
    expect(copyConflict[0]).toMatchObject({
      associated: true,
      switchChecked: true,
      switchDisabled: true,
      action: null,
      reasonKind: "direct_copy",
    });
    expect(copyConflict[1]).toMatchObject({
      associated: false,
      switchDisabled: true,
      action: null,
      reasonKind: "conflict",
      forceUnlinkable: false,
    });

    const relativeConflict = buildSkillsCliDetailRows(
      skill([
        {
          ...placement("amp", "conflict", "Amp"),
          reasonCode: "wrong_link_target",
        },
      ]),
      targets,
    );
    expect(relativeConflict[0]?.forceUnlinkable).toBe(true);
  });

  it("counts associated as managed_link plus direct_copy and prefers Link all over Unlink all", () => {
    const mixed = buildSkillsCliDetailRows(
      skill([
        placement("cursor", "managed_link", "Cursor"),
        placement("amp", "missing", "Amp"),
        placement("codex", "direct_copy", "Codex"),
      ]),
      targets,
    );
    const summary = summarizeDetailPlacements(mixed, targets);
    expect(summary.associatedCount).toBe(2);
    expect(summary.enabledCount).toBe(2);
    expect(summary.missingAgentIds).toEqual(["amp"]);
    expect(summary.managedLinkAgentIds).toEqual(["cursor"]);
    expect(summary.aggregate).toBe("linkAll");

    const unlinkOnly = summarizeDetailPlacements(
      buildSkillsCliDetailRows(
        skill([
          placement("cursor", "managed_link", "Cursor"),
          placement("amp", "direct_copy", "Amp"),
        ]),
        targets,
      ),
      targets,
    );
    expect(unlinkOnly.associatedCount).toBe(2);
    expect(unlinkOnly.aggregate).toBe("unlinkAll");
    expect(unlinkOnly.missingAgentIds).toEqual([]);

    const blocked = summarizeDetailPlacements(
      buildSkillsCliDetailRows(
        skill([
          placement("cursor", "direct_copy", "Cursor"),
          placement("amp", "conflict", "Amp"),
        ]),
        targets,
      ),
      targets,
    );
    expect(blocked.associatedCount).toBe(1);
    expect(blocked.aggregate).toBe("disabled");
    expect(blocked.missingAgentIds).toEqual([]);
    expect(blocked.managedLinkAgentIds).toEqual([]);
  });
});

describe("skillsCliDetailModel width, focus, hash, and doc guard", () => {
  it("uses content width 719 as full and 720 as 460px", () => {
    expect(skillsCliDrawerPanelWidth(719)).toEqual({
      mode: "fullWidth",
      cssWidth: "719px",
    });
    expect(skillsCliDrawerPanelWidth(720)).toEqual({
      mode: "fixed460",
      cssWidth: "460px",
    });
    expect(skillsCliDrawerPanelWidth(null).mode).toBe("fullWidth");
  });

  it("opens ordinary detail with null focus and consumes a links intent", () => {
    expect(detailFocusForEntry("detail")).toBeNull();
    expect(detailFocusForEntry("manageLinks")).toBe("links");
    expect(consumeDetailFocus("links")).toBeNull();
    expect(consumeDetailFocus(null)).toBeNull();
  });

  it("takes the first 7 hash characters and prefers updatedAt over installedAt", () => {
    expect(folderHashPrefix("abcdef1234567890")).toBe("abcdef1");
    expect(folderHashPrefix("  ")).toBeNull();
    expect(folderHashPrefix(null)).toBeNull();
    expect(
      skillLocalTimestamp({
        updatedAt: "2026-08-26T12:00:00Z",
        installedAt: "2026-08-01T00:00:00Z",
      }),
    ).toBe("2026-08-26T12:00:00Z");
    expect(
      skillLocalTimestamp({ updatedAt: null, installedAt: null }),
    ).toBeNull();
  });

  it("keeps en/zh detail keys and new backend codes aligned", () => {
    function keysOf(value: unknown, prefix = ""): string[] {
      if (!value || typeof value !== "object" || Array.isArray(value)) {
        return prefix ? [prefix] : [];
      }
      return Object.entries(value as Record<string, unknown>).flatMap(
        ([key, child]) => keysOf(child, prefix ? `${prefix}.${key}` : key),
      );
    }
    expect(keysOf(en.skillsCli.detail)).toEqual(keysOf(zh.skillsCli.detail));
    expect(keysOf(en.backendErrors.skills_cli)).toEqual(
      keysOf(zh.backendErrors.skills_cli),
    );
    expect(en.skillsCli.detail.hashTooltip.toLowerCase()).toContain(
      "local content hash",
    );
    expect(en.skillsCli.detail.hashTooltip.toLowerCase()).not.toBe("commit");
    expect(zh.skillsCli.detail.hashTooltip).toContain("本地内容哈希");
  });

  it("ignores a stale doc response after the skill or request id changes", () => {
    const loadingA: SkillsCliDocState = {
      status: "loading",
      skillName: "alpha",
      requestId: "req-a",
    };
    expect(
      applySkillDocResponse(loadingA, "req-a", "alpha", {
        ok: true,
        content: "---\nname: alpha\n---\n",
        byteSize: 20,
      }),
    ).toMatchObject({ status: "ready", skillName: "alpha", byteSize: 20 });
    expect(
      applySkillDocResponse(loadingA, "req-old", "alpha", {
        ok: true,
        content: "stale",
        byteSize: 5,
      }),
    ).toEqual(loadingA);
    expect(
      applySkillDocResponse(
        { status: "loading", skillName: "beta", requestId: "req-b" },
        "req-a",
        "alpha",
        { ok: true, content: "stale", byteSize: 5 },
      ),
    ).toMatchObject({ status: "loading", skillName: "beta" });
    expect(
      applySkillDocResponse(loadingA, "req-a", "alpha", {
        ok: true,
        content: "",
        byteSize: 0,
      }),
    ).toEqual({ status: "empty", skillName: "alpha", byteSize: 0 });
    expect(
      applySkillDocResponse(loadingA, "req-a", "alpha", {
        ok: false,
        errorCode: "skills_cli.skill_doc_missing",
      }),
    ).toEqual({
      status: "error",
      skillName: "alpha",
      errorCode: "skills_cli.skill_doc_missing",
    });
  });

  it("shows loading instead of another skill's ready or idle doc", () => {
    const readyAlpha: SkillsCliDocState = {
      status: "ready",
      skillName: "alpha",
      content: "stale-alpha",
      byteSize: 11,
    };
    expect(visibleSkillDocState("beta", readyAlpha)).toEqual({
      status: "loading",
      skillName: "beta",
      requestId: "pending",
    });
    expect(visibleSkillDocState("alpha", { status: "idle" })).toEqual({
      status: "loading",
      skillName: "alpha",
      requestId: "pending",
    });
    expect(visibleSkillDocState("alpha", readyAlpha)).toEqual(readyAlpha);
    expect(visibleSkillDocState(null, readyAlpha)).toEqual({ status: "idle" });
  });
});
