import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import defaultCapability from "../../src-tauri/capabilities/default.json";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { BatchInstallCentralSkillsDialog } from "../components/central/BatchInstallCentralSkillsDialog";
import { getPlatformTargetGroups } from "../lib/platformTargetGroups";
import { useTargetStore } from "../stores/targetStore";
import type { AgentWithStatus, CentralBatchInstallResult, TargetSummary } from "../types";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

const mockAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "kiro",
    display_name: "Kiro",
    category: "coding",
    global_skills_dir: "~/.kiro/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

const successInstallResult: CentralBatchInstallResult = {
  succeeded: [
    {
      skill_id: "frontend-design",
      agent_id: "codex",
      target_path: "D:\\work\\demo\\.agents\\skills\\frontend-design",
    },
  ],
  skipped: [],
  failed: [],
};

const mockOnInstall = vi.fn();
const mockOnOpenChange = vi.fn();

function renderDialog(props: { agents?: AgentWithStatus[] } = {}) {
  return render(
    <BatchInstallCentralSkillsDialog
      open
      onOpenChange={mockOnOpenChange}
      skillCount={2}
      agents={getPlatformTargetGroups(props.agents ?? mockAgents, {
        coding: true,
        lobster: true,
      })}
      isInstalling={false}
      onInstall={mockOnInstall}
    />
  );
}

describe("BatchInstallCentralSkillsDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(openDialog).mockResolvedValue(null);
    mockOnInstall.mockResolvedValue(successInstallResult);
    useTargetStore.setState({
      targets: [localTarget],
      activeTarget: localTarget,
    });
  });

  it("keeps Tauri dialog permission for project folder picking", () => {
    const capability = defaultCapability as { permissions: string[] };

    expect(capability.permissions).toContain("dialog:default");
  });

  it("fills project path from the folder picker and submits it", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("D:\\picked\\batch-project");
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    fireEvent.click(screen.getByRole("button", { name: "选择项目文件夹" }));

    const input = screen.getByPlaceholderText("D:\\Projects\\example 或 /Users/me/project");
    await waitFor(() => expect(input).toHaveValue("D:\\picked\\batch-project"));

    fireEvent.click(screen.getByRole("button", {
      name: /将 2 个技能安装到 .* 个平台/i,
    }));

    await waitFor(() =>
      expect(mockOnInstall).toHaveBeenCalledWith(
        expect.any(Array),
        "copy",
        "D:\\picked\\batch-project"
      )
    );
    expect(openDialog).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      defaultPath: undefined,
      canCreateDirectories: true,
    });
  });

  it("keeps manual project path when the folder picker is cancelled", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce(null);
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    const input = screen.getByPlaceholderText("D:\\Projects\\example 或 /Users/me/project");
    fireEvent.change(input, {
      target: { value: "D:\\manual\\batch-project" },
    });
    fireEvent.click(screen.getByRole("button", { name: "选择项目文件夹" }));

    await waitFor(() => expect(openDialog).toHaveBeenCalled());
    expect(input).toHaveValue("D:\\manual\\batch-project");
  });

  it("shows a folder picker error without submitting install", async () => {
    vi.mocked(openDialog).mockRejectedValueOnce(new Error("Dialog denied"));
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    fireEvent.click(screen.getByRole("button", { name: "选择项目文件夹" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "无法选择项目文件夹：Error: Dialog denied"
    );
    expect(mockOnInstall).not.toHaveBeenCalled();
  });

  it("closes without an error when every selected target is skipped", async () => {
    mockOnInstall.mockResolvedValueOnce({
      succeeded: [],
      skipped: [
        {
          skill_id: "frontend-design",
          agent_id: "codex",
          target_path: "D:\\work\\demo\\.agents\\skills\\frontend-design",
          reason: "already_installed",
        },
      ],
      failed: [],
    } satisfies CentralBatchInstallResult);
    renderDialog();

    fireEvent.click(screen.getByRole("button", {
      name: /将 2 个技能安装到 .* 个平台/i,
    }));

    await waitFor(() => {
      expect(mockOnOpenChange).toHaveBeenCalledWith(false);
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows skipped counts next to failures for mixed results", async () => {
    mockOnInstall.mockResolvedValueOnce({
      succeeded: [
        {
          skill_id: "frontend-design",
          agent_id: "codex",
          target_path: "D:\\work\\demo\\.agents\\skills\\frontend-design",
        },
      ],
      skipped: [
        {
          skill_id: "frontend-design",
          agent_id: "claude-code",
          target_path: "D:\\work\\demo\\.claude\\skills\\frontend-design",
          reason: "already_installed",
        },
      ],
      failed: [
        {
          skill_id: "frontend-design",
          agent_id: "kiro",
          error: "A real directory already exists",
        },
      ],
    } satisfies CentralBatchInstallResult);
    renderDialog();

    fireEvent.click(screen.getByRole("button", {
      name: /将 2 个技能安装到 .* 个平台/i,
    }));

    expect(await screen.findByText(/成功 1，跳过 1，失败 1/)).toBeInTheDocument();
    expect(screen.getByText(/frontend-design \/ kiro/)).toBeInTheDocument();
    expect(mockOnOpenChange).not.toHaveBeenCalledWith(false);
  });
});
