import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ModalInstallButton } from "@/components/skill/ModalInstallButton";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useTargetStore } from "@/stores/targetStore";
import { usePlatformStore } from "@/stores/platformStore";
import type { AgentWithStatus, SkillDetail as SkillDetailType } from "@/types";

vi.mock("@/stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

const mockInstallSkill = vi.fn();

const baseAgents: AgentWithStatus[] = [
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
    global_skills_dir: "~/.cursor/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const baseDetail: SkillDetailType = {
  id: "test-skill",
  name: "Test Skill",
  description: "A test skill",
  file_path: "~/.skillsmanage/skills/test-skill/SKILL.md",
  canonical_path: "~/.skillsmanage/skills/test-skill",
  is_central: true,
  source: "native",
  scanned_at: "2026-01-01T00:00:00Z",
  installations: [],
  collections: [],
};

function applyMocks(overrides: {
  detail?: Partial<SkillDetailType> | null;
  installingAgentId?: string | null;
  agents?: AgentWithStatus[];
} = {}) {
  const detail = overrides.detail === null
    ? null
    : { ...baseDetail, ...(overrides.detail ?? {}) };

  vi.mocked(useSkillDetailStore).mockImplementation((selector?: unknown) => {
    const state = {
      detail,
      installingAgentId: overrides.installingAgentId ?? null,
      installSkill: mockInstallSkill,
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });

  vi.mocked(useTargetStore).mockImplementation((selector?: unknown) => {
    const state = {
      activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });

  vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
    const state = {
      agents: overrides.agents ?? baseAgents,
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

describe("ModalInstallButton", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not render when is_read_only is true", () => {
    applyMocks({ detail: { is_read_only: true } });
    const { container } = render(<ModalInstallButton skillId="test-skill" />);
    expect(container.innerHTML).toBe("");
  });

  it("shows spinner and disabled state when installing", () => {
    applyMocks({ installingAgentId: "claude-code" });
    render(<ModalInstallButton skillId="test-skill" />);

    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    // Loader2 has animate-spin class
    const spinner = button.querySelector(".animate-spin");
    expect(spinner).not.toBeNull();
  });

  it("shows check icon and disabled text when all agents are installed", () => {
    applyMocks({
      detail: {
        installations: [
          { agent_id: "claude-code" },
          { agent_id: "cursor" },
        ] as SkillDetailType["installations"],
      },
    });
    render(<ModalInstallButton skillId="test-skill" />);

    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent("已安装");
  });

  it("renders disabled button when enabledAgents is empty", () => {
    applyMocks({ agents: [] });
    render(<ModalInstallButton skillId="test-skill" />);

    const button = screen.getByRole("button");
    expect(button).toBeDisabled();
  });

  it("renders clickable button that calls installSkill in normal state", () => {
    applyMocks();
    render(<ModalInstallButton skillId="test-skill" />);

    const button = screen.getByRole("button");
    expect(button).not.toBeDisabled();
    expect(button).toHaveTextContent("安装");

    fireEvent.click(button);
    expect(mockInstallSkill).toHaveBeenCalledWith("test-skill", "claude-code");
  });

  it("has aria-label in format '安装 {技能名称}'", () => {
    applyMocks();
    render(<ModalInstallButton skillId="test-skill" />);

    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "安装 Test Skill");
  });
});
