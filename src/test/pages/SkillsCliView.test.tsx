import { describe, expect, it, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { Sidebar } from "@/components/layout/Sidebar";
import { SkillsCliView } from "@/pages/SkillsCliView";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { SKILLS_CLI_GRID_CLASS } from "@/pages/skillsCliViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, mockIpcCommands } from "@/test/support/ipcMock";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

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
    showSkillsCliActionToast.mockClear();
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
    expect(screen.queryByLabelText("库存统计 KPI")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "导出全部" })).toBeDisabled();
    expect(screen.getByTestId("skills-cli-layout-bands")).toHaveAttribute(
      "data-grid",
      "twoColumns",
    );
    expect(screen.getByTestId("skills-cli-layout-bands")).toHaveAttribute(
      "data-drawer",
      "fullWidth",
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
    expect(screen.queryByLabelText("库存统计 KPI")).not.toBeInTheDocument();
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
    expect(screen.queryByTestId("skills-cli-census-empty")).not.toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
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

  it("locks the named content container and 2/3/4-column grid classes", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-content").className).toContain(
      "@container/skills-cli",
    );
    const grid = document.getElementById("skills-cli-group-panel-none:all")
      ?? document.querySelector("[class*='@min-[900px]/skills-cli:grid-cols-3']");
    expect(grid?.className).toContain("grid-cols-2");
    expect(grid?.className).toContain("@min-[900px]/skills-cli:grid-cols-3");
    expect(grid?.className).toContain("@min-[1180px]/skills-cli:grid-cols-4");
    expect(SKILLS_CLI_GRID_CLASS).not.toMatch(/\b(?:md|lg|xl):grid-cols-/);
  });

  it("shows skeletons while the first inventory load is in flight", async () => {
    let resolveList: ((value: typeof listGlobal) => void) | undefined;
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: () =>
        new Promise<typeof listGlobal>((resolve) => {
          resolveList = resolve;
        }),
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);
    const skeleton = await screen.findByTestId("skills-cli-skeleton");
    expect(skeleton).toHaveAttribute("aria-busy", "true");
    expect(skeleton.querySelectorAll("[aria-hidden]")).toHaveLength(12);
    resolveList?.(listGlobal);
    await screen.findByText("demo-skill");
  });

  it("shows the search query in the filtered empty state", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.change(screen.getByLabelText("搜索技能"), {
      target: { value: "no-such-skill" },
    });
    expect(
      screen.getByText("没有匹配“no-such-skill”的技能。"),
    ).toBeInTheDocument();
  });

  it("opens detail with null focus, links focus, and uninstall payload, then resets", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "detail",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "null",
    );
    fireEvent.click(screen.getByRole("button", { name: "管理 demo-skill 的链接" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "links",
    );
    fireEvent.click(screen.getByRole("button", { name: "卸载 demo-skill" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "uninstall",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-uninstall",
      "demo-skill",
    );
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "none",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "",
    );
  });

  it("clears selection on bubbling Escape only when no surface is open and the event was not prevented", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    fireEvent.click(screen.getByLabelText("选择技能"));
    expect(screen.getByLabelText("选择技能")).toBeChecked();
    const page = screen.getByTestId("skills-cli-page");
    const prevented = new KeyboardEvent("keydown", {
      key: "Escape",
      bubbles: true,
      cancelable: true,
    });
    prevented.preventDefault();
    page.dispatchEvent(prevented);
    expect(screen.getByLabelText("选择技能")).toBeChecked();
    fireEvent.keyDown(page, { key: "Escape" });
    expect(screen.queryByLabelText("选择技能")).not.toBeInTheDocument();
  });

  it("keeps mutation success when a follow-up inventory refresh fails", async () => {
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
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["demo-skill", "helper-skill"],
      },
      skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.change(screen.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览技能" }));
    await screen.findByLabelText("安装 demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "安装所选" }));
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith(
        expect.objectContaining({
          semantic: "success",
        }),
      );
    });
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith(
        expect.objectContaining({
          semantic: "error",
          message: expect.stringContaining("安装成功，但库存刷新失败"),
        }),
      );
    });
    expect(showSkillsCliActionToast).not.toHaveBeenCalledWith(
      expect.objectContaining({
        message: expect.stringMatching(/安装失败|addError/),
      }),
    );
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
  });

  it("does not treat a runtime-only refresh failure as an add failure", async () => {
    const doctorOnce = vi
      .fn()
      .mockReturnValueOnce(doctor)
      .mockImplementation(() => {
        throw ipcFixtureError(
          "skills_cli.cli_unavailable",
          "The Skills CLI package could not be executed.",
        );
      });
    mockIpcCommands({
      skills_cli_doctor: doctorOnce,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["demo-skill", "helper-skill"],
      },
      skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.change(screen.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览技能" }));
    await screen.findByLabelText("安装 demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "安装所选" }));
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith(
        expect.objectContaining({ semantic: "success" }),
      );
    });
    expect(showSkillsCliActionToast).not.toHaveBeenCalledWith(
      expect.objectContaining({
        message: expect.stringContaining("安装成功，但库存刷新失败"),
      }),
    );
    expect(
      await screen.findByText("无法执行 Skills CLI 软件包。"),
    ).toBeInTheDocument();
  });

  it("reports mutation failure without retrying add", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["demo-skill", "helper-skill"],
      },
      skills_cli_add_global: () => {
        throw ipcFixtureError(
          "skills_cli.cli_unavailable",
          "The Skills CLI package could not be executed.",
        );
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.change(screen.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览技能" }));
    await screen.findByLabelText("安装 demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "安装所选" }));
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith(
        expect.objectContaining({
          semantic: "error",
          message: "无法执行 Skills CLI 软件包。",
        }),
      );
    });
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
  });

  it("disables duplicate refresh while a refresh is in flight", async () => {
    let resolveList: ((value: typeof listGlobal) => void) | undefined;
    const list = vi
      .fn()
      .mockReturnValueOnce(listGlobal)
      .mockImplementation(
        () =>
          new Promise<typeof listGlobal>((resolve) => {
            resolveList = resolve;
          }),
      );
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: list,
      skills_cli_install_targets: targets,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    resolveList?.(listGlobal);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "刷新" })).toBeEnabled();
    });
  });
});
