import { describe, expect, it, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { Sidebar } from "@/components/layout/Sidebar";
import { SkillsCliView } from "@/pages/SkillsCliView";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, mockIpcCommands } from "@/test/support/ipcMock";

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn((selector?: (state: Record<string, unknown>) => unknown) => {
    const state = {
      agents: [],
      skillsByAgent: {},
      collectionCount: 0,
      categoryVisibility: { coding: true, lobster: true },
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
    };
    return typeof selector === "function" ? selector(state) : state;
  }),
}));

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

function mockHappyPath() {
  mockIpcCommands({
    skills_cli_doctor: doctor,
    skills_cli_list_global: listGlobal,
    skills_cli_install_targets: targets,
    skills_cli_preview_source: {
      source: "owner/repo",
      skills: ["demo-skill", "helper-skill"],
    },
    skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
    skills_cli_remove_global: undefined,
    cancel_skills_cli_job: true,
  });
}

describe("SkillsCliView", () => {
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
    useTargetStore.setState({
      activeTarget: {
        id: "local",
        kind: "local",
        label: "Local",
        isActive: true,
      },
    });
  });

  it("lists Skills CLI global skills", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    expect(await screen.findByText("demo-skill")).toBeInTheDocument();
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "skills@1.5.23",
    );
  });

  it("defaults preview skill and enabled platform checkboxes", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    fireEvent.change(screen.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览技能" }));

    const demo = await screen.findByLabelText("安装 demo-skill");
    const helper = screen.getByLabelText("安装 helper-skill");
    const cursor = screen.getByLabelText("安装到 Cursor");
    const amp = screen.getByLabelText("安装到 Amp");

    expect(demo).toBeChecked();
    expect(helper).toBeChecked();
    expect(cursor).toBeChecked();
    expect(amp).not.toBeChecked();
  });

  it("sends the changed selection payload on add", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    fireEvent.change(screen.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览技能" }));
    await screen.findByLabelText("安装 demo-skill");

    fireEvent.click(screen.getByLabelText("安装 demo-skill"));
    fireEvent.click(screen.getByLabelText("安装到 Amp"));
    fireEvent.click(screen.getByRole("button", { name: "安装所选" }));

    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
    });
    expect(ipcInvokeCalls("skills_cli_add_global")[0].args).toMatchObject({
      source: "owner/repo",
      skillNames: ["helper-skill"],
      skillportAgentIds: ["cursor", "amp"],
    });
  });

  it("confirms uninstall of canonical copy, platform links, and lock row", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "卸载 demo-skill" }));
    expect(
      screen.getByText(/规范副本、Skills CLI 创建的平台链接，以及 lock 中的对应行/),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "卸载" }));
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
    expect(ipcInvokeCalls("skills_cli_remove_global")[0].args).toMatchObject({
      skillName: "demo-skill",
    });
  });

  it("keeps the uninstall dialog open when remove fails", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_remove_global: () => {
        throw ipcFixtureError(
          "skills_cli.cli_unavailable",
          "The Skills CLI package could not be executed.",
        );
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "卸载 demo-skill" }));
    fireEvent.click(screen.getByRole("button", { name: "卸载" }));
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
    expect(
      screen.getByText(/规范副本、Skills CLI 创建的平台链接，以及 lock 中的对应行/),
    ).toBeInTheDocument();
  });

  it("shows npx missing as a localized doctor error", async () => {
    mockIpcCommands({
      skills_cli_doctor: () => {
        throw ipcFixtureError(
          "skills_cli.cli_unavailable",
          "The Skills CLI package could not be executed.",
        );
      },
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "无法执行 Skills CLI 软件包。",
    );
  });

  it("hides the sidebar entry unless ActiveTarget is Local", () => {
    useTargetStore.setState({
      activeTarget: {
        id: "ssh-1",
        kind: "ssh",
        label: "Remote",
        isActive: true,
      },
    });
    const { unmount } = render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>,
    );
    expect(screen.queryByRole("button", { name: "Skills CLI 全局" })).not.toBeInTheDocument();
    unmount();

    render(<SkillsCliView />);
    expect(
      screen.getByText("Skills CLI 全局管理仅可在本机 Local 目标上使用。"),
    ).toBeInTheDocument();
  });
});
