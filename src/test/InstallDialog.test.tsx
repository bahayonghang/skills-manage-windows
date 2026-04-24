import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { InstallDialog } from "../components/central/InstallDialog";
import { AgentWithStatus, SkillWithLinks } from "../types";
import { getPlatformTargetGroups } from "../lib/platformTargetGroups";

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
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockSkill: SkillWithLinks = {
  id: "frontend-design",
  name: "frontend-design",
  description: "Build distinctive, production-grade frontend interfaces",
  file_path: "~/.agents/skills/frontend-design/SKILL.md",
  canonical_path: "~/.agents/skills/frontend-design",
  is_central: true,
  scanned_at: "2026-04-09T00:00:00Z",
  linked_agents: ["claude-code", "codex"],
  shared_root_agents: ["codex", "cursor", "gemini-cli"],
};

const mockOnInstall = vi.fn();
const mockOnOpenChange = vi.fn();

function renderDialog(props: {
  open?: boolean;
  skill?: SkillWithLinks | null;
} = {}) {
  const targetAgents = getPlatformTargetGroups(mockAgents, {
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

  it("shows 'already linked' badge for linked agents", () => {
    renderDialog();
    // Claude Code is in linked_agents
    expect(screen.getAllByText("已链接").length).toBeGreaterThanOrEqual(1);
  });

  it("shows 'not detected' badge for undetected agents", () => {
    renderDialog();
    // Kiro has is_detected: false
    expect(screen.getByText("(未检测到)")).toBeInTheDocument();
  });

  it("shows Universal as available but disabled for central skills", () => {
    renderDialog();

    expect(screen.getByLabelText("Universal (.agents/skills)")).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByText("始终包含")).toBeInTheDocument();
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
    // By default, linked agents (claude-code) are pre-selected.
    // Unlinked independent agents are not pre-selected.
    // So 1 is pre-selected: claude-code
    expect(
      screen.getByRole("button", { name: /安装到 1 个平台/i })
    ).toBeInTheDocument();
  });

  it("calls onInstall with selected agent IDs on confirm", async () => {
    mockOnInstall.mockResolvedValueOnce(undefined);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        expect.any(String)
      );
    });
  });

  it("does not submit shared-root agents on confirm", async () => {
    mockOnInstall.mockResolvedValueOnce(undefined);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        ["claude-code"],
        "symlink"
      );
    });
  });

  it("passes 'symlink' method to onInstall by default", async () => {
    mockOnInstall.mockResolvedValueOnce(undefined);

    renderDialog();
    const confirmBtn = screen.getByRole("button", {
      name: /安装到 .* 个平台/i,
    });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(
        "frontend-design",
        expect.any(Array),
        "symlink"
      );
    });
  });

  it("passes 'copy' method to onInstall when copy is selected", async () => {
    mockOnInstall.mockResolvedValueOnce(undefined);

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
        "copy"
      );
    });
  });

  it("calls onOpenChange(false) after successful install", async () => {
    mockOnInstall.mockResolvedValueOnce(undefined);

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

    // Initially 1 selected (claude-code, already linked)
    expect(
      screen.getByRole("button", { name: /安装到 1 个平台/i })
    ).toBeInTheDocument();

    // Check Kiro (add 1 more)
    const kiroCheckbox = screen.getByLabelText("Kiro");
    fireEvent.click(kiroCheckbox);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /安装到 2 个平台/i })
      ).toBeInTheDocument();
    });
  });

  it("disables confirm when no platforms selected", async () => {
    // Start with NO agents linked so none are pre-selected
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

    // 0 selected → confirm button disabled
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
