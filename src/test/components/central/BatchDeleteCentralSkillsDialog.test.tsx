import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { BatchDeleteCentralSkillsDialog } from "@/components/central/BatchDeleteCentralSkillsDialog";
import type { AgentWithStatus, BatchDeleteCentralSkillPreviewResult } from "@/types";

const agents: AgentWithStatus[] = [
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

function recoveryPreview(
  skillId: string,
  eligible: boolean,
): BatchDeleteCentralSkillPreviewResult["previews"][number] {
  return {
    skill_id: skillId,
    skill_name: skillId,
    central_path: `~/.skillsmanage/skills/${skillId}`,
    copy_installations: [],
    auto_removed_agent_ids: [],
    pending_recovery: {
      operation_id: `${skillId}-op`,
      operation_kind: "central_delete",
      phase: eligible ? "prepared" : "fs_staged",
      error_code: "central_operation.delete_restore_collision",
      force_delete_eligible: eligible,
      blocker_codes: eligible ? [] : ["recovery.reconcile_unsupported_phase"],
    },
  };
}

function preview(eligible: boolean): BatchDeleteCentralSkillPreviewResult {
  return {
    previews: [recoveryPreview("yao-meta", eligible)],
    failed: [],
  };
}

describe("BatchDeleteCentralSkillsDialog", () => {
  it("shows force delete only for eligible pending recovery", () => {
    const onConfirm = vi.fn().mockResolvedValue({ succeeded: [], failed: [] });
    render(
      <BatchDeleteCentralSkillsDialog
        open
        onOpenChange={vi.fn()}
        skillIds={["yao-meta"]}
        preview={preview(false)}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={null}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    expect(
      within(dialog).getByText(
        "Central 恢复证据发生冲突。请在操作日志中检查并处理待恢复操作。",
      ),
    ).toBeInTheDocument();
    expect(within(dialog).queryByTestId("force-delete-batch-central-skills")).not.toBeInTheDocument();
  });

  it("sends force true only for eligible skills", async () => {
    const onConfirm = vi.fn().mockResolvedValue({ succeeded: [], failed: [] });
    render(
      <BatchDeleteCentralSkillsDialog
        open
        onOpenChange={vi.fn()}
        skillIds={["yao-meta", "blocked"]}
        preview={{
          previews: [recoveryPreview("yao-meta", true), recoveryPreview("blocked", false)],
          failed: [],
        }}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={null}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    const forceButton = within(dialog).getByTestId("force-delete-batch-central-skills");
    fireEvent.click(forceButton);
    fireEvent.click(forceButton);

    expect(onConfirm).toHaveBeenCalledWith([
      { skill_id: "yao-meta", remove_agent_ids: [], force: true },
      { skill_id: "blocked", remove_agent_ids: [], force: false },
    ]);
  });

  it("keeps regular batch delete on force false", () => {
    const onConfirm = vi.fn().mockResolvedValue({ succeeded: [], failed: [] });
    render(
      <BatchDeleteCentralSkillsDialog
        open
        onOpenChange={vi.fn()}
        skillIds={["yao-meta"]}
        preview={preview(true)}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={null}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-batch-delete-central-skills"));
    expect(onConfirm).toHaveBeenCalledWith([
      { skill_id: "yao-meta", remove_agent_ids: [], force: false },
    ]);
  });
});
