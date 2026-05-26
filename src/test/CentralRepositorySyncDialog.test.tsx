import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CentralRepositorySyncDialog } from "@/components/central/CentralRepositorySyncDialog";
import type { AgentWithStatus, BatchDeleteCentralSkillPreviewResult, SkillWithLinks } from "@/types";
import type { CentralRemoteAddedSkill, CentralRepositorySyncPreview } from "@/types/centralRepositorySync";

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

function remoteAddedSkill(
  sourcePath: string,
  skillId: string,
  skillName: string,
  conflict?: {
    existingSkillId: string;
    existingName: string;
  }
): CentralRemoteAddedSkill {
  return {
    repositoryId: "github-openai-skills-main",
    repo: {
      owner: "openai",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/openai/skills",
    },
    preview: {
      sourcePath,
      skillId,
      skillName,
      description: null,
      rootDirectory: "skills",
      skillDirectoryName: skillId,
      downloadUrl: `https://raw.githubusercontent.com/openai/skills/main/${sourcePath}/SKILL.md`,
      conflict: conflict
        ? {
            ...conflict,
            existingCanonicalPath: "~/.skillsmanage/skills/conflicting-skill",
            proposedSkillId: skillId,
            proposedName: skillName,
          }
        : null,
    },
  };
}

const existingSkills: SkillWithLinks[] = [
  {
    id: "conflicting-skill",
    name: "Local Conflicting Skill",
    file_path: "~/.skillsmanage/skills/conflicting-skill/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/conflicting-skill",
    is_central: true,
    scanned_at: "2026-05-19T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
    repository: {
      id: "github-anthropic-skills-main",
      name: "anthropic/skills",
      source_type: "github",
      owner: "anthropic",
      repo: "skills",
      branch: "main",
      url: "https://github.com/anthropic/skills",
      pinned: false,
      is_unknown: false,
      created_at: "2026-05-19T00:00:00Z",
      updated_at: "2026-05-19T00:00:00Z",
    },
    source_path: "skills/conflicting",
  },
  {
    id: "unsourced-skill",
    name: "Unsourced Skill",
    file_path: "~/.skillsmanage/skills/unsourced-skill/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/unsourced-skill",
    is_central: true,
    scanned_at: "2026-05-19T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
  },
];

const preview: CentralRepositorySyncPreview = {
  states: [],
  remoteAdded: [],
  skippedRemoteAdded: [],
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
      skills={existingSkills}
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
        [],
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
        [],
        [],
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
        [],
        [],
        []
      );
    });
  });



  it("renders four tabs and defaults to pending additions before skipped, missing, and failures", () => {
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [remoteAddedSkill("skills/new-skill", "new-skill", "New Skill")],
          skippedRemoteAdded: [remoteAddedSkill("skills/skipped-skill", "skipped-skill", "Skipped Skill")],
          failedRepositories: [
            { repositoryId: "repo-failed", name: "Broken Repo", error: "network failed" },
          ],
        }}
        deletePreview={deletePreview}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={vi.fn()}
      />
    );

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("tab", { name: "待处理新增 1" })).toHaveAttribute(
      "aria-selected",
      "true"
    );
    expect(within(dialog).getByRole("tab", { name: "已跳过新增 1" })).toBeInTheDocument();
    expect(within(dialog).getByRole("tab", { name: "远端删除 3" })).toBeInTheDocument();
    expect(within(dialog).getByRole("tab", { name: "失败仓库 1" })).toBeInTheDocument();
    expect(within(dialog).getByText("New Skill")).toBeInTheDocument();
    expect(within(dialog).queryByText("Skipped Skill")).not.toBeInTheDocument();
  });

  it("shows delete-old-skill only for conflict rows with a removable Central skill", () => {
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [
            remoteAddedSkill("skills/conflicting", "conflicting-skill", "Conflicting Remote", {
              existingSkillId: "conflicting-skill",
              existingName: "Local Conflicting Skill",
            }),
            remoteAddedSkill("skills/fresh", "fresh-skill", "Fresh Remote"),
          ],
          remoteMissing: [],
        }}
        deletePreview={{
          previews: [
            {
              skill_id: "conflicting-skill",
              skill_name: "Local Conflicting Skill",
              central_path: "~/.skillsmanage/skills/conflicting-skill",
              copy_installations: [],
              auto_removed_agent_ids: ["cursor"],
            },
          ],
          failed: [],
        }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={vi.fn()}
      />
    );

    const dialog = screen.getByRole("dialog");
    const conflictRow = within(dialog).getByText("Conflicting Remote").closest("article")!;
    const freshRow = within(dialog).getByText("Fresh Remote").closest("article")!;
    expect(within(conflictRow).getByRole("button", { name: "删除旧 skill" })).toBeInTheDocument();
    expect(within(freshRow).queryByRole("button", { name: "删除旧 skill" })).not.toBeInTheDocument();
  });

  it("pending delete-old-skill submits only deleteRequests and leaves the remote addition pending", async () => {
    const onConfirm = vi.fn();
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [
            remoteAddedSkill("skills/conflicting", "conflicting-skill", "Conflicting Remote", {
              existingSkillId: "conflicting-skill",
              existingName: "Local Conflicting Skill",
            }),
          ],
          remoteMissing: [],
        }}
        deletePreview={{
          previews: [
            {
              skill_id: "conflicting-skill",
              skill_name: "Local Conflicting Skill",
              central_path: "~/.skillsmanage/skills/conflicting-skill",
              copy_installations: [],
              auto_removed_agent_ids: ["cursor"],
            },
          ],
          failed: [],
        }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={onConfirm}
      />
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "删除旧 skill" }));
    expect(within(dialog).getByText("中央路径：~/.skillsmanage/skills/conflicting-skill")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        [],
        [{ skill_id: "conflicting-skill", remove_agent_ids: [] }],
        [],
        [],
        []
      );
    });
  });

  it("skipped delete-old-skill submits only deleteRequests and keeps the remembered skip state", async () => {
    const onConfirm = vi.fn();
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [],
          skippedRemoteAdded: [
            remoteAddedSkill("skills/conflicting", "conflicting-skill", "Conflicting Remote", {
              existingSkillId: "conflicting-skill",
              existingName: "Local Conflicting Skill",
            }),
          ],
          remoteMissing: [],
        }}
        deletePreview={{
          previews: [
            {
              skill_id: "conflicting-skill",
              skill_name: "Local Conflicting Skill",
              central_path: "~/.skillsmanage/skills/conflicting-skill",
              copy_installations: [],
              auto_removed_agent_ids: [],
            },
          ],
          failed: [],
        }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={onConfirm}
      />
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "删除旧 skill" }));
    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        [],
        [{ skill_id: "conflicting-skill", remove_agent_ids: [] }],
        [],
        [],
        []
      );
    });
  });

  it("disables Apply and shows an inline error for invalid rename ids", () => {
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [remoteAddedSkill("skills/new-skill", "new-skill", "New Skill")],
          remoteMissing: [],
        }}
        deletePreview={{ previews: [], failed: [] }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={vi.fn()}
      />
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "重命名" }));
    fireEvent.change(within(dialog).getByLabelText("重命名后的技能 ID"), {
      target: { value: "Bad Skill!" },
    });

    expect(within(dialog).getByText("技能 ID 只能使用小写字母、数字和单个短横线。")).toBeInTheDocument();
    expect(within(dialog).getByTestId("confirm-repo-sync")).toBeDisabled();
  });

  it("shows failed repositories as cards instead of a semicolon-joined warning line", () => {
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [],
          remoteMissing: [],
          failedRepositories: [
            { repositoryId: "repo-one", name: "Repo One", error: "first failure" },
            { repositoryId: "repo-two", name: "Repo Two", error: "second failure" },
          ],
        }}
        deletePreview={{ previews: [], failed: [] }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={vi.fn()}
      />
    );

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("tab", { name: "失败仓库 2" })).toHaveAttribute(
      "aria-selected",
      "true"
    );
    expect(within(dialog).getByText("Repo One")).toBeInTheDocument();
    expect(within(dialog).getByText("Repo Two")).toBeInTheDocument();
    expect(within(dialog).getByText("first failure")).toBeInTheDocument();
    expect(within(dialog).getByText("second failure")).toBeInTheDocument();
    expect(within(dialog).queryByText("first failure; second failure")).not.toBeInTheDocument();
  });

  it("submits active remote additions selected as Skip through skipAdditions", async () => {
    const onConfirm = vi.fn();
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [remoteAddedSkill("skills/planning-with-files-ar", "planning-with-files-ar", "Planning AR")],
          remoteMissing: [],
        }}
        deletePreview={{ previews: [], failed: [] }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={onConfirm}
      />
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "跳过" }));
    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        [],
        [],
        [],
        [
          {
            repositoryId: "github-openai-skills-main",
            sourcePath: "skills/planning-with-files-ar",
            skillId: "planning-with-files-ar",
            skillName: "Planning AR",
          },
        ],
        []
      );
    });
  });

  it("shows remote and existing source details for conflicting remote additions", () => {
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [
            remoteAddedSkill("skills/conflicting", "conflicting-skill", "Conflicting Remote", {
              existingSkillId: "conflicting-skill",
              existingName: "Legacy Conflict Name",
            }),
            remoteAddedSkill("skills/unsourced", "unsourced-skill", "Unsourced Remote", {
              existingSkillId: "unsourced-skill",
              existingName: "Unsourced Skill",
            }),
          ],
          remoteMissing: [],
        }}
        deletePreview={{ previews: [], failed: [] }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={vi.fn()}
      />
    );

    const dialog = screen.getByRole("dialog");
    expect(
      within(dialog).getByText(
        "冲突：远端 openai/skills/skills/conflicting ↔ 已有 Local Conflicting Skill（anthropic/skills/skills/conflicting）"
      )
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText(
        "冲突：远端 openai/skills/skills/unsourced ↔ 已有 Unsourced Skill（未分配来源）"
      )
    ).toBeInTheDocument();
  });

  it("shows skipped remote additions separately and can import, rename, or re-show them", async () => {
    const onConfirm = vi.fn();
    render(
      <CentralRepositorySyncDialog
        open
        onOpenChange={vi.fn()}
        preview={{
          ...preview,
          remoteAdded: [],
          skippedRemoteAdded: [
            remoteAddedSkill("skills/skipped-import", "skipped-import", "Skipped Import"),
            remoteAddedSkill("skills/skipped-rename", "skipped-rename", "Skipped Rename"),
            remoteAddedSkill("skills/skipped-unskip", "skipped-unskip", "Skipped Unskip"),
          ],
          remoteMissing: [],
        }}
        deletePreview={{ previews: [], failed: [] }}
        agents={agents}
        skills={existingSkills}
        isPreviewLoading={false}
        isApplying={false}
        error={null}
        onConfirm={onConfirm}
      />
    );

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("上次检测已跳过（3）")).toBeInTheDocument();

    const importRow = within(dialog).getByText("Skipped Import").closest("article")!;
    fireEvent.click(within(importRow).getByRole("button", { name: "导入" }));

    const renameRow = within(dialog).getByText("Skipped Rename").closest("article")!;
    fireEvent.click(within(renameRow).getByRole("button", { name: "重命名" }));
    fireEvent.change(within(renameRow).getByLabelText("重命名后的技能 ID"), {
      target: { value: "skipped-rename-cn" },
    });

    const unskipRow = within(dialog).getByText("Skipped Unskip").closest("article")!;
    fireEvent.click(within(unskipRow).getByRole("button", { name: "重新显示" }));

    fireEvent.click(within(dialog).getByTestId("confirm-repo-sync"));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        [],
        [],
        [
          {
            repositoryId: "github-openai-skills-main",
            previewWorkspaceId: null,
            selections: [
              {
                sourcePath: "skills/skipped-import",
                resolution: "overwrite",
                renamedSkillId: null,
              },
              {
                sourcePath: "skills/skipped-rename",
                resolution: "rename",
                renamedSkillId: "skipped-rename-cn",
              },
            ],
          },
        ],
        [],
        [
          {
            repositoryId: "github-openai-skills-main",
            sourcePath: "skills/skipped-unskip",
          },
        ]
      );
    });
  });
});
