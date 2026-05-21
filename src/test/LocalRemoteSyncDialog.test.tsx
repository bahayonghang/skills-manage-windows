import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { LocalRemoteSyncDialog } from "@/components/settings/LocalRemoteSyncDialog";
import type { LocalRemoteSyncPreview } from "@/types";

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
  });

  it("calls onApply when apply is clicked", () => {
    const { onApply } = renderDialog();

    fireEvent.click(screen.getByTestId("apply-local-remote-sync"));

    expect(onApply).toHaveBeenCalled();
  });
});
