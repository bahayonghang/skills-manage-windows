import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DeleteCentralSkillDialog } from "@/components/central/DeleteCentralSkillDialog";
import type {
  AgentWithStatus,
  DeleteCentralSkillPreview,
  SkillWithLinks,
} from "@/types";

const agents: AgentWithStatus[] = [
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

const skill: SkillWithLinks = {
  id: "yao-meta",
  name: "yao-meta",
  description: "Yao Meta",
  file_path: "C:\\Users\\alice\\.skillsmanage\\skills\\yao-meta\\SKILL.md",
  canonical_path: "C:\\Users\\alice\\.skillsmanage\\skills\\yao-meta",
  is_central: true,
  scanned_at: "2026-08-05T00:00:00Z",
  linked_agents: ["claude-code"],
  shared_root_agents: [],
};

function preview(
  overrides: Partial<DeleteCentralSkillPreview> = {},
): DeleteCentralSkillPreview {
  return {
    skill_id: "yao-meta",
    skill_name: "yao-meta",
    central_path: "~/.skillsmanage/skills/yao-meta",
    copy_installations: [
      {
        skill_id: "yao-meta",
        agent_id: "cursor",
        installed_path: "~/.cursor/skills/yao-meta",
        link_type: "copy",
        installed_at: "2026-08-05T00:00:00Z",
      },
    ],
    auto_removed_agent_ids: ["claude-code"],
    ...overrides,
  };
}

describe("DeleteCentralSkillDialog", () => {
  const onConfirm = vi.fn().mockResolvedValue(undefined);
  const onOpenChange = vi.fn();

  beforeEach(() => {
    onConfirm.mockClear();
    onOpenChange.mockClear();
  });

  it("shows recovery copy and hides the force button when not eligible", () => {
    render(
      <DeleteCentralSkillDialog
        open
        onOpenChange={onOpenChange}
        skill={skill}
        preview={preview({
          pending_recovery: {
            operation_id: "op-1",
            operation_kind: "central_delete",
            phase: "fs_staged",
            error_code: "central_operation.delete_restore_collision",
            force_delete_eligible: false,
            blocker_codes: ["recovery.reconcile_unsupported_phase"],
          },
        })}
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
    expect(
      within(dialog).getByText("当前无法强制删除。请在操作日志中处理该恢复记录。"),
    ).toBeInTheDocument();
    expect(within(dialog).queryByTestId("force-delete-central-skill")).not.toBeInTheDocument();
  });

  it("shows the force button only when eligible and confirms with force true", async () => {
    render(
      <DeleteCentralSkillDialog
        open
        onOpenChange={onOpenChange}
        skill={skill}
        preview={preview({
          pending_recovery: {
            operation_id: "op-1",
            operation_kind: "central_delete",
            phase: "prepared",
            error_code: "central_operation.delete_restore_collision",
            force_delete_eligible: true,
            blocker_codes: [],
          },
        })}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={null}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    const forceButton = within(dialog).getByTestId("force-delete-central-skill");
    fireEvent.click(forceButton);
    expect(
      within(dialog).getByText(
        "这将放弃待恢复记录并删除当前中央副本。链接安装仍会清理。未勾选的独立副本会保留。",
      ),
    ).toBeInTheDocument();
    expect(onConfirm).not.toHaveBeenCalled();

    fireEvent.click(forceButton);
    expect(onConfirm).toHaveBeenCalledWith("yao-meta", [], true);
  });

  it("keeps regular delete on force false", () => {
    render(
      <DeleteCentralSkillDialog
        open
        onOpenChange={onOpenChange}
        skill={skill}
        preview={preview({
          pending_recovery: {
            operation_id: "op-1",
            operation_kind: "central_delete",
            phase: "prepared",
            error_code: "central_operation.delete_restore_collision",
            force_delete_eligible: true,
            blocker_codes: [],
          },
        })}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={null}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /删除中央技能/i }));
    expect(onConfirm).toHaveBeenCalledWith("yao-meta", [], false);
  });

  it("renders a reviewed collision error without path token or manifest", () => {
    const message = "Central 恢复证据发生冲突。请在操作日志中检查并处理待恢复操作。";
    const leaked = "C:\\Users\\alice\\.skillsmanage\\skills\\yao-meta ghp_secret manifest_json";

    render(
      <DeleteCentralSkillDialog
        open
        onOpenChange={onOpenChange}
        skill={skill}
        preview={preview({
          pending_recovery: {
            operation_id: "op-1",
            operation_kind: "central_delete",
            phase: "prepared",
            error_code: "central_operation.delete_restore_collision",
            force_delete_eligible: true,
            blocker_codes: [],
          },
        })}
        agents={agents}
        isPreviewLoading={false}
        isDeleting={false}
        error={message}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("dialog");
    const alert = within(dialog).getByRole("alert");
    expect(alert).toHaveTextContent(
      "Central 恢复证据发生冲突。请在操作日志中检查并处理待恢复操作。",
    );
    expect(alert).not.toHaveTextContent(leaked);
    expect(alert).not.toHaveTextContent("ghp_secret");
    expect(alert).not.toHaveTextContent("manifest_json");
    expect(alert).not.toHaveTextContent("See runtime logs");
  });
});
