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
    installKind: "canonical",
    scope: "global",
    agents: ["Cursor"],
    source: "owner/repo",
    sourceUrl: "https://github.com/owner/repo",
    sourceType: "github",
    sourceTypeBucket: "github",
    canonicalPath: "/tmp/demo-skill",
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements: [],
  },
];
const listGlobal = {
  skills,
  canonicalRoot: "/tmp/agents",
  lockPath: "/tmp/agents/skills.lock",
};
const emptyList = {
  skills: [],
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
    skills_cli_remove_global: {
      removedCanonical: true,
      removedManagedAgentIds: [],
      retainedDirectCopyAgentIds: [],
    },
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
    useTargetStore.setState({
      activeTarget: {
        id: "local",
        kind: "local",
        label: "Local",
        isActive: true,
      },
    });
  });

  it("lists Skills CLI global skills with doctor status and paths", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    expect(await screen.findByText("demo-skill")).toBeInTheDocument();
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "skills@1.5.23",
    );
    expect(screen.getByTestId("skills-cli-paths")).toHaveTextContent(
      "/tmp/agents",
    );
    expect(screen.getByTestId("skills-cli-paths")).toHaveTextContent(
      "/tmp/agents/skills.lock",
    );
  });

  it("orders inventory before the collapsed install section", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    const inventory = screen.getByTestId("skills-cli-inventory");
    const install = screen.getByTestId("skills-cli-install");
    expect(
      inventory.compareDocumentPosition(install) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(install).not.toHaveAttribute("open");
    expect(document.querySelector("svg[role='img']")).not.toBeNull();
  });

  it("shows the empty state with an open install section on an empty lock", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: emptyList,
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);

    // The empty text exists before the first load settles; wait for the
    // post-load commit via the install section's open attribute.
    await waitFor(() => {
      expect(screen.getByTestId("skills-cli-install")).toHaveAttribute("open");
    });
    expect(screen.getByTestId("skills-cli-census-empty")).toBeInTheDocument();
    expect(screen.queryByText("demo-skill")).not.toBeInTheDocument();
    expect(
      screen.getByText("尚未安装 Skills CLI 全局技能。"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("skills-cli-inventory-error"),
    ).not.toBeInTheDocument();
  });

  it("keeps the inventory rendered once when doctor reports cli_unavailable", async () => {
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

    await screen.findByText("demo-skill");
    expect(
      screen.getAllByText("无法执行 Skills CLI 软件包。"),
    ).toHaveLength(1);
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "无法执行 Skills CLI 软件包。",
    );
    expect(screen.getByRole("button", { name: "安装所选" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "卸载 demo-skill" }),
    ).toBeDisabled();
  });

  it("keeps the stale list visible when a refresh fails", async () => {
    const failingList = vi
      .fn()
      .mockReturnValueOnce(listGlobal)
      .mockImplementation(() => {
        throw ipcFixtureError("internal.unexpected", "list failed");
      });
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: failingList,
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));

    expect(
      await screen.findByTestId("skills-cli-inventory-error"),
    ).toBeInTheDocument();
    expect(screen.getByText("demo-skill")).toBeInTheDocument();
  });

  it("shows the inventory error instead of the empty state on first-load failure", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: () => {
        throw ipcFixtureError("internal.unexpected", "list failed");
      },
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);

    expect(
      await screen.findByTestId("skills-cli-inventory-error"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("尚未安装 Skills CLI 全局技能。"),
    ).not.toBeInTheDocument();
    expect(screen.getByTestId("skills-cli-install")).not.toHaveAttribute("open");
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
