import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mockListen = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockListen,
}));

import { ipcFixtureError } from "@/lib/ipc/errors";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { ipcInvokeCalls, ipcInvokedCommands, mockIpcCommand, mockIpcCommands } from "@/test/support/ipcMock";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliUpdateInventory,
} from "@/types";

const doctor = { nodeVersion: "v22.20.0", npmSpec: "skills@1.5.23" };
const skills = [
  {
    name: "demo-skill",
    path: "/tmp/demo-skill",
    agents: ["cursor"],
    source: "owner/repo",
  },
];
const listGlobal = {
  skills,
  canonicalRoot: "/tmp/agents",
  lockPath: "/tmp/agents/skills.lock",
};
const targets = [
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
    isEnabled: false,
    defaultSelected: false,
  },
];

function resetState() {
  useSkillsCliStore.setState({
    skills: [],
    targets: [],
    preview: null,
    doctor: null,
    canonicalRoot: null,
    lockPath: null,
    isLoading: false,
    isRefreshing: false,
    isPreviewing: false,
    isMutating: false,
    isCancelling: false,
    jobId: null,
    runtimeError: null,
    inventoryError: null,
    actionError: null,
    docState: { status: "idle" },
    updateInventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    isLoadingUpdateCache: false,
    updateJob: { jobId: null, phase: null },
    updateError: null,
    updateProgress: null,
  });
}

describe("skillsCliStore", () => {
  beforeEach(() => {
    resetState();
    mockListen.mockReset();
    mockListen.mockResolvedValue(vi.fn());
  });

  it("loads doctor, list, and install targets with snapshot paths", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
    });

    await useSkillsCliStore.getState().loadAll();

    expect(ipcInvokeCalls("skills_cli_doctor")).toHaveLength(1);
    expect(ipcInvokeCalls("skills_cli_list_global")).toHaveLength(1);
    expect(ipcInvokeCalls("skills_cli_install_targets")).toHaveLength(1);
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().targets).toEqual(targets);
    expect(useSkillsCliStore.getState().doctor).toEqual(doctor);
    expect(useSkillsCliStore.getState().canonicalRoot).toBe("/tmp/agents");
    expect(useSkillsCliStore.getState().lockPath).toBe("/tmp/agents/skills.lock");
    expect(useSkillsCliStore.getState().runtimeError).toBeNull();
    expect(useSkillsCliStore.getState().inventoryError).toBeNull();
  });

  it("keeps the inventory when doctor rejects node_missing", async () => {
    mockIpcCommand("skills_cli_doctor", () => {
      throw ipcFixtureError(
        "skills_cli.node_missing",
        "Node.js 22.20 or later is required.",
      );
    });
    mockIpcCommand("skills_cli_list_global", listGlobal);
    mockIpcCommand("skills_cli_install_targets", targets);

    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().runtimeError).toContain(
      "skills_cli.node_missing",
    );
    expect(useSkillsCliStore.getState().runtimeError).not.toContain("npm ERR!");
    expect(useSkillsCliStore.getState().inventoryError).toBeNull();
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().doctor).toBeNull();
  });

  it("keeps the inventory when doctor rejects timeout", async () => {
    mockIpcCommand("skills_cli_doctor", () => {
      throw ipcFixtureError(
        "skills_cli.timeout",
        "The Skills CLI command timed out.",
      );
    });
    mockIpcCommand("skills_cli_list_global", listGlobal);
    mockIpcCommand("skills_cli_install_targets", targets);

    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().runtimeError).toContain(
      "skills_cli.timeout",
    );
    expect(useSkillsCliStore.getState().inventoryError).toBeNull();
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().doctor).toBeNull();
  });

  it("keeps stale skills and surfaces inventoryError when list fails on refresh", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
    });
    await useSkillsCliStore.getState().loadAll();

    mockIpcCommand("skills_cli_list_global", () => {
      throw ipcFixtureError("internal.unexpected", "list failed");
    });
    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().inventoryError).toContain(
      "internal.unexpected",
    );
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().doctor).toEqual(doctor);
    expect(useSkillsCliStore.getState().runtimeError).toBeNull();
    expect(useSkillsCliStore.getState().isRefreshing).toBe(false);
  });

  it("reports a failed first load as inventoryError without fabricating skills", async () => {
    mockIpcCommand("skills_cli_list_global", () => {
      throw ipcFixtureError("internal.unexpected", "list failed");
    });
    mockIpcCommand("skills_cli_install_targets", targets);
    mockIpcCommand("skills_cli_doctor", doctor);

    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().skills).toEqual([]);
    expect(useSkillsCliStore.getState().inventoryError).toContain(
      "internal.unexpected",
    );
  });

  it("refuses empty add selections without invoking", async () => {
    await expect(
      useSkillsCliStore.getState().addGlobal({
        source: "owner/repo",
        skillNames: [],
        skillportAgentIds: ["cursor"],
      }),
    ).rejects.toThrow(/skills_cli.selection_empty/);
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(0);
    expect(useSkillsCliStore.getState().actionError).toContain(
      "skills_cli.selection_empty",
    );
  });

  it("sends jobId and the changed selection payload on add", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_add_global: {
        installedSkills: 1,
        targetedPlatforms: 1,
      },
    });

    const result = await useSkillsCliStore.getState().addGlobal({
      source: "owner/repo",
      skillNames: ["helper-skill"],
      skillportAgentIds: ["amp"],
    });

    expect(result).toEqual({ installedSkills: 1, targetedPlatforms: 1 });
    const calls = ipcInvokeCalls("skills_cli_add_global");
    expect(calls).toHaveLength(1);
    const args = calls[0].args as {
      jobId: string;
      source: string;
      skillNames: string[];
      skillportAgentIds: string[];
    };
    expect(args.jobId).toMatch(/\S/);
    expect(args.source).toBe("owner/repo");
    expect(args.skillNames).toEqual(["helper-skill"]);
    expect(args.skillportAgentIds).toEqual(["amp"]);
    expect(ipcInvokeCalls("skills_cli_list_global")).toHaveLength(0);
    expect(ipcInvokeCalls("skills_cli_install_targets")).toHaveLength(0);
    expect(ipcInvokeCalls("skills_cli_doctor")).toHaveLength(0);
    expect(useSkillsCliStore.getState().isMutating).toBe(false);
    expect(useSkillsCliStore.getState().jobId).toBeNull();
    expect(useSkillsCliStore.getState().preview).toBeNull();
  });

  it("rejects a busy start without replacing the running job", async () => {
    useSkillsCliStore.setState({
      isMutating: true,
      jobId: "job-running",
      actionError: null,
    });
    await expect(
      useSkillsCliStore.getState().addGlobal({
        source: "owner/repo",
        skillNames: ["helper-skill"],
        skillportAgentIds: ["amp"],
      }),
    ).rejects.toThrow(/skills_cli.busy/);
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(0);
    expect(useSkillsCliStore.getState().jobId).toBe("job-running");
    expect(useSkillsCliStore.getState().isMutating).toBe(true);
    expect(useSkillsCliStore.getState().actionError).toBeNull();
  });

  it("writes actionError and rethrows the current job's backend failure", async () => {
    mockIpcCommand("skills_cli_add_global", () => {
      throw ipcFixtureError(
        "skills_cli.cli_unavailable",
        "The Skills CLI package could not be executed.",
      );
    });
    await expect(
      useSkillsCliStore.getState().addGlobal({
        source: "owner/repo",
        skillNames: ["helper-skill"],
        skillportAgentIds: ["amp"],
      }),
    ).rejects.toThrow(/Skills CLI package could not be executed/);
    expect(useSkillsCliStore.getState().actionError).toContain(
      "skills_cli.cli_unavailable",
    );
    expect(useSkillsCliStore.getState().isMutating).toBe(false);
    expect(useSkillsCliStore.getState().jobId).toBeNull();
    expect(ipcInvokeCalls("skills_cli_list_global")).toHaveLength(0);
  });

  it("does not let a stale add completion overwrite a successor job", async () => {
    let resolveAdd:
      | ((value: { installedSkills: number; targetedPlatforms: number }) => void)
      | undefined;
    mockIpcCommand(
      "skills_cli_add_global",
      () =>
        new Promise((resolve) => {
          resolveAdd = resolve;
        }),
    );
    const pending = useSkillsCliStore.getState().addGlobal({
      source: "owner/repo",
      skillNames: ["helper-skill"],
      skillportAgentIds: ["amp"],
    });
    expect(useSkillsCliStore.getState().jobId).toMatch(/\S/);
    useSkillsCliStore.setState({
      jobId: "job-successor",
      isMutating: true,
      preview: { source: "keep", skills: ["keep"] },
    });
    resolveAdd?.({ installedSkills: 1, targetedPlatforms: 1 });
    await expect(pending).resolves.toEqual({
      installedSkills: 1,
      targetedPlatforms: 1,
    });
    expect(useSkillsCliStore.getState().jobId).toBe("job-successor");
    expect(useSkillsCliStore.getState().isMutating).toBe(true);
    expect(useSkillsCliStore.getState().preview).toEqual({
      source: "keep",
      skills: ["keep"],
    });
  });

  it("does not let a stale add failure overwrite a successor job", async () => {
    let rejectAdd: ((error: unknown) => void) | undefined;
    mockIpcCommand(
      "skills_cli_add_global",
      () =>
        new Promise((_, reject) => {
          rejectAdd = reject;
        }),
    );
    const pending = useSkillsCliStore.getState().addGlobal({
      source: "owner/repo",
      skillNames: ["helper-skill"],
      skillportAgentIds: ["amp"],
    });
    useSkillsCliStore.setState({
      jobId: "job-successor",
      isMutating: true,
      actionError: null,
      preview: { source: "keep", skills: ["keep"] },
    });
    rejectAdd?.(
      ipcFixtureError(
        "skills_cli.cli_unavailable",
        "The Skills CLI package could not be executed.",
      ),
    );
    await expect(pending).rejects.toThrow(/Skills CLI package could not be executed/);
    expect(useSkillsCliStore.getState().jobId).toBe("job-successor");
    expect(useSkillsCliStore.getState().isMutating).toBe(true);
    expect(useSkillsCliStore.getState().actionError).toBeNull();
    expect(useSkillsCliStore.getState().preview).toEqual({
      source: "keep",
      skills: ["keep"],
    });
    expect(ipcInvokeCalls("skills_cli_list_global")).toHaveLength(0);
  });

  it("cancels with the current job id", async () => {
    mockIpcCommand("cancel_skills_cli_job", true);
    useSkillsCliStore.setState({ jobId: "job-keep" });
    await useSkillsCliStore.getState().cancelJob();
    expect(ipcInvokeCalls("cancel_skills_cli_job")).toEqual([
      { command: "cancel_skills_cli_job", args: { jobId: "job-keep" } },
    ]);
  });

  it("does not invoke cancel when jobId is null", async () => {
    await useSkillsCliStore.getState().cancelJob();
    expect(ipcInvokeCalls("cancel_skills_cli_job")).toHaveLength(0);
  });
});

function placement(
  agentId: string,
  state: "managed_link" | "direct_copy" | "missing" | "conflict" | "unavailable",
  displayName = agentId,
) {
  return {
    agentId,
    displayName,
    targetPath: `/tmp/${agentId}/skills/x`,
    state,
    managedLinkKind: state === "managed_link" ? ("windows_junction" as const) : null,
    reasonCode: null,
  };
}

function globalSkill(
  name: string,
  placements: ReturnType<typeof placement>[],
) {
  return {
    name,
    path: `/tmp/${name}`,
    installKind: "canonical" as const,
    scope: "global",
    agents: [],
    source: "owner/repo",
    sourceUrl: null,
    sourceType: "github",
    sourceTypeBucket: "github" as const,
    canonicalPath: `/tmp/${name}`,
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements,
  };
}

const linkedPlacement = {
  agentId: "cursor",
  displayName: "Cursor",
  targetPath: "/tmp/cursor/skills/x",
  state: "managed_link" as const,
  managedLinkKind: "windows_junction" as const,
  reasonCode: null,
};

const missingPlacement = {
  ...linkedPlacement,
  state: "missing" as const,
  managedLinkKind: null,
};

describe("skillsCliStore placement mutations", () => {
  beforeEach(resetState);

  it("skips link/unlink IPC for direct_copy, conflict, and unavailable", async () => {
    useSkillsCliStore.setState({
      skills: [
        globalSkill("copy", [placement("cursor", "direct_copy")]),
        globalSkill("conflict", [placement("cursor", "conflict")]),
        globalSkill("gone", [placement("cursor", "unavailable")]),
      ],
    });
    await expect(
      useSkillsCliStore.getState().linkPlatform("copy", "cursor"),
    ).rejects.toThrow(/direct_copy_not_toggleable/);
    await expect(
      useSkillsCliStore.getState().linkPlatform("conflict", "cursor"),
    ).rejects.toThrow(/placement_conflict/);
    await expect(
      useSkillsCliStore.getState().linkPlatform("gone", "cursor"),
    ).rejects.toThrow(/placement_unavailable/);
    await expect(
      useSkillsCliStore.getState().unlinkPlatform("copy", "cursor"),
    ).rejects.toThrow(/direct_copy_not_toggleable/);
    expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(0);
    expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(0);
  });

  it("links only missing placements serially, rolls back a failed item, and keeps a failed refresh from flipping success", async () => {
    const missingA = globalSkill("alpha", [placement("cursor", "missing")]);
    const missingB = globalSkill("beta", [placement("cursor", "missing")]);
    const copyC = globalSkill("gamma", [placement("cursor", "direct_copy")]);
    useSkillsCliStore.setState({
      skills: [missingA, missingB, copyC],
      targets,
      doctor,
    });
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_install_targets: targets,
      skills_cli_list_global: () => {
        throw ipcFixtureError("internal.unexpected", "list failed");
      },
      skills_cli_link_platform: ({ skillName }: { skillName: string }) => {
        if (skillName === "beta") {
          throw ipcFixtureError(
            "skills_cli.busy",
            "Another skill operation is using this target.",
          );
        }
        return linkedPlacement;
      },
    });

    const outcome = await useSkillsCliStore.getState().linkPlatformBatch(
      ["alpha", "beta", "gamma"],
      "cursor",
    );

    expect(outcome.succeeded).toEqual([{ skillName: "alpha", agentId: "cursor" }]);
    expect(outcome.failed).toEqual([
      { skillName: "beta", agentId: "cursor", errorCode: "skills_cli.busy" },
    ]);
    expect(outcome.skipped).toEqual([
      {
        skillName: "gamma",
        agentId: "cursor",
        reasonCode: "skills_cli.direct_copy_not_toggleable",
      },
    ]);
    expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(2);
    expect(
      ipcInvokeCalls("skills_cli_link_platform").map(
        (call) => (call.args as { skillName: string }).skillName,
      ),
    ).toEqual(["alpha", "beta"]);
    const skills = useSkillsCliStore.getState().skills;
    expect(skills.find((item) => item.name === "alpha")?.placements[0]?.state).toBe(
      "managed_link",
    );
    expect(skills.find((item) => item.name === "beta")?.placements[0]?.state).toBe(
      "missing",
    );
    expect(skills.find((item) => item.name === "gamma")?.placements[0]?.state).toBe(
      "direct_copy",
    );
    expect(useSkillsCliStore.getState().inventoryError).toContain("internal.unexpected");
    expect(useSkillsCliStore.getState().isMutating).toBe(false);
  });

  it("keeps a successful mutation outcome when trailing refresh throws", async () => {
    const missingA = globalSkill("alpha", [placement("cursor", "missing")]);
    useSkillsCliStore.setState({
      skills: [missingA],
      targets,
      doctor,
    });
    mockIpcCommand("skills_cli_link_platform", linkedPlacement);
    const originalLoadAll = useSkillsCliStore.getState().loadAll;
    useSkillsCliStore.setState({
      loadAll: async () => {
        throw new Error("internal.unexpected:refresh boom");
      },
    });
    try {
      const outcome = await useSkillsCliStore.getState().linkPlatformBatch(
        ["alpha"],
        "cursor",
      );
      expect(outcome.succeeded).toEqual([
        { skillName: "alpha", agentId: "cursor" },
      ]);
      expect(outcome.failed).toEqual([]);
      expect(useSkillsCliStore.getState().inventoryError).toContain(
        "internal.unexpected",
      );
      expect(useSkillsCliStore.getState().isMutating).toBe(false);
    } finally {
      useSkillsCliStore.setState({ loadAll: originalLoadAll });
    }
  });

  it("unlinks only managed_link placements and does not send force or keep-links flags", async () => {
    useSkillsCliStore.setState({
      skills: [
        globalSkill("linked", [placement("cursor", "managed_link")]),
        globalSkill("copy", [placement("amp", "direct_copy")]),
      ],
      doctor,
      targets,
    });
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_install_targets: targets,
      skills_cli_list_global: listGlobal,
      skills_cli_unlink_platform: missingPlacement,
    });
    const outcome = await useSkillsCliStore.getState().unlinkManagedBatch([
      "linked",
      "copy",
    ]);
    expect(outcome.succeeded).toEqual([{ skillName: "linked", agentId: "cursor" }]);
    expect(outcome.skipped[0]?.reasonCode).toBe(
      "skills_cli.direct_copy_not_toggleable",
    );
    expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(1);
    const args = ipcInvokeCalls("skills_cli_unlink_platform")[0]?.args as Record<
      string,
      unknown
    >;
    expect(args).toMatchObject({
      skillName: "linked",
      skillportAgentId: "cursor",
    });
    expect(args).not.toHaveProperty("force");
    expect(args).not.toHaveProperty("keepLinks");
    expect(JSON.stringify(args)).not.toContain("--force");
    expect(JSON.stringify(args)).not.toContain("--keep-links");
  });

  it("removes serially, restores a failed name, and still refreshes afterwards", async () => {
    const keep = globalSkill("keep", [placement("cursor", "managed_link")]);
    const drop = globalSkill("drop", [placement("cursor", "managed_link")]);
    useSkillsCliStore.setState({ skills: [keep, drop], doctor, targets });
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_install_targets: targets,
      skills_cli_list_global: {
        skills: [keep],
        canonicalRoot: "/tmp/agents",
        lockPath: "/tmp/agents/skills.lock",
      },
      skills_cli_remove_global: ({ skillName }: { skillName: string }) => {
        if (skillName === "keep") {
          throw ipcFixtureError(
            "skills_cli.placement_conflict",
            "Conflict",
          );
        }
        return {
          removedCanonical: true,
          removedManagedAgentIds: ["cursor"],
          retainedDirectCopyAgentIds: [],
        };
      },
    });
    const outcome = await useSkillsCliStore.getState().removeGlobalBatch([
      "keep",
      "drop",
    ]);
    expect(outcome.failed).toEqual([
      { skillName: "keep", errorCode: "skills_cli.placement_conflict" },
    ]);
    expect(outcome.succeeded).toEqual([{ skillName: "drop" }]);
    expect(useSkillsCliStore.getState().skills.map((item) => item.name)).toEqual([
      "keep",
    ]);
    expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(2);
  });

  it("writes export JSON through IPC and surfaces a coded failure", async () => {
    mockIpcCommand("skills_cli_export_inventory", null);
    await useSkillsCliStore.getState().exportInventory({
      path: "D:/tmp/out.json",
      json: "{\"schemaVersion\":1}\n",
    });
    expect(ipcInvokeCalls("skills_cli_export_inventory")).toEqual([
      {
        command: "skills_cli_export_inventory",
        args: { path: "D:/tmp/out.json", json: "{\"schemaVersion\":1}\n" },
      },
    ]);
    mockIpcCommand("skills_cli_export_inventory", () => {
      throw ipcFixtureError(
        "skills_cli.export_failed",
        "The export could not be written.",
      );
    });
    await expect(
      useSkillsCliStore.getState().exportInventory({
        path: "D:/tmp/out.json",
        json: "{}\n",
      }),
    ).rejects.toThrow(/export could not be written|skills_cli.export_failed/);
    expect(useSkillsCliStore.getState().actionError).toContain(
      "skills_cli.export_failed",
    );
  });
});

describe("skillsCliStore doc and reveal", () => {
  beforeEach(resetState);

  it("reads ready, empty, and error docs and ignores a stale response", async () => {
    mockIpcCommand("skills_cli_read_skill_md", {
      skillName: "demo-skill",
      content: "---\nname: demo\n---\nbody",
      byteSize: 24,
    });
    await useSkillsCliStore.getState().readSkillDoc("demo-skill");
    expect(ipcInvokeCalls("skills_cli_read_skill_md")[0]?.args).toEqual({
      skillName: "demo-skill",
    });
    expect(useSkillsCliStore.getState().docState).toMatchObject({
      status: "ready",
      skillName: "demo-skill",
      content: "---\nname: demo\n---\nbody",
      byteSize: 24,
    });

    mockIpcCommand("skills_cli_read_skill_md", {
      skillName: "empty-skill",
      content: "",
      byteSize: 0,
    });
    await useSkillsCliStore.getState().readSkillDoc("empty-skill");
    expect(useSkillsCliStore.getState().docState).toEqual({
      status: "empty",
      skillName: "empty-skill",
      byteSize: 0,
    });

    mockIpcCommand("skills_cli_read_skill_md", () => {
      throw ipcFixtureError(
        "skills_cli.skill_doc_missing",
        "SKILL.md was not found in the owned skill folder.",
      );
    });
    await useSkillsCliStore.getState().readSkillDoc("missing-skill");
    expect(useSkillsCliStore.getState().docState).toEqual({
      status: "error",
      skillName: "missing-skill",
      errorCode: "skills_cli.skill_doc_missing",
    });

    let releaseAlpha: (() => void) | undefined;
    mockIpcCommand(
      "skills_cli_read_skill_md",
      ({ skillName }: { skillName: string }) => {
        if (skillName === "alpha") {
          return new Promise((resolve) => {
            releaseAlpha = () =>
              resolve({
                skillName: "alpha",
                content: "stale-alpha",
                byteSize: 11,
              });
          });
        }
        return {
          skillName: "beta",
          content: "beta-doc",
          byteSize: 8,
        };
      },
    );
    const alphaDone = useSkillsCliStore.getState().readSkillDoc("alpha");
    const betaDone = useSkillsCliStore.getState().readSkillDoc("beta");
    await betaDone;
    releaseAlpha?.();
    await alphaDone;
    expect(useSkillsCliStore.getState().docState).toMatchObject({
      status: "ready",
      skillName: "beta",
      content: "beta-doc",
    });

    useSkillsCliStore.getState().clearSkillDoc("beta");
    expect(useSkillsCliStore.getState().docState).toEqual({ status: "idle" });
  });

  it("reveals by skillName only and keeps typed errors", async () => {
    mockIpcCommand("skills_cli_reveal_skill_folder", null);
    await useSkillsCliStore.getState().revealSkillFolder("demo-skill");
    const args = ipcInvokeCalls("skills_cli_reveal_skill_folder")[0]?.args as Record<
      string,
      unknown
    >;
    expect(args).toEqual({ skillName: "demo-skill" });
    expect(args).not.toHaveProperty("path");
    expect(JSON.stringify(args)).not.toContain("--force");
    expect(ipcInvokedCommands()).not.toContain("open_in_file_manager");

    mockIpcCommand("skills_cli_reveal_skill_folder", () => {
      throw ipcFixtureError(
        "skills_cli.skill_not_owned",
        "Skills CLI does not own that skill.",
      );
    });
    await expect(
      useSkillsCliStore.getState().revealSkillFolder("demo-skill"),
    ).rejects.toMatchObject({ code: "skills_cli.skill_not_owned" });
  });

  it("does not rewrite batch-owned link actions from the doc/reveal increment", () => {
    const store = readFileSync(
      resolve(process.cwd(), "src/stores/skillsCliStore.ts"),
      "utf8",
    );
    const updateSlice = readFileSync(
      resolve(process.cwd(), "src/stores/skillsCliStore.updateSlice.ts"),
      "utf8",
    );
    expect(store).toContain("async linkPlatform(");
    expect(store).toContain("async unlinkPlatform(");
    expect(store).toContain("async linkPlatformBatch(");
    expect(store).toContain("async unlinkManagedBatch(");
    expect(store).toContain("async removeGlobalBatch(");
    expect(store).toContain("async exportInventory(");
    expect(store).toContain("async readSkillDoc(");
    expect(store).toContain("async revealSkillFolder(");
    expect(updateSlice).toContain("async checkUpdates(");
    expect(updateSlice).toContain("skills-cli://update-progress");
  });

  it("loads update cache independently and keeps global inventory when the cache rejects", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: () => {
        throw ipcFixtureError(
          "skills_cli.update_migration",
          "The Skills CLI update database is not available.",
        );
      },
    });
    await useSkillsCliStore.getState().loadAll();
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().inventoryError).toBeNull();
    expect(useSkillsCliStore.getState().updateError).toContain(
      "skills_cli.update_migration",
    );
  });

  it("listens for update progress before invoking check updates", async () => {
    const inventory: SkillsCliUpdateInventory = {
      ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      lastSuccessAt: "2026-08-26T00:00:00.000Z",
    };
    const order: string[] = [];
    mockListen.mockImplementation(async () => {
      order.push("listen");
      return vi.fn();
    });
    mockIpcCommand("skills_cli_check_updates", () => {
      order.push("invoke");
      return inventory;
    });

    await useSkillsCliStore.getState().checkUpdates();

    expect(order).toEqual(["listen", "invoke"]);
    expect(mockListen.mock.calls[0]?.[0]).toBe("skills-cli://update-progress");
    expect(typeof mockListen.mock.calls[0]?.[1]).toBe("function");
    expect(ipcInvokeCalls("skills_cli_check_updates")[0]?.args).toEqual({
      jobId: expect.any(String),
    });
    expect(useSkillsCliStore.getState().updateInventory).toEqual(inventory);
    expect(useSkillsCliStore.getState().updateJob.phase).toBeNull();
  });

  it("ignores a stale check result after a newer job starts", async () => {
    let resolveFirst: ((value: SkillsCliUpdateInventory) => void) | undefined;
    mockIpcCommand(
      "skills_cli_check_updates",
      () =>
        new Promise<SkillsCliUpdateInventory>((resolve) => {
          resolveFirst = resolve;
        }),
    );
    const first = useSkillsCliStore.getState().checkUpdates();
    await vi.waitFor(() => {
      expect(resolveFirst).toBeTypeOf("function");
    });
    useSkillsCliStore.setState({
      updateJob: { jobId: "newer-job", phase: "checking" },
    });
    resolveFirst?.({
      ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      lastSuccessAt: "old",
    });
    await first;
    expect(useSkillsCliStore.getState().updateInventory.lastSuccessAt).not.toBe(
      "old",
    );
    expect(useSkillsCliStore.getState().updateJob.jobId).toBe("newer-job");
  });

  it("rejects a second check while an update job is running", async () => {
    useSkillsCliStore.setState({
      updateJob: { jobId: "job-running", phase: "checking" },
    });
    await expect(useSkillsCliStore.getState().checkUpdates()).rejects.toThrow(
      /skills_cli.busy/,
    );
    expect(ipcInvokeCalls("skills_cli_check_updates")).toHaveLength(0);
  });

  it("applies using backend pending tokens and never sends force flags", async () => {
    useSkillsCliStore.setState({
      updateInventory: {
        ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
        skills: [
          {
            skillName: "demo-skill",
            repositoryKey: "owner/repo@main",
            normalizedSource: "https://github.com/owner/repo",
            skillPath: "demo-skill",
            status: "update_available",
            installedRevisionSha: "aaa",
            observedRevisionSha: "bbb",
            pendingRevisionSha: "bbb",
            installedLocalDigest: "sha256-v1:a",
            observedUpstreamDigest: "sha256-v1:b",
            pendingUpstreamDigest: "sha256-v1:b",
            isStale: false,
            lastErrorCode: null,
            changeSummary: [],
            blockers: [],
            argvPreview: [
              "refresh",
              "owned-canonical",
              "from-pinned-github-snapshot",
              "demo-skill",
            ],
          },
        ],
      },
    });
    mockIpcCommands({
      skills_cli_apply_updates: {
        appliedSkillNames: ["demo-skill"],
        installedRevisionSha: "bbb",
      },
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    await useSkillsCliStore.getState().applyUpdates({
      repositoryKey: "owner/repo@main",
      skillNames: ["demo-skill"],
    });
    const payload = ipcInvokeCalls("skills_cli_apply_updates")[0]?.args as {
      request: {
        jobId: string;
        repositoryKey: string;
        selections: Array<{ skillName: string }>;
      };
    };
    expect(payload.request.repositoryKey).toBe("owner/repo@main");
    expect(payload.request.selections[0]?.skillName).toBe("demo-skill");
    expect(JSON.stringify(payload)).not.toContain("--force");
    expect(JSON.stringify(payload)).not.toContain("--keep-links");
  });

  it("does not treat a follow-up inventory refresh failure as an apply failure", async () => {
    useSkillsCliStore.setState({
      updateInventory: {
        ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
        skills: [
          {
            skillName: "demo-skill",
            repositoryKey: "owner/repo@main",
            normalizedSource: "https://github.com/owner/repo",
            skillPath: "demo-skill",
            status: "update_available",
            installedRevisionSha: "aaa",
            observedRevisionSha: "bbb",
            pendingRevisionSha: "bbb",
            installedLocalDigest: "sha256-v1:a",
            observedUpstreamDigest: "sha256-v1:b",
            pendingUpstreamDigest: "sha256-v1:b",
            isStale: false,
            lastErrorCode: null,
            changeSummary: [],
            blockers: [],
            argvPreview: [
              "refresh",
              "owned-canonical",
              "from-pinned-github-snapshot",
              "demo-skill",
            ],
          },
        ],
      },
    });
    mockIpcCommands({
      skills_cli_apply_updates: {
        appliedSkillNames: ["demo-skill"],
        installedRevisionSha: "bbb",
      },
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: () => {
        throw ipcFixtureError(
          "skills_cli.update_migration",
          "The Skills CLI update database is not available.",
        );
      },
    });
    await expect(
      useSkillsCliStore.getState().applyUpdates({
        repositoryKey: "owner/repo@main",
        skillNames: ["demo-skill"],
      }),
    ).resolves.toEqual({
      appliedSkillNames: ["demo-skill"],
      installedRevisionSha: "bbb",
    });
    expect(useSkillsCliStore.getState().updateJob.phase).toBeNull();
    expect(useSkillsCliStore.getState().updateError).toContain(
      "skills_cli.update_migration",
    );
  });
});


