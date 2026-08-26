import { beforeEach, describe, expect, it } from "vitest";

import { ipcFixtureError } from "@/lib/ipc/errors";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { ipcInvokeCalls, mockIpcCommand, mockIpcCommands } from "@/test/support/ipcMock";

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
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
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
  });
}

describe("skillsCliStore", () => {
  beforeEach(resetState);

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

  it("keeps the inventory when doctor rejects cli_unavailable", async () => {
    mockIpcCommand("skills_cli_doctor", () => {
      throw ipcFixtureError(
        "skills_cli.cli_unavailable",
        "The Skills CLI package could not be executed.",
      );
    });
    mockIpcCommand("skills_cli_list_global", listGlobal);
    mockIpcCommand("skills_cli_install_targets", targets);

    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().runtimeError).toContain(
      "skills_cli.cli_unavailable",
    );
    expect(useSkillsCliStore.getState().runtimeError).not.toContain("npm ERR!");
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
