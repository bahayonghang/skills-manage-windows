import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";

import { UpdateCenterDialog } from "@/components/central/UpdateCenterDialog";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import type {
  SkillUpdateApplyResult,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

const initialUpdateCenterState = useUpdateCenterStore.getState();
const initialCentralSkillsState = useCentralSkillsStore.getState();

function inventoryWithLeftovers(): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    deletedPlatformCopies: [
      {
        agentId: "codex",
        skillId: "removed-skill",
        skillName: "removed-skill",
        writablePaths: [
          "C:\\Users\\lyh\\.agents\\skills\\removed-skill",
          "C:\\Users\\lyh\\.agents\\skills\\removed-skill-copy",
        ],
      },
    ],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-07-08T00:00:00.000Z",
  };
}

function inventoryWithUpdate(): SkillUpdateInventory {
  return {
    ...inventoryWithLeftovers(),
    updatable: [
      {
        state: {
          skill_id: "skill-a",
          source_type: "github",
          source_url: "https://github.com/owner/repo",
          source_path: "skills/skill-a",
          status: "update_available",
        },
        repositoryId: "owner/repo",
      },
    ],
    deletedPlatformCopies: [],
  };
}

function applyResult(
  overrides: Partial<SkillUpdateApplyResult> = {},
): SkillUpdateApplyResult {
  return {
    updatedSkillIds: [],
    keptMissingSkillIds: [],
    deletedSkillIds: [],
    importedSkillIds: [],
    skippedAdditions: [],
    unskippedAdditions: [],
    removedPlatformDuplicatePaths: [],
    removedDeletedPlatformCopyPaths: [],
    failures: [],
    ...overrides,
  };
}

function renderOpenDialog(
  apply: ReturnType<typeof vi.fn>,
  inventory: SkillUpdateInventory = inventoryWithLeftovers(),
  activeTab: "updatable" | "deletedPlatformCopies" = "deletedPlatformCopies",
) {
  useCentralSkillsStore.setState({ skills: [], repositories: [] });
  useUpdateCenterStore.setState({
    ...initialUpdateCenterState,
    inventory,
    isDialogOpen: true,
    activeTab,
    refreshContext: { repositoryIds: [], skillIds: [], agentIds: [] },
    refreshMode: "sync",
    apply: apply as unknown as typeof initialUpdateCenterState.apply,
  });
  render(<UpdateCenterDialog />);
}

describe("UpdateCenterDialog platform leftover cleanup", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.clearAllMocks();
    useUpdateCenterStore.setState(initialUpdateCenterState, true);
    useCentralSkillsStore.setState(initialCentralSkillsState, true);
  });

  it("does not apply cleanup when confirmation is cancelled", async () => {
    const apply = vi.fn().mockResolvedValue(applyResult());
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    renderOpenDialog(apply);

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "清理残留（2）" }),
    );

    expect(confirm).toHaveBeenCalledWith(
      expect.stringContaining("2 条平台残留路径"),
    );
    expect(apply).not.toHaveBeenCalled();
  });

  it("applies only platform leftover removals after confirmation", async () => {
    const apply = vi.fn().mockResolvedValue(
      applyResult({
        removedDeletedPlatformCopyPaths: [
          "C:\\Users\\lyh\\.agents\\skills\\removed-skill",
          "C:\\Users\\lyh\\.agents\\skills\\removed-skill-copy",
        ],
      }),
    );
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderOpenDialog(apply);

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "清理残留（2）" }),
    );

    await waitFor(() => expect(apply).toHaveBeenCalledTimes(1));
    expect(apply).toHaveBeenCalledWith(
      {
        allowedAgentIds: null,
        updates: [],
        keepMissing: [],
        deleteMissing: [],
        importAdditions: [],
        skipAdditions: [],
        unskipAdditions: [],
        removePlatformDuplicates: [],
        removeDeletedPlatformCopies: [
          {
            agentId: "codex",
            skillId: "removed-skill",
            paths: [
              "C:\\Users\\lyh\\.agents\\skills\\removed-skill",
              "C:\\Users\\lyh\\.agents\\skills\\removed-skill-copy",
            ],
          },
        ],
      },
      { kind: "all", mode: "sync" },
    );
    expect(toast.success).toHaveBeenCalledWith("已清理 2 条平台残留路径");
  });

  it("shows the safe identifier and reviewed recovery error for cleanup failures", async () => {
    const apply = vi.fn().mockResolvedValue(
      applyResult({
        failures: [
          {
            step: "remove_deleted_platform_copy",
            identifier: "codex::removed-skill",
            error: "token=secret https://example.invalid C:\\Users\\private",
            errorCode: "central_operation.delete_restore_collision",
            errorCategory: "central_updates.central_operation",
          },
        ],
      }),
    );
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderOpenDialog(apply);

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "清理残留（2）" }),
    );

    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "codex::removed-skill：Central 恢复证据发生冲突。请在操作日志中检查并处理待恢复操作。",
      ),
    );
    const messages = vi.mocked(toast.error).mock.calls.flat().join("\n");
    expect(messages).not.toContain("secret");
    expect(messages).not.toContain("example.invalid");
    expect(messages).not.toContain("Users\\private");
  });

  it("uses the same reviewed identifier feedback for selected updates", async () => {
    const apply = vi.fn().mockResolvedValue(
      applyResult({
        failures: [
          {
            step: "update",
            identifier: "skill-a",
            phase: "recovery",
            error: "token=secret https://example.invalid C:\\Users\\private",
            errorCode: "central_operation.delete_restore_collision",
            errorCategory: "central_updates.central_operation",
          },
        ],
      }),
    );
    renderOpenDialog(apply, inventoryWithUpdate(), "updatable");

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "应用已选项 (1)" }),
    );

    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "skill-a：Central 恢复证据发生冲突。请在操作日志中检查并处理待恢复操作。",
      ),
    );
    const messages = vi.mocked(toast.error).mock.calls.flat().join("\n");
    expect(messages).not.toContain("secret");
    expect(messages).not.toContain("example.invalid");
    expect(messages).not.toContain("Users\\private");
  });

  it("shows the localized GitHub import sentence for apply failures", async () => {
    const apply = vi.fn().mockResolvedValue(
      applyResult({
        failures: [
          {
            step: "import_addition",
            identifier: "github:emilkowalski-skill-main",
            phase: "decision_apply",
            error: "token=secret https://example.invalid C:\\Users\\private",
            errorCode: "github_import.access_denied",
            errorCategory: "github_import.access_denied",
          },
        ],
      }),
    );
    renderOpenDialog(apply, inventoryWithUpdate(), "updatable");

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "应用已选项 (1)" }),
    );

    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "github:emilkowalski-skill-main：GitHub 拒绝访问该仓库。请确认令牌具备读取权限。",
      ),
    );
    const messages = vi.mocked(toast.error).mock.calls.flat().join("\n");
    expect(messages).not.toContain("secret");
    expect(messages).not.toContain("example.invalid");
    expect(messages).not.toContain("Users\\private");
  });
});
