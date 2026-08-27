import { describe, expect, it, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { Sidebar } from "@/components/layout/Sidebar";
import { SkillsCliView } from "@/pages/SkillsCliView";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { SKILLS_CLI_GRID_CLASS } from "@/pages/skillsCliViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, ipcInvokedCommands, mockIpcCommands } from "@/test/support/ipcMock";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliUpdateInventory,
} from "@/types";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

const ASYNC_UI_TIMEOUT_MS = 5_000;

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const saveMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: (...args: unknown[]) => saveMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
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
const pendingUpdateInventory: SkillsCliUpdateInventory = {
  ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  lastSuccessAt: "2026-08-26T00:00:00.000Z",
  repositories: [
    {
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      branch: "main",
      observedRevisionSha: "bbbbbbb222222222222222222222222222222222",
      status: "ok",
      lastCheckedAt: "2026-08-26T00:00:00.000Z",
      lastErrorCode: null,
      rateLimitResetAt: null,
      pendingCount: 1,
    },
  ],
  skills: [
    {
      skillName: "demo-skill",
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      skillPath: "demo-skill",
      status: "update_available",
      installedRevisionSha: "aaaaaaa111111111111111111111111111111111",
      observedRevisionSha: "bbbbbbb222222222222222222222222222222222",
      pendingRevisionSha: "bbbbbbb222222222222222222222222222222222",
      installedLocalDigest: "sha256-v1:local",
      observedUpstreamDigest: "sha256-v1:up",
      pendingUpstreamDigest: "sha256-v1:up",
      isStale: false,
      lastErrorCode: null,
      changeSummary: ["SKILL.md"],
      blockers: [],
      argvPreview: [
        "refresh",
        "owned-canonical",
        "from-pinned-github-snapshot",
        "demo-skill",
      ],
    },
  ],
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
    get_setting: null,
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
    skills_cli_preview_remove_global: ({ skillName }: { skillName: string }) => ({
      skillName,
      ownedCanonical: true,
      managedPlacements: [{ agentId: "cursor", displayName: "Cursor" }],
      retainedDirectCopies: [],
      conflicts: [],
      confirmable: true,
    }),
    skills_cli_export_inventory: null,
    skills_cli_read_skill_md: ({ skillName }: { skillName: string }) => ({
      skillName,
      content: "---\nname: demo\n---\n# Demo",
      byteSize: 25,
    }),
    skills_cli_reveal_skill_folder: null,
    cancel_skills_cli_job: true,
    skills_cli_update_inventory: pendingUpdateInventory,
    skills_cli_check_updates: pendingUpdateInventory,
    skills_cli_apply_updates: {
      appliedSkillNames: ["demo-skill"],
      installedRevisionSha: "bbbbbbb222222222222222222222222222222222",
    },
    skills_cli_verify_update_baseline: pendingUpdateInventory,
    skills_cli_retry_update_recovery: {
      operationId: "op-1",
      phase: "db_committed",
    },
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
      docState: { status: "idle" },
      updateInventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      isLoadingUpdateCache: false,
      updateJob: { jobId: null, phase: null },
      updateError: null,
      updateProgress: null,
      batchProgress: null,
    });
    showSkillsCliActionToast.mockClear();
    saveMock.mockReset();
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
    expect(screen.getByRole("button", { name: "安装技能" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "导出全部" })).toBeEnabled();
    expect(screen.getByTestId("skills-cli-layout-bands")).toHaveAttribute(
      "data-grid",
      "twoColumns",
    );
    expect(screen.getByTestId("skills-cli-layout-bands")).toHaveAttribute(
      "data-drawer",
      "fullWidth",
    );
  });

  it("keeps a single header dialog entry and does not render a footer install details block", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    expect(screen.getByTestId("skills-cli-inventory")).toBeInTheDocument();
    expect(screen.queryByTestId("skills-cli-install")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "安装所选" })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("库存统计 KPI")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "安装技能" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "install",
    );
    expect(
      await screen.findByRole("dialog", { name: "安装技能" }),
    ).toBeInTheDocument();
  });

  it("shows the empty state without a leftover install details block on an empty lock", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: emptyList,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      get_setting: null,
    });
    render(<SkillsCliView />);

    await waitFor(() => {
      expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
        "skills@1.5.23",
      );
    });
    expect(screen.queryByTestId("skills-cli-census-empty")).not.toBeInTheDocument();
    expect(screen.queryByText("demo-skill")).not.toBeInTheDocument();
    expect(
      screen.getByText("尚未安装 Skills CLI 全局技能。"),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("skills-cli-install")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "安装技能" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "导出全部" })).toBeDisabled();
    expect(
      screen.queryByTestId("skills-cli-inventory-error"),
    ).not.toBeInTheDocument();
  });

  it("keeps inventory and non-install actions when doctor reports node_missing", async () => {
    mockIpcCommands({
      skills_cli_doctor: () => {
        throw ipcFixtureError(
          "skills_cli.node_missing",
          "Node.js 22.20 or later is required.",
        );
      },
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_preview_remove_global: ({ skillName }: { skillName: string }) => ({
        skillName,
        ownedCanonical: true,
        managedPlacements: [],
        retainedDirectCopies: [],
        conflicts: [],
        confirmable: true,
      }),
    });
    render(<SkillsCliView />);

    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "需要 Node.js 22.20 或更高版本。",
    );
    expect(
      screen.queryByText("无法执行 Skills CLI 软件包。"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
    expect(screen.queryByRole("dialog", { name: "安装技能" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导出全部" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "查看 demo-skill 的详情" })).toBeEnabled();
    const uninstall = screen.getByRole("button", { name: "卸载 demo-skill" });
    expect(uninstall).toBeEnabled();
    fireEvent.click(uninstall);
    expect(
      await screen.findByRole(
        "dialog",
        { name: /卸载 demo-skill/ },
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    ).toBeInTheDocument();
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
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
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
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    render(<SkillsCliView />);

    expect(
      await screen.findByTestId("skills-cli-inventory-error"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("尚未安装 Skills CLI 全局技能。"),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId("skills-cli-install")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "安装技能" })).toBeEnabled();
  });

  it("confirms uninstall of canonical copy, platform links, and lock row", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "卸载 demo-skill" }));
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(
      within(dialog).getByText(/规范副本、Skills CLI 创建的平台链接，以及 lock 中的对应行/),
    ).toBeInTheDocument();
    const confirm = within(dialog).getByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
    expect(ipcInvokeCalls("skills_cli_remove_global")[0].args).toMatchObject({
      skillName: "demo-skill",
    });
    expect(JSON.stringify(ipcInvokeCalls("skills_cli_remove_global")[0].args)).not.toContain(
      "--keep-links",
    );
    expect(showSkillsCliActionToast).toHaveBeenCalledWith(
      expect.objectContaining({ semantic: "destructiveSuccess" }),
    );
  });

  it("keeps the uninstall dialog open when remove fails", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_preview_remove_global: ({ skillName }: { skillName: string }) => ({
        skillName,
        ownedCanonical: true,
        managedPlacements: [],
        retainedDirectCopies: [],
        conflicts: [],
        confirmable: true,
      }),
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
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const confirm = within(dialog).getByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
    expect(
      within(dialog).getByText(/规范副本、Skills CLI 创建的平台链接，以及 lock 中的对应行/),
    ).toBeInTheDocument();
    expect(showSkillsCliActionToast).toHaveBeenCalledWith(
      expect.objectContaining({ semantic: "destructiveError" }),
    );
  });

  it("shows the sidebar entry and remote inventory on a non-Local target", async () => {
    const localOnly = "Skills CLI 仅可在本机 Local 目标上使用。";
    useTargetStore.setState({
      activeTarget: {
        id: "ssh-1",
        kind: "ssh",
        label: "Remote",
        isActive: true,
      },
    });
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_read_skill_md: ({ skillName }: { skillName: string }) => ({
        skillName,
        content: "---\nname: demo\n---\n# Demo",
        byteSize: 25,
      }),
    });
    const { unmount } = render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>,
    );
    expect(screen.getByRole("button", { name: "Skills CLI 全局" })).toBeInTheDocument();
    unmount();

    render(<SkillsCliView />);
    expect(await screen.findByText("demo-skill")).toBeInTheDocument();
    expect(screen.getByTestId("skills-cli-inventory")).toBeInTheDocument();
    expect(
      screen.queryByText("Skills CLI 全局管理仅可在本机 Local 目标上使用。"),
    ).not.toBeInTheDocument();
    const install = screen.getByRole("button", { name: "安装技能" });
    expect(install).toBeEnabled();
    expect(install).not.toHaveAttribute("title", localOnly);
    expect(screen.getByTestId("skills-cli-check-updates")).toBeEnabled();
    expect(screen.getByTestId("skills-cli-check-updates")).not.toHaveAttribute(
      "title",
      localOnly,
    );
    expect(screen.getByRole("button", { name: "导出全部" })).toBeEnabled();
    const uninstall = screen.getByTestId("uninstall-skills-cli-demo-skill");
    expect(uninstall).toBeEnabled();
    expect(uninstall).not.toHaveAttribute("title", localOnly);
    expect(screen.queryByTestId("skills-cli-update-cache-error")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const reveal = within(drawer).getByTestId("skills-cli-detail-reveal");
    expect(reveal).toBeDisabled();
    expect(reveal).toHaveAttribute("title", localOnly);
    expect(ipcInvokeCalls("skills_cli_reveal_skill_folder")).toHaveLength(0);
  });

  it("keeps a stale inventory when remote list rejects remote_unavailable", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");

    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: () => {
        throw ipcFixtureError(
          "skills_cli.remote_unavailable",
          "The remote Skills CLI host is unavailable.",
          true,
        );
      },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));

    const alert = await screen.findByTestId("skills-cli-inventory-error");
    expect(alert).toHaveTextContent("无法连接到远端 Skills CLI 主机。");
    expect(screen.getByText("demo-skill")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
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
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
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
    const scrollIntoView = vi.spyOn(Element.prototype, "scrollIntoView");
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "detail",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "null",
    );
    expect(within(drawer).getByTestId("skills-cli-detail-update")).toBeEnabled();
    fireEvent.click(within(drawer).getByTestId("skills-cli-detail-close"));
    await waitFor(
      () => {
        expect(screen.queryByTestId("skills-cli-detail-drawer")).not.toBeInTheDocument();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    scrollIntoView.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "管理 demo-skill 的链接" }));
    const linksDrawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(
      () => {
        expect(scrollIntoView).toHaveBeenCalled();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(
      () => {
        expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
          "data-focus",
          "null",
        );
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );

    fireEvent.click(within(linksDrawer).getByTestId("skills-cli-detail-uninstall"));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "uninstall",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-uninstall",
      "demo-skill",
    );
    await waitFor(
      () => {
        expect(screen.queryByTestId("skills-cli-detail-drawer")).not.toBeInTheDocument();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const uninstall = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
    fireEvent.click(within(uninstall).getByRole("button", { name: "取消" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "none",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "",
    );
    scrollIntoView.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-focus",
      "null",
    );
    expect(scrollIntoView).not.toHaveBeenCalled();
    scrollIntoView.mockRestore();
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
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
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

  it("toggles selection from the card, merges Select all, and shows the batch bar", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    expect(screen.getByLabelText("选择技能")).toBeChecked();
    expect(screen.getByTestId("skills-cli-batch-bar")).toHaveTextContent("已选 1 项");
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-demo-skill"));
    expect(screen.getByLabelText("选择技能")).not.toBeChecked();
    expect(screen.queryByTestId("skills-cli-batch-bar")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-demo-skill"));
    expect(screen.getByLabelText("选择技能")).toBeChecked();
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "none",
    );
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    expect(screen.queryByLabelText("选择技能")).not.toBeInTheDocument();
  });

  it("does not clear selection when Escape originates from a text input", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    fireEvent.click(screen.getByLabelText("选择技能"));
    const search = screen.getByLabelText("搜索技能");
    search.focus();
    fireEvent.keyDown(search, { key: "Escape" });
    expect(screen.getByLabelText("选择技能")).toBeChecked();
  });

  it("exports the unfiltered inventory and stays silent when the save dialog is cancelled", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.change(screen.getByLabelText("搜索技能"), {
      target: { value: "no-such-skill" },
    });
    saveMock.mockResolvedValueOnce(null);
    fireEvent.click(screen.getByRole("button", { name: "导出全部" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalled());
    expect(ipcInvokeCalls("skills_cli_export_inventory")).toHaveLength(0);
    expect(showSkillsCliActionToast).not.toHaveBeenCalled();

    saveMock.mockResolvedValueOnce("D:/tmp/all.json");
    fireEvent.click(screen.getByRole("button", { name: "导出全部" }));
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_export_inventory")).toHaveLength(1);
    });
    const payload = ipcInvokeCalls("skills_cli_export_inventory")[0]?.args as {
      json: string;
    };
    expect(JSON.parse(payload.json).scope).toBe("all");
    expect(JSON.parse(payload.json).skillCount).toBe(1);
    expect(JSON.parse(payload.json).skills[0].name).toBe("demo-skill");
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "success",
      message: "已导出完整 Skills CLI 库存。",
    });
  });

  it("links only missing placements and toasts ordinary success semantic", async () => {
    const missingSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "missing" as const,
          managedLinkKind: null,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [missingSkill] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_link_platform: {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/tmp/cursor/skills/demo-skill",
        state: "managed_link",
        managedLinkKind: "windows_junction",
        reasonCode: null,
        installOrigin: null,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    fireEvent.click(screen.getByTestId("skills-cli-batch-link"));
    fireEvent.click(
      await screen.findByTestId(
        "skills-cli-batch-link-cursor",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_link_platform")[0]?.args).toMatchObject({
      skillName: "demo-skill",
      skillportAgentId: "cursor",
    });
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "success",
      message: "已链接 1 处。",
    });
  });

  it("unlinks only managed links with ordinary toast semantic", async () => {
    const linkedSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "managed_link" as const,
          managedLinkKind: "windows_junction" as const,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [linkedSkill] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_unlink_platform: {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/tmp/cursor/skills/demo-skill",
        state: "missing",
        managedLinkKind: null,
        reasonCode: null,
        installOrigin: null,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink"));
    fireEvent.click(
      await screen.findByTestId(
        "skills-cli-batch-unlink-all",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "success",
      message: "已取消链接 1 处。",
    });
  });

  it("exports the current selection in store order", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    saveMock.mockResolvedValueOnce("D:/tmp/selected.json");
    fireEvent.click(screen.getByRole("button", { name: "导出所选" }));
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_export_inventory")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const payload = ipcInvokeCalls("skills_cli_export_inventory")[0]?.args as {
      json: string;
    };
    expect(JSON.parse(payload.json).scope).toBe("selected");
    expect(JSON.parse(payload.json).skillCount).toBe(1);
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "success",
      message: "已导出选定技能。",
    });
  });

  it("reveals the owned folder by skill name and keeps the card/drawer on one snapshot", async () => {
    const missingSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "missing" as const,
          managedLinkKind: null,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    const linkedPlacement = {
      agentId: "cursor",
      displayName: "Cursor",
      targetPath: "/tmp/cursor/skills/demo-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
      installOrigin: null,
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [missingSkill] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_read_skill_md: {
        skillName: "demo-skill",
        content: "---\nname: demo\n---\n",
        byteSize: 18,
      },
      skills_cli_reveal_skill_folder: null,
      skills_cli_link_platform: linkedPlacement,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(within(drawer).getByTestId("skills-cli-detail-reveal"));
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_reveal_skill_folder")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_reveal_skill_folder")[0]?.args).toEqual({
      skillName: "demo-skill",
    });
    expect(
      JSON.stringify(ipcInvokeCalls("skills_cli_reveal_skill_folder")[0]?.args),
    ).not.toContain("/tmp/demo-skill");

    fireEvent.click(
      within(drawer).getByRole("switch", { name: "将 demo-skill 链接到 Cursor" }),
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_link_platform")[0]?.args).toMatchObject({
      skillName: "demo-skill",
      skillportAgentId: "cursor",
    });
  });

  it("links only missing placements from the drawer and never mutates a direct copy", async () => {
    const mixedSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "missing" as const,
          managedLinkKind: null,
          reasonCode: null,
          installOrigin: null,
        },
        {
          agentId: "amp",
          displayName: "Amp",
          targetPath: "/tmp/amp/skills/demo-skill",
          state: "direct_copy" as const,
          managedLinkKind: null,
          reasonCode: "skills_cli.direct_copy_not_toggleable",
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [mixedSkill] },
      skills_cli_install_targets: [
        { ...targets[0], isEnabled: true },
        { ...targets[1], isEnabled: true },
      ],
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_read_skill_md: {
        skillName: "demo-skill",
        content: "---\nname: demo\n---\n",
        byteSize: 18,
      },
      skills_cli_link_platform: {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/tmp/cursor/skills/demo-skill",
        state: "managed_link",
        managedLinkKind: "windows_junction",
        reasonCode: null,
        installOrigin: null,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(within(drawer).getByRole("switch", { name: /Amp/ }));
    expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(0);
    expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(0);
    fireEvent.click(within(drawer).getByTestId("skills-cli-detail-link-all"));
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_link_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_link_platform")[0]?.args).toMatchObject({
      skillName: "demo-skill",
      skillportAgentId: "cursor",
    });
    expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(0);
  });

  it("keeps Check updates enabled when doctor reports node_missing", async () => {
    mockIpcCommands({
      skills_cli_doctor: () => {
        throw ipcFixtureError(
          "skills_cli.node_missing",
          "Node.js 22.20 or later is required.",
        );
      },
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-check-updates")).toBeEnabled();
    expect(screen.getByRole("button", { name: "刷新" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
  });

  it("disables Install for timeout and unexpected doctor failures without clearing inventory", async () => {
    mockIpcCommands({
      skills_cli_doctor: () => {
        throw ipcFixtureError(
          "skills_cli.timeout",
          "The Skills CLI command timed out.",
        );
      },
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    const { unmount } = render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "Skills CLI 命令超时。",
    );
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "刷新" })).toBeEnabled();
    unmount();

    mockIpcCommands({
      skills_cli_doctor: () => {
        throw ipcFixtureError(
          "internal.unexpected",
          "The operation failed. See runtime logs for details.",
        );
      },
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "The operation failed. See runtime logs for details.",
    );
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "刷新" })).toBeEnabled();
  });

  it("keeps uninstall and batch mutations usable when doctor succeeds", async () => {
    const managedSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "managed_link" as const,
          managedLinkKind: "windows_junction" as const,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [managedSkill] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_preview_remove_global: ({ skillName }: { skillName: string }) => ({
        skillName,
        ownedCanonical: true,
        managedPlacements: [{ agentId: "cursor", displayName: "Cursor" }],
        retainedDirectCopies: [],
        conflicts: [],
        confirmable: true,
      }),
      skills_cli_remove_global: {
        removedCanonical: true,
        removedManagedAgentIds: ["cursor"],
        retainedDirectCopyAgentIds: [],
      },
      skills_cli_unlink_platform: {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/tmp/cursor/skills/demo-skill",
        state: "missing",
        managedLinkKind: null,
        reasonCode: null,
        installOrigin: null,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByRole("button", { name: "卸载 demo-skill" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    fireEvent.click(screen.getByLabelText("选择技能"));
    const batch = await screen.findByTestId("skills-cli-batch-bar");
    expect(within(batch).getByTestId("skills-cli-batch-unlink")).toBeEnabled();
    expect(within(batch).getByRole("button", { name: "卸载" })).toBeEnabled();
    fireEvent.click(within(batch).getByTestId("skills-cli-batch-unlink"));
    fireEvent.click(
      await screen.findByTestId(
        "skills-cli-batch-unlink-all",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(within(batch).getByRole("button", { name: "卸载" }));
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const confirm = within(dialog).getByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
  });

  it("toasts add cli_unavailable without changing the doctor status line", async () => {
    mockHappyPath();
    mockIpcCommands({
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["helper-skill"],
      },
      skills_cli_add_global: () => {
        throw ipcFixtureError(
          "skills_cli.cli_unavailable",
          "The Skills CLI package could not be executed.",
        );
      },
      get_setting: "[]",
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "安装技能" }));
    const wizard = within(
      await screen.findByRole(
        "dialog",
        { name: "安装技能" },
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    fireEvent.click(
      await wizard.findByRole(
        "button",
        { name: "继续" },
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    const install = await wizard.findByRole(
      "button",
      { name: "安装" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
    fireEvent.click(install);
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
    });
    expect(showSkillsCliActionToast).toHaveBeenCalledWith(
      expect.objectContaining({
        semantic: "error",
        message: expect.stringContaining("无法执行 Skills CLI 软件包。"),
      }),
    );
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "skills@1.5.23",
    );
    expect(screen.getByTestId("skills-cli-doctor")).not.toHaveTextContent(
      "无法执行 Skills CLI 软件包。",
    );
    fireEvent.click(wizard.getByRole("button", { name: "关闭" }));
    await waitFor(
      () => {
        expect(screen.queryByRole("dialog", { name: "安装技能" })).not.toBeInTheDocument();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const uninstall = screen.getByRole("button", { name: "卸载 demo-skill" });
    expect(uninstall).toBeEnabled();
    fireEvent.click(uninstall);
    const uninstallDialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const confirm = within(uninstallDialog).getByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
    });
  });

  it("opens the update drawer from Update all and from detail Update with the current skill", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全部更新" }));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "update",
    );
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-update-repo",
      "owner/repo@main",
    );
    const drawer = await screen.findByTestId("skills-cli-update-drawer");
    expect(drawer).toHaveAttribute("data-width-mode", "fullWidth");
    expect(screen.getByTestId("skills-cli-update-argv").textContent).not.toContain(
      "--force",
    );
    expect(screen.getByTestId("skills-cli-update-argv").textContent).not.toContain(
      "--keep-links",
    );
    fireEvent.click(screen.getByTestId("skills-cli-update-close"));
    await waitFor(() => {
      expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
        "data-kind",
        "none",
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "查看 demo-skill 的详情" }));
    const detail = await screen.findByRole("dialog", { name: "demo-skill" });
    fireEvent.click(within(detail).getByTestId("skills-cli-detail-update"));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "update",
    );
    expect(await screen.findByTestId("skills-cli-update-row-demo-skill")).toBeInTheDocument();
  });

  it("listens then invokes check updates from the header", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByTestId("skills-cli-check-updates"));
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_check_updates")).toHaveLength(1);
    });
    const args = ipcInvokeCalls("skills_cli_check_updates")[0]?.args as {
      jobId: string;
    };
    expect(args.jobId).toMatch(/\S/);
  });

  it("disables cleanup when no Unavailable skills exist and does not send IPC", async () => {
    mockHappyPath();
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    expect(screen.getByTestId("skills-cli-cleanup")).toBeDisabled();
    fireEvent.click(screen.getByTestId("skills-cli-cleanup"));
    expect(screen.getByTestId("skills-cli-active-surface")).toHaveAttribute(
      "data-kind",
      "none",
    );
    expect(ipcInvokeCalls("skills_cli_preview_remove_global")).toHaveLength(0);
    expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(0);
  });

  it("opens cleanup then reuses preview and removeGlobalBatch for stale entries", async () => {
    const ghost = {
      ...skills[0],
      name: "ghost",
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/ghost",
          state: "unavailable" as const,
          managedLinkKind: null,
          reasonCode: "canonical_missing",
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [ghost] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_preview_remove_global: ({ skillName }: { skillName: string }) => ({
        skillName,
        ownedCanonical: false,
        managedPlacements: [],
        retainedDirectCopies: [],
        conflicts: [],
        confirmable: true,
      }),
      skills_cli_remove_global: {
        removedCanonical: false,
        removedManagedAgentIds: [],
        retainedDirectCopyAgentIds: [],
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("ghost");
    fireEvent.click(screen.getByTestId("skills-cli-cleanup"));
    const cleanup = await screen.findByRole(
      "dialog",
      { name: /清理 Unavailable/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_preview_remove_global")).toHaveLength(0);
    fireEvent.click(within(cleanup).getByRole("button", { name: "查看卸载影响" }));
    const uninstall = await screen.findByRole(
      "dialog",
      { name: /卸载 ghost/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_preview_remove_global")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const confirm = within(uninstall).getByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokedCommands().filter((name) => name.includes("remove"))).toEqual([
      "skills_cli_preview_remove_global",
      "skills_cli_remove_global",
    ]);
  });

  it("blocks uninstall confirm when cleanup preview reports a conflict", async () => {
    const ghost = {
      ...skills[0],
      name: "ghost",
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/ghost",
          state: "unavailable" as const,
          managedLinkKind: null,
          reasonCode: "canonical_missing",
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [ghost] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_preview_remove_global: {
        skillName: "ghost",
        ownedCanonical: true,
        managedPlacements: [],
        retainedDirectCopies: [
          { agentId: "amp", displayName: "Amp" },
        ],
        conflicts: [
          {
            agentId: "codex",
            displayName: "Codex",
            reasonCode: "skills_cli.placement_conflict",
          },
        ],
        confirmable: false,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("ghost");
    fireEvent.click(screen.getByTestId("skills-cli-cleanup"));
    const cleanup = await screen.findByRole(
      "dialog",
      { name: /清理 Unavailable/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(within(cleanup).getByRole("button", { name: "查看卸载影响" }));
    const uninstall = await screen.findByRole(
      "dialog",
      { name: /卸载 ghost/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(
      () => {
        expect(
          within(uninstall).getByTestId("skills-cli-uninstall-conflicts"),
        ).toBeInTheDocument();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(within(uninstall).getByRole("button", { name: "卸载" })).toBeDisabled();
    expect(ipcInvokeCalls("skills_cli_remove_global")).toHaveLength(0);
    expect(within(uninstall).getByTestId("skills-cli-uninstall-retained")).toBeInTheDocument();
  });

  it("toasts checkFirst and does not apply when updates have not been checked", async () => {
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    fireEvent.click(screen.getByTestId("skills-cli-batch-update"));
    expect(ipcInvokeCalls("skills_cli_apply_updates")).toHaveLength(0);
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "error",
      message: "请先检查更新，再应用。",
    });
  });

  it("applies selected updates serially across repositories and ignores a second click", async () => {
    const alpha = {
      ...skills[0],
      name: "alpha",
      source: "owner/alpha",
    };
    const beta = {
      ...skills[0],
      name: "beta",
      source: "owner/beta",
    };
    const inventory: SkillsCliUpdateInventory = {
      ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      lastSuccessAt: "2026-08-26T00:00:00.000Z",
      skills: [
        {
          ...pendingUpdateInventory.skills[0],
          skillName: "alpha",
          repositoryKey: "owner/alpha@main",
          skillPath: "alpha",
        },
        {
          ...pendingUpdateInventory.skills[0],
          skillName: "beta",
          repositoryKey: "owner/beta@main",
          skillPath: "beta",
        },
      ],
    };
    let releaseFirst: (() => void) | undefined;
    let releaseSecond: (() => void) | undefined;
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [alpha, beta] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: inventory,
      skills_cli_apply_updates: ({
        request,
      }: {
        request: { repositoryKey: string };
      }) => {
        if (request.repositoryKey === "owner/alpha@main") {
          return new Promise((resolve) => {
            releaseFirst = () =>
              resolve({
                appliedSkillNames: ["alpha"],
                installedRevisionSha: "aaa",
              });
          });
        }
        return new Promise((resolve) => {
          releaseSecond = () =>
            resolve({
              appliedSkillNames: ["beta"],
              installedRevisionSha: "bbb",
            });
        });
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("alpha");
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-alpha"));
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-beta"));
    fireEvent.click(screen.getByTestId("skills-cli-batch-update"));
    expect(await screen.findByTestId("skills-cli-batch-progress")).toHaveTextContent(
      "0 / 2",
    );
    expect(screen.getByTestId("skills-cli-batch-update")).toBeDisabled();
    fireEvent.click(screen.getByTestId("skills-cli-batch-update"));
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_apply_updates")).toHaveLength(1);
    });
    releaseFirst?.();
    expect(
      await screen.findByTestId(
        "skills-cli-batch-progress",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    ).toHaveTextContent("1 / 2");
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_apply_updates")).toHaveLength(2);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const jobIds = ipcInvokeCalls("skills_cli_apply_updates").map((call) => {
      const args = call.args as { request: { jobId: string } };
      return args.request.jobId;
    });
    expect(jobIds[0]).not.toEqual(jobIds[1]);
    fireEvent.click(screen.getByTestId("skills-cli-batch-update"));
    expect(ipcInvokeCalls("skills_cli_apply_updates")).toHaveLength(2);
    releaseSecond?.();
    await waitFor(() => {
      expect(screen.queryByTestId("skills-cli-batch-progress")).not.toBeInTheDocument();
    });
  });

  it("skips non-managed placements when unlinking one platform", async () => {
    const linked = {
      ...skills[0],
      name: "linked-skill",
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/linked-skill",
          state: "managed_link" as const,
          managedLinkKind: "windows_junction" as const,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    const copy = {
      ...skills[0],
      name: "copy-skill",
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/copy-skill",
          state: "direct_copy" as const,
          managedLinkKind: null,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [linked, copy] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_unlink_platform: {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/tmp/cursor/skills/linked-skill",
        state: "missing",
        managedLinkKind: null,
        reasonCode: null,
        installOrigin: null,
      },
    });
    render(<SkillsCliView />);
    await screen.findByText("linked-skill");
    fireEvent.click(screen.getByRole("button", { name: "选择" }));
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-linked-skill"));
    fireEvent.click(screen.getByTestId("skills-cli-dense-card-copy-skill"));
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink"));
    fireEvent.click(
      await screen.findByTestId(
        "skills-cli-batch-unlink-cursor",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    await waitFor(
      () => {
        expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(ipcInvokeCalls("skills_cli_unlink_platform")[0]?.args).toMatchObject({
      skillName: "linked-skill",
      skillportAgentId: "cursor",
    });
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "error",
      message: expect.stringContaining("直接副本不能链接或取消链接。"),
    });
  });

  it("disables per-platform unlink while a batch is in flight and does not start a second batch", async () => {
    const linkedSkill = {
      ...skills[0],
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/skills/demo-skill",
          state: "managed_link" as const,
          managedLinkKind: "windows_junction" as const,
          reasonCode: null,
          installOrigin: null,
        },
      ],
    };
    let releaseUnlink: (() => void) | undefined;
    mockIpcCommands({
      skills_cli_doctor: doctor,
      skills_cli_list_global: { ...listGlobal, skills: [linkedSkill] },
      skills_cli_install_targets: targets,
      skills_cli_update_inventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills_cli_unlink_platform: () =>
        new Promise((resolve) => {
          releaseUnlink = () =>
            resolve({
              agentId: "cursor",
              displayName: "Cursor",
              targetPath: "/tmp/cursor/skills/demo-skill",
              state: "missing",
              managedLinkKind: null,
              reasonCode: null,
              installOrigin: null,
            });
        }),
    });
    render(<SkillsCliView />);
    await screen.findByText("demo-skill");
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink"));
    fireEvent.click(
      await screen.findByTestId(
        "skills-cli-batch-unlink-cursor",
        {},
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    expect(await screen.findByTestId("skills-cli-batch-progress")).toHaveTextContent(
      "0 / 1",
    );
    expect(screen.getByTestId("skills-cli-batch-unlink")).toBeDisabled();
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink"));
    expect(ipcInvokeCalls("skills_cli_unlink_platform")).toHaveLength(1);
    releaseUnlink?.();
    await waitFor(() => {
      expect(screen.queryByTestId("skills-cli-batch-progress")).not.toBeInTheDocument();
    });
  });
});
