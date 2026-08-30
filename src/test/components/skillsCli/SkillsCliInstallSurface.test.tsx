import { createRef, useState } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { SkillsCliInstallSurface } from "@/components/skillsCli/SkillsCliInstallSurface";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillsCliRecentSourcesStore } from "@/stores/skillsCliRecentSourcesStore";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import type { SkillsCliGlobalSkill } from "@/types";
import { ipcInvokeCalls, ipcInvokedCommands, mockIpcCommands } from "@/test/support/ipcMock";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const ASYNC_UI_TIMEOUT_MS = 5_000;

const doctor = { nodeVersion: "v22.20.0", npmSpec: "skills" };
const targets = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: "cursor",
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
];
const listGlobal: {
  skills: SkillsCliGlobalSkill[];
  canonicalRoot: string;
  lockPath: string;
} = {
  skills: [
    {
      name: "demo-skill",
      path: "/tmp/demo-skill",
      installKind: "canonical",
      scope: "global",
      agents: ["Cursor"],
      source: "owner/repo",
      sourceUrl: null,
      sourceType: "github",
      sourceTypeBucket: "github",
      canonicalPath: "/tmp/demo-skill",
      folderHash: null,
      installedAt: null,
      updatedAt: null,
      placements: [
        {
          agentId: "cursor",
          displayName: "Cursor",
          targetPath: "/tmp/cursor/demo-skill",
          state: "managed_link",
          managedLinkKind: "windows_junction",
          reasonCode: null,
          installOrigin: null,
        },
      ],
    },
  ],
  canonicalRoot: "/tmp/agents",
  lockPath: "/tmp/agents/skills.lock",
};

function resetStores() {
  useSkillsCliStore.getState().resetForTargetChange();
  useSkillsCliRecentSourcesStore.getState().reset();
}

function renderSurface(open = true) {
  const onOpenChange = vi.fn();
  function Harness() {
    const [isOpen, setOpen] = useState(open);
    return (
      <SkillsCliInstallSurface
        open={isOpen}
        onOpenChange={(next) => {
          onOpenChange(next);
          setOpen(next);
        }}
        returnFocusRef={createRef<HTMLElement | null>()}
        contentWidthPx={720}
      />
    );
  }
  render(<Harness />);
  return { onOpenChange };
}

async function findWizard() {
  const dialog = await screen.findByRole(
    "dialog",
    { name: "安装技能" },
    { timeout: ASYNC_UI_TIMEOUT_MS },
  );
  return within(dialog);
}

async function walkToInstall(wizard: ReturnType<typeof within>) {
  fireEvent.change(wizard.getByLabelText("技能来源"), {
    target: { value: "owner/repo" },
  });
  fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
  fireEvent.click(
    await wizard.findByRole("button", { name: "继续" }, {
      timeout: ASYNC_UI_TIMEOUT_MS,
    }),
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
}

describe("SkillsCliInstallSurface", () => {
  beforeEach(() => {
    resetStores();
    showSkillsCliActionToast.mockClear();
    usePlatformStore.setState({ agents: [] });
    useSkillsCliStore.setState({
      doctor,
      targets,
      skills: listGlobal.skills,
      canonicalRoot: "/tmp/agents",
      runtimeError: null,
      inventoryError: null,
      actionError: null,
    });
  });

  it("opens the dialog from the mount seam and maps a null preview to a reviewed error", async () => {
    mockIpcCommands({
      get_setting: "[]",
      skills_cli_preview_source: () => {
        throw ipcFixtureError(
          "skills_cli.source_invalid",
          "The skill source is not allowed.",
        );
      },
    });
    renderSurface();
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "bad" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(
      await wizard.findByRole("alert", {}, { timeout: ASYNC_UI_TIMEOUT_MS }),
    ).toHaveTextContent("技能来源不被允许。");
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
  });

  it("closes and toasts success, then refreshes inventory and recent sources independently", async () => {
    mockIpcCommands({
      get_setting: "[]",
      set_setting: undefined,
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["helper-skill"],
      },
      skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
    });
    const { onOpenChange } = renderSurface();
    const wizard = await findWizard();
    await walkToInstall(wizard);
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "success",
      message: expect.stringContaining("1"),
    });
    await waitFor(() => {
      expect(ipcInvokeCalls("skills_cli_list_global").length).toBeGreaterThan(0);
      expect(ipcInvokeCalls("set_setting")).toEqual([
        {
          command: "set_setting",
          args: {
            key: "skills_cli.recent_sources",
            value: JSON.stringify(["owner/repo"]),
          },
        },
      ]);
    });
    expect(ipcInvokedCommands()).not.toContain("refresh_skill_update_inventory");
    expect(ipcInvokedCommands()).not.toContain("verify_skill_update_baseline");
  });

  it("reports refresh failure without reopening, recasting install, or resubmitting", async () => {
    mockIpcCommands({
      get_setting: "[]",
      set_setting: undefined,
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["helper-skill"],
      },
      skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
      skills_cli_doctor: doctor,
      skills_cli_list_global: () => {
        throw ipcFixtureError("internal.unexpected", "list failed");
      },
      skills_cli_install_targets: targets,
    });
    const { onOpenChange } = renderSurface();
    const wizard = await findWizard();
    await walkToInstall(wizard);
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith({
        semantic: "error",
        message: expect.stringContaining("安装成功，但库存刷新失败"),
      });
    });
    expect(showSkillsCliActionToast).not.toHaveBeenCalledWith(
      expect.objectContaining({
        message: expect.stringMatching(/安装失败/),
      }),
    );
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
    expect(screen.queryByRole("dialog", { name: "安装技能" })).not.toBeInTheDocument();
  });

  it("reports recent push failure as a warning without treating install as failed", async () => {
    mockIpcCommands({
      get_setting: "[]",
      set_setting: () => {
        throw ipcFixtureError(
          "setting_value_invalid",
          "The setting value is invalid.",
        );
      },
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["helper-skill"],
      },
      skills_cli_add_global: { installedSkills: 1, targetedPlatforms: 1 },
      skills_cli_doctor: doctor,
      skills_cli_list_global: listGlobal,
      skills_cli_install_targets: targets,
    });
    const { onOpenChange } = renderSurface();
    const wizard = await findWizard();
    await walkToInstall(wizard);
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
    await waitFor(() => {
      expect(showSkillsCliActionToast).toHaveBeenCalledWith({
        semantic: "error",
        message: expect.stringContaining("最近来源列表未能保存"),
      });
    });
    expect(ipcInvokeCalls("skills_cli_add_global")).toHaveLength(1);
    expect(
      showSkillsCliActionToast.mock.calls.some(
        (call) => call[0]?.semantic === "success",
      ),
    ).toBe(true);
  });

  it("fail-closes invalid recent JSON and still allows a manual preview", async () => {
    mockIpcCommands({
      get_setting: '{"source":"https://user:token@github.com/owner/repo"}',
      skills_cli_preview_source: {
        source: "owner/repo",
        skills: ["helper-skill"],
      },
    });
    renderSurface();
    const wizard = await findWizard();
    expect(
      await wizard.findByText("最近来源未能加载。你仍可粘贴来源并预览。", {
        exact: false,
      }, { timeout: ASYNC_UI_TIMEOUT_MS }),
    ).toBeInTheDocument();
    expect(wizard.queryByRole("button", { name: /预览 / })).not.toBeInTheDocument();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
    expect(ipcInvokeCalls("skills_cli_preview_source")).toHaveLength(1);
  });
});
