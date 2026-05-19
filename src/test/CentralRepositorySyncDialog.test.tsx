import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CentralRepositorySyncDialog } from "@/components/central/CentralRepositorySyncDialog";
import type { AgentWithStatus, BatchDeleteCentralSkillPreviewResult } from "@/types";
import type { CentralRepositorySyncPreview } from "@/types/centralRepositorySync";

const agents: AgentWithStatus[] = [
  {
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "~/.cursor/skills",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

function remoteMissingState(skillId: string, sourcePath: string, url = "https://github.com/openai/skills") {
  return {
    skill_id: skillId,
    source_type: "github" as const,
    source_url: url,
    ref: "main",
    source_path: sourcePath,
    last_remote_hash: null,
    latest_remote_hash: null,
    last_checked_at: "2026-05-19T00:00:00Z",
    last_updated_at: null,
    status: "remote_missing" as const,
    error: "removed remotely",
  };
}

const preview: CentralRepositorySyncPreview = {
  states: [],
  remoteAdded: [],
  remoteMissing: [
    {
      state: remoteMissingState("frontend-design", "skills/frontend-design"),
      repositoryId: "github-openai-skills-main",
      repositoryName: "openai/skills",
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
    },
    {
      state: remoteMissingState("code-reviewer", "skills/code-reviewer"),
      repositoryId: "github-openai-skills-main",
      repositoryName: "openai/skills",
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
    },
    {
      state: remoteMissingState("unknown-source", "skills/unknown", "https://github.com/unknown/source"),
      repositoryId: null,
      repositoryName: "Unknown source",
      repo: null,
    },
  ],
  repositories: [],
  failedRepositories: [],
};

const deletePreview: BatchDeleteCentralSkillPreviewResult = {
  previews: [
    {
      skill_id: "frontend-design",
      skill_name: "frontend-design",
      central_path: "~/.skillsmanage/skills/frontend-design",
      copy_installations: [
        {
          skill_id: "frontend-design",
          agent_id: "cursor",
          installed_path: "~/.cursor/skills/frontend-design",
          link_type: "copy",
          symlink_target: undefined,
          installed_at: "2026-05-19T00:00:00Z",
        },
      ],
      auto_removed_agent_ids: [],
    },
    {
      skill_id: "code-reviewer",
      skill_name: "code-reviewer",
      central_path: "~/.skillsmanage/skills/code-reviewer",
      copy_installations: [],
      auto_removed_agent_ids: [],
    },
  ],
  failed: [{ skill_id: "unknown-source", error: "preview failed" }],
};

function renderDialog(onConfirm = vi.fn()) {
  render(
    <CentralRepositorySyncDialog
      open
      onOpenChange={vi.fn()}
      preview={preview}
      deletePreview={deletePreview}
      agents={agents}
      isPreviewLoading={false}
      isApplying={false}
      error={null}
      onConfirm={onConfirm}
    />
  );
  return onConfirm;
}

describe("CentralRepositorySyncDialog", () => {
  it("keeps remote removals by default and displays repository/source details", async () => {
    const onConfirm = renderDialog();

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getAllByText("仓库：openai/skills · main")).toHaveLength(2);
    expect(within(dialog).getByText("仓库：未知来源")).toBeInTheDocument();
    expect(within(dialog).getByText("来源路径：skills/frontend-design")).toBeInTheDocument();
    expect(within(dialog).getByText("3 个远端已删除 · 0 个将删除 · 1 个不可用")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        ["frontend-design", "code-reviewer", "unknown-source"],
        [],
        []
      );
    });
  });

  it("bulk-selects only removable delete previews and keeps unavailable removals", async () => {
    const onConfirm = renderDialog();

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "删除所有可删除项" }));

    expect(within(dialog).getByText("3 个远端已删除 · 2 个将删除 · 1 个不可用")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        ["unknown-source"],
        [
          {
            skill_id: "frontend-design",
            remove_agent_ids: ["cursor"],
          },
          {
            skill_id: "code-reviewer",
            remove_agent_ids: [],
          },
        ],
        []
      );
    });
  });

  it("returns all remote removals to keep after bulk keep", async () => {
    const onConfirm = renderDialog();

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "删除所有可删除项" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "全部保留" }));
    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        ["frontend-design", "code-reviewer", "unknown-source"],
        [],
        []
      );
    });
  });
});
