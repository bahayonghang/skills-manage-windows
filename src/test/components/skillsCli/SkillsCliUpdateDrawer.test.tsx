import { createRef } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliUpdateDrawer } from "@/components/skillsCli/SkillsCliUpdateDrawer";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliGlobalSkill,
  type SkillsCliUpdateInventory,
} from "@/types";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const skill: SkillsCliGlobalSkill = {
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
};

const inventory: SkillsCliUpdateInventory = {
  ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  lastSuccessAt: "2026-08-26T00:00:00.000Z",
  skills: [
    {
      skillName: "demo-skill",
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      skillPath: "demo-skill",
      status: "update_available",
      installedRevisionSha: "aaaaaaa1",
      observedRevisionSha: "bbbbbbb2",
      pendingRevisionSha: "bbbbbbb2",
      installedLocalDigest: "sha256-v1:a",
      observedUpstreamDigest: "sha256-v1:b",
      pendingUpstreamDigest: "sha256-v1:b",
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
  repositories: [
    {
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      branch: "main",
      observedRevisionSha: "bbbbbbb2",
      status: "ok",
      lastCheckedAt: "2026-08-26T00:00:00.000Z",
      lastErrorCode: null,
      rateLimitResetAt: null,
      pendingCount: 1,
    },
  ],
};

describe("SkillsCliUpdateDrawer", () => {
  beforeEach(() => {
    showSkillsCliActionToast.mockClear();
  });

  it("renders installed to observed, summary, argv preview, and applies the preselected skill", async () => {
    const onApply = vi.fn().mockResolvedValue({
      appliedSkillNames: ["demo-skill"],
      installedRevisionSha: "bbbbbbb2",
    });
    const onClose = vi.fn();
    render(
      <SkillsCliUpdateDrawer
        open
        repositoryKey="owner/repo@main"
        skillNames={["demo-skill"]}
        skills={[skill]}
        inventory={inventory}
        contentWidth={800}
        updateError={null}
        updateJobPhase={null}
        updateProgress={null}
        returnFocusRef={createRef<HTMLElement>()}
        onClose={onClose}
        onApply={onApply}
        onVerifyBaseline={vi.fn()}
      />,
    );

    const drawer = screen.getByTestId("skills-cli-update-drawer");
    expect(drawer).toHaveAttribute("data-width-mode", "fixed460");
    expect(screen.getByTestId("skills-cli-update-row-demo-skill")).toHaveAttribute(
      "data-status",
      "update_available",
    );
    expect(screen.getByText(/aaaaaaa/)).toBeInTheDocument();
    expect(screen.getByText(/bbbbbbb/)).toBeInTheDocument();
    expect(screen.getByText("SKILL.md")).toBeInTheDocument();
    expect(screen.getByTestId("skills-cli-update-argv")).toHaveTextContent(
      "refresh owned-canonical from-pinned-github-snapshot demo-skill",
    );
    expect(screen.getByTestId("skills-cli-update-argv").textContent).not.toContain(
      "--force",
    );

    fireEvent.click(screen.getByTestId("skills-cli-update-apply"));
    expect(onApply).toHaveBeenCalledWith({
      repositoryKey: "owner/repo@main",
      skillNames: ["demo-skill"],
    });
  });

  it("keeps baseline rows out of Update selected and offers overwrite reinstall", async () => {
    const onApply = vi.fn().mockResolvedValue({
      appliedSkillNames: ["demo-skill"],
      installedRevisionSha: "bbbbbbb2",
    });
    render(
      <SkillsCliUpdateDrawer
        open
        repositoryKey="owner/repo@main"
        skillNames={["demo-skill"]}
        skills={[skill]}
        inventory={{
          ...inventory,
          skills: inventory.skills.map((row) => ({
            ...row,
            status: "baseline_required" as const,
            installedRevisionSha: null,
            installedLocalDigest: null,
          })),
        }}
        contentWidth={800}
        updateError={null}
        updateJobPhase={null}
        updateProgress={null}
        onClose={vi.fn()}
        onApply={onApply}
        onVerifyBaseline={vi.fn()}
      />,
    );

    expect(screen.getByTestId("skills-cli-update-apply")).toBeDisabled();
    expect(screen.getByTestId("skills-cli-update-reinstall-warning")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("skills-cli-update-reinstall"));
    expect(onApply).toHaveBeenCalledWith({
      repositoryKey: "owner/repo@main",
      skillNames: ["demo-skill"],
    });
  });

  it("uses full width below the 720px content band and shows empty summary", () => {
    render(
      <SkillsCliUpdateDrawer
        open
        repositoryKey="owner/repo@main"
        skillNames={["demo-skill"]}
        skills={[skill]}
        inventory={{
          ...inventory,
          skills: inventory.skills.map((row) => ({
            ...row,
            changeSummary: [],
          })),
        }}
        contentWidth={500}
        updateError={null}
        updateJobPhase={null}
        updateProgress={null}
        onClose={vi.fn()}
        onApply={vi.fn()}
        onVerifyBaseline={vi.fn()}
      />,
    );
    expect(screen.getByTestId("skills-cli-update-drawer")).toHaveAttribute(
      "data-width-mode",
      "fullWidth",
    );
    expect(
      screen.getByText("该技能没有可用的文件级摘要。"),
    ).toBeInTheDocument();
  });
});
