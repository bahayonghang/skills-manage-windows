import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { InstallDialog } from "../components/central/InstallDialog";
import { AgentWithStatus, SkillWithLinks, TargetSummary } from "../types";
import { getPlatformTargetGroups } from "../lib/platformTargetGroups";
import { useTargetStore } from "../stores/targetStore";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

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
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "gemini-cli",
    display_name: "Gemini CLI",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    is_detected: false,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "kiro",
    display_name: "Kiro",
    category: "coding",
    global_skills_dir: "~/.kiro/skills/",
    is_detected: false,
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
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockSkill: SkillWithLinks = {
  id: "frontend-design",
  name: "frontend-design",
  description: "Build distinctive, production-grade frontend interfaces",
  file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
  canonical_path: "~/.skillsmanage/skills/frontend-design",
  is_central: true,
  scanned_at: "2026-04-09T00:00:00Z",
  linked_agents: ["claude-code", "codex"],
  shared_root_agents: [],
};

const mockOnInstall = vi.fn();
const mockOnOpenChange = vi.fn();
const successInstallResult = {
  succeeded: ["claude-code", "codex", "kiro"],
  skipped: [],
  failed: [],
};

const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

function renderDialog(props: {
  open?: boolean;
  skill?: SkillWithLinks | null;
  agents?: AgentWithStatus[];
} = {}) {
  const targetAgents = getPlatformTargetGroups(props.agents ?? mockAgents, {
    coding: true,
    lobster: true,
  });

  return render(
    <InstallDialog
      open={props.open ?? true}
      onOpenChange={mockOnOpenChange}
      skill={props.skill ?? mockSkill}
      agents={targetAgents}
      onInstall={mockOnInstall}
    />
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("InstallDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(openDialog).mockResolvedValue(null);
    mockOnInstall.mockResolvedValue(successInstallResult);
    useTargetStore.setState({
      targets: [localTarget],
      activeTarget: localTarget,
    });
  });

  // ── Rendering ─────────────────────────────────────────────────────────────

  it("renders dialog when open=true", () => {
    renderDialog();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("does not render dialog when open=false", () => {
    renderDialog({ open: false });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows skill name in title", () => {
    renderDialog();
    expect(screen.getByText("安装 frontend-design")).toBeInTheDocument();
  });

  it("shows non-central agent checkboxes", () => {
    renderDialog();
    expect(screen.getByLabelText("Claude Code")).toBeInTheDocument();
    expect(screen.getByLabelText("Universal (.agents/skills)")).toBeInTheDocument();
    expect(screen.getByLabelText("Kiro")).toBeInTheDocument();
    expect(screen.queryByLabelText("Cursor")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Gemini CLI")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Codex")).not.toBeInTheDocument();
  });

  it("does not show 'central' agent checkbox", () => {
    renderDialog();
    expect(screen.queryByLabelText("Central Skills")).not.toBeInTheDocument();
  });

  it("shows a compact linked status icon for linked agents", () => {
    renderDialog();
    // Claude Code is in linked_agents
    expect(screen.queryByText("已链接")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Claude Code 已链接")).toBeInTheDocument();
  });

  it("shows project target mode for Central skills", () => {
    renderDialog();

    expect(screen.getByText("目标")).toBeInTheDocument();
    expect(screen.getByText("全局平台目录")).toBeInTheDocument();
    expect(screen.getByText("项目目录")).toBeInTheDocument();
  });

  it("does not show project target mode for non-Central skills", () => {
    renderDialog({
      skill: {
        ...mockSkill,
        is_central: false,
        linked_agents: [],
      },
    });

    expect(screen.queryByText("目标")).not.toBeInTheDocument();
    expect(screen.queryByText("项目目录")).not.toBeInTheDocument();
  });

  it("shows 'not detected' badge for undetected agents", () => {
    renderDialog();
    // Kiro has is_detected: false
    expect(screen.getByText("(未检测到)")).toBeInTheDocument();
  });

  it("shows Universal as a normal selectable platform target", () => {
    renderDialog();

    expect(screen.getByLabelText("Universal (.agents/skills)")).not.toHaveAttribute("aria-disabled", "true");
    expect(screen.queryByText("始终包含")).not.toBeInTheDocument();
  });

  it("shows symlink/copy radio options", () => {
    renderDialog();
    // The radio items are rendered
    expect(screen.getByText("符号链接")).toBeInTheDocument();
    expect(screen.getByText("复制安装")).toBeInTheDocument();
  });

  // ── Confirm ───────────────────────────────────────────────────────────────

  it("shows confirm button with count of selected platforms", () => {
    renderDialog();
    // By default, all enabled and visible targets are pre-selected.
    expect(
      screen.getByRole("button", { name: /安装到 3 个平台/i })
    ).toBeInTheDocument();
  });

  it("calls onInstall with selected agent IDs on confirm", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        expect.any(String),
        null
      );
    });
  });

  it("submits selected platform targets on confirm", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalled();
    });
    const [, agentIds, method] = mockOnInstall.mock.calls[0];
    expect([...agentIds].sort()).toEqual(["claude-code", "codex", "kiro"]);
    expect(method).toBe("symlink");
  });

  it("does not submit shared-root targets", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog({
      skill: {
        ...mockSkill,
        shared_root_agents: ["codex", "cursor", "gemini-cli"],
      },
    });
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 2 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalled();
    });
    const [, agentIds] = mockOnInstall.mock.calls[0];
    expect([...agentIds].sort()).toEqual(["claude-code", "kiro"]);
  });

  it("passes 'symlink' method to onInstall by default", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "symlink",
        null
      );
    });
  });

  it("passes 'copy' method to onInstall when copy is selected", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog();

    // Select the Copy radio button
    const copyRadio = screen.getByText("复制安装").closest("label");
    expect(copyRadio).not.toBeNull();
    fireEvent.click(copyRadio!);

    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "copy",
        null
      );
    });
  });

  it("requires a project path when project target mode is selected", async () => {
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    fireEvent.click(screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    }));

    expect(await screen.findByRole("alert")).toHaveTextContent("请输入项目路径");
    expect(mockOnInstall).not.toHaveBeenCalled();
  });

  it("disables platforms without a project pattern in project target mode", () => {
    renderDialog({
      agents: [
        ...mockAgents,
        {
          id: "custom-tool",
          display_name: "Custom Tool",
          category: "coding",
          global_skills_dir: "D:\\Tools\\skills",
          is_detected: true,
          is_builtin: false,
          is_enabled: true,
        },
      ],
    });

    fireEvent.click(screen.getByText("项目目录").closest("label")!);

    expect(screen.getByLabelText("Custom Tool")).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByText("无项目模式")).toBeInTheDocument();
  });

  it("passes project path to onInstall in project target mode", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    fireEvent.change(screen.getByPlaceholderText("D:\\Projects\\example 或 /Users/me/project"), {
      target: { value: "D:\\work\\demo" },
    });
    fireEvent.click(screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    }));

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "copy",
        "D:\\work\\demo"
      );
    });
  });

  it("fills project path from the folder picker and submits it", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("D:\\picked\\project");
    mockOnInstall.mockResolvedValueOnce(successInstallResult);
    renderDialog();

    fireEvent.click(screen.getByText("项目目录").closest("label")!);
    fireEvent.click(screen.getByRole("button", { name: "选择项目文件夹" }));
    await waitFor(() =>
      expect(screen.getByPlaceholderText("D:\\Projects\\example 或 /Users/me/project"))
        .toHaveValue("D:\\picked\\project")
    );

    fireEvent.click(screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    }));

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "copy",
        "D:\\picked\\project"
      );
    });
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
      target: { value: "D:\\manual\\project" },
    });
    fireEvent.click(screen.getByRole("button", { name: "选择项目文件夹" }));

    await waitFor(() => expect(openDialog).toHaveBeenCalled());
    expect(input).toHaveValue("D:\\manual\\project");
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

  it("defaults to symlink for remote targets that support symlinks", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);
    const remoteTarget: TargetSummary = {
      id: "ssh-demo",
      kind: "ssh",
      label: "Demo",
      remoteOs: "Linux",
      symlinkEnabled: true,
      isActive: true,
    };
    useTargetStore.setState({
      targets: [localTarget, remoteTarget],
      activeTarget: remoteTarget,
    });

    renderDialog();

    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "symlink",
        null
      );
    });
  });

  it("keeps partial failures open with agent error details", async () => {
    mockOnInstall.mockResolvedValueOnce({
      succeeded: ["codex"],
      skipped: [],
      failed: [
        { agent_id: "claude-code", error: "A remote directory already exists" },
      ],
    });

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(screen.getByText(/claude-code: A remote directory already exists/)).toBeInTheDocument();
    });
    expect(mockOnOpenChange).not.toHaveBeenCalledWith(false);
  });

  it("closes without an error when every selected target is skipped", async () => {
    mockOnInstall.mockResolvedValueOnce({
      succeeded: [],
      skipped: [
        {
          agent_id: "claude-code",
          target_path: "/Users/test/.claude/skills/frontend-design",
          reason: "already_installed",
        },
      ],
      failed: [],
    });

    renderDialog();
    fireEvent.click(screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    }));

    await waitFor(() => {
      expect(mockOnOpenChange).toHaveBeenCalledWith(false);
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("calls onOpenChange(false) after successful install", async () => {
    mockOnInstall.mockResolvedValueOnce(successInstallResult);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it("shows error message when install fails", async () => {
    mockOnInstall.mockRejectedValueOnce(new Error("Permission denied"));

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
      expect(screen.getByText(/Permission denied/)).toBeInTheDocument();
    });
  });

  // ── Cancel ────────────────────────────────────────────────────────────────

  it("calls onOpenChange(false) when Cancel is clicked", () => {
    renderDialog();
    const cancelBtn = screen.getByRole("button", { name: /取消/i });
    fireEvent.click(cancelBtn);
    expect(mockOnOpenChange).toHaveBeenCalledWith(false);
  });

  // ── Checkbox Interaction ──────────────────────────────────────────────────

  it("updates confirm button count when checkbox toggled", async () => {
    renderDialog();

    // Initially 3 selected: Claude Code, Kiro, and Universal via Codex.
    expect(
      screen.getByRole("button", { name: /安装到 3 个平台/i })
    ).toBeInTheDocument();

    // Uncheck Kiro.
    const kiroCheckbox = screen.getByLabelText("Kiro");
    fireEvent.click(kiroCheckbox);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /安装到 2 个平台/i })
      ).toBeInTheDocument();
    });
  });

  it("disables confirm when no platforms selected", async () => {
    // Start with no linked agents; defaults still select all visible targets.
    const noLinkedSkill: SkillWithLinks = {
      ...mockSkill,
      linked_agents: [],
    };

    render(
      <InstallDialog
        open={true}
        onOpenChange={mockOnOpenChange}
        skill={noLinkedSkill}
        agents={getPlatformTargetGroups(mockAgents, {
          coding: true,
          lobster: true,
        })}
        onInstall={mockOnInstall}
      />
    );

    fireEvent.click(screen.getByLabelText("Universal (.agents/skills)"));
    fireEvent.click(screen.getByLabelText("Claude Code"));
    fireEvent.click(screen.getByLabelText("Kiro"));

    const confirmBtn = screen.getByRole("button", {
      name: /安装到 0 个平台/i,
    });
    expect(confirmBtn).toBeDisabled();
  });

  // ── No Skill ──────────────────────────────────────────────────────────────

  it("renders nothing when skill is null", () => {
    render(
      <InstallDialog
        open={true}
        onOpenChange={mockOnOpenChange}
        skill={null}
        agents={mockAgents}
        onInstall={mockOnInstall}
      />
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
