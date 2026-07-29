import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { LocalRemoteSyncDialog } from "@/components/settings/LocalRemoteSyncDialog";
import type { LocalRemoteSyncApplyResult, LocalRemoteSyncPreview } from "@/types";

const preview: LocalRemoteSyncPreview = {
  targetId: "wsl-1",
  targetLabel: "Ubuntu",
  repoRoot: "D:/repo",
  skillsRoot: "C:/Users/alice/.skillsmanage/skills",
  repo: {
    id: "skills-manage-windows",
    label: "skills-manage-windows",
    kind: "repo",
    localPath: "D:/repo",
    remotePath: "/home/alice/.skillsmanage/repos/skills-manage-windows",
    fileCount: 2,
    byteCount: 2048,
    localHash: "sha256-manifest:local",
    remoteHash: null,
    status: "add",
  },
  skills: [
    {
      id: "planning-with-files-zh",
      label: "planning-with-files-zh",
      kind: "skill",
      localPath: "C:/Users/alice/.skillsmanage/skills/planning-with-files-zh",
      remotePath: "/home/alice/.skillsmanage/skills/planning-with-files-zh",
      fileCount: 1,
      byteCount: 128,
      localHash: "sha256-manifest:skill",
      remoteHash: "sha256-manifest:old",
      status: "update",
    },
  ],
  totalFileCount: 3,
  totalByteCount: 2176,
};

const skipPreview: LocalRemoteSyncPreview = {
  ...preview,
  repo: {
    ...preview.repo,
    status: "skip",
    remoteHash: preview.repo.localHash,
  },
  skills: preview.skills.map((skill) => ({
    ...skill,
    status: "skip",
    remoteHash: skill.localHash,
  })),
};

function renderDialog(props: Partial<Parameters<typeof LocalRemoteSyncDialog>[0]> = {}) {
  const onApply = props.onApply ?? vi.fn();
  render(
    <LocalRemoteSyncDialog
      open
      targetLabel="Ubuntu"
      preview={preview}
      result={null}
      isPreviewing={false}
      isApplying={false}
      error={null}
      onOpenChange={vi.fn()}
      onPreview={vi.fn()}
      onApply={onApply}
      {...props}
    />
  );
  return { onApply };
}

describe("LocalRemoteSyncDialog", () => {
  it("renders title and target label", () => {
    renderDialog({ preview: null });

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("同步本机 repo 与 skills")).toBeInTheDocument();
    expect(within(dialog).getByText(/Ubuntu/)).toBeInTheDocument();
    expect(within(dialog).getByText(/应用前先生成预览/)).toBeInTheDocument();
    expect(within(dialog).getByText("选择目标")).toBeInTheDocument();
    expect(within(dialog).getByText("预览快照")).toBeInTheDocument();
    expect(within(dialog).getAllByText("应用同步").length).toBeGreaterThan(0);
    expect(within(dialog).getByText(/不复制数据库、凭据/)).toBeInTheDocument();
  });

  it("shows repo remote path and skills summary when preview exists", () => {
    renderDialog();

    const dialog = screen.getByRole("dialog");
    expect(
      within(dialog).getByText("/home/alice/.skillsmanage/repos/skills-manage-windows")
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText("共 1 个 skill，其中 1 个需要同步。")
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText(/仓库：新增 · Skills：共 1 个，1 个变化/)
    ).toBeInTheDocument();
  });

  it("calls onApply when apply is clicked", () => {
    const { onApply } = renderDialog();

    fireEvent.click(screen.getByTestId("apply-local-remote-sync"));

    expect(onApply).toHaveBeenCalled();
  });

  it("disables apply when preview has no syncable changes", () => {
    renderDialog({ preview: skipPreview });

    expect(screen.getByTestId("apply-local-remote-sync")).toBeDisabled();
    expect(screen.getByText(/没有需要同步的变化/)).toBeInTheDocument();
  });

  it("shows item error warning but allows syncing eligible changes", () => {
    renderDialog({
      preview: {
        ...preview,
        skills: [
          ...preview.skills,
          {
            ...preview.skills[0],
            id: "bad-skill",
            label: "bad-skill",
            status: "error",
            error: "hash failed",
          },
        ],
      },
    });

    expect(screen.getByText(/存在错误项/)).toBeInTheDocument();
    expect(screen.getByTestId("apply-local-remote-sync")).not.toBeDisabled();
  });

  it("shows apply failures in the result panel", () => {
    const result: LocalRemoteSyncApplyResult = {
      targetId: "wsl-1",
      targetLabel: "Ubuntu",
      syncedRepo: preview.repo,
      syncedSkills: [],
      skippedSkills: [],
      failed: [
        {
          id: "bad-skill",
          label: "bad-skill",
          targetPath: "/home/alice/.skillsmanage/skills/bad-skill",
          error: "permission denied",
        },
      ],
    };

    renderDialog({ result });

    expect(screen.getByText(/远程同步完成，但有 1 个失败/)).toBeInTheDocument();
    expect(screen.getByText(/permission denied/)).toBeInTheDocument();
  });
});
