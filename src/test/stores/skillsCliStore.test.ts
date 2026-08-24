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

describe("skillsCliStore", () => {
  beforeEach(() => {
    useSkillsCliStore.setState({
      skills: [],
      targets: [],
      preview: null,
      doctor: null,
      isLoading: false,
      isPreviewing: false,
      isMutating: false,
      isCancelling: false,
      jobId: null,
      error: null,
    });
  });

  it("loads doctor, list, and install targets", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: skills,
      skills_cli_install_targets: targets,
    });

    await useSkillsCliStore.getState().loadAll();

    expect(ipcInvokeCalls("skills_cli_doctor")).toHaveLength(1);
    expect(ipcInvokeCalls("skills_cli_list_global")).toHaveLength(1);
    expect(ipcInvokeCalls("skills_cli_install_targets")).toHaveLength(1);
    expect(useSkillsCliStore.getState().skills).toEqual(skills);
    expect(useSkillsCliStore.getState().targets).toEqual(targets);
    expect(useSkillsCliStore.getState().doctor).toEqual(doctor);
  });

  it("refuses empty add selections without invoking", async () => {
    const result = await useSkillsCliStore.getState().addGlobal({
      source: "owner/repo",
      skillNames: [],
      skillportAgentIds: ["cursor"],
    });
    expect(result).toBeNull();
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(0);
    expect(useSkillsCliStore.getState().error).toContain("skills_cli.selection_empty");
  });

  it("sends jobId and the changed selection payload on add", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: skills,
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

  it("surfaces doctor errors without leaking stderr", async () => {
    mockIpcCommand("skills_cli_doctor", () => {
      throw ipcFixtureError(
        "skills_cli.cli_unavailable",
        "The Skills CLI package could not be executed.",
      );
    });
    mockIpcCommand("skills_cli_list_global", skills);
    mockIpcCommand("skills_cli_install_targets", targets);

    await useSkillsCliStore.getState().loadAll();

    expect(useSkillsCliStore.getState().error).toContain("skills_cli.cli_unavailable");
    expect(useSkillsCliStore.getState().error).not.toContain("npm ERR!");
  });
});
