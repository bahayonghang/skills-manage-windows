import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { CollectionInstallDialog } from "@/components/collection/CollectionInstallDialog";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";
import { AgentWithStatus } from "@/types";

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
    is_detected: false,
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

const mockOnInstall = vi.fn();
const mockOnOpenChange = vi.fn();

function renderDialog() {
  render(
    <CollectionInstallDialog
      open={true}
      onOpenChange={mockOnOpenChange}
      collectionName="Starter"
      skillCount={3}
      agents={getPlatformTargetGroups(mockAgents, {
        coding: true,
        lobster: true,
      })}
      onInstall={mockOnInstall}
    />
  );
}

describe("CollectionInstallDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("defaults to enabled visible platform targets, including undetected ones", async () => {
    mockOnInstall.mockResolvedValueOnce({ succeeded: ["claude-code", "kiro"], failed: [] });

    renderDialog();

    expect(
      screen.getByRole("button", { name: /安装到 2 个平台/i })
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockOnInstall).toHaveBeenCalledWith(["claude-code", "kiro"]);
    });
  });
});
