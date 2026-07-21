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

function renderOpenDialog(apply: ReturnType<typeof vi.fn>) {
  useCentralSkillsStore.setState({ skills: [], repositories: [] });
  useUpdateCenterStore.setState({
    ...initialUpdateCenterState,
    inventory: inventoryWithLeftovers(),
    isDialogOpen: true,
    activeTab: "deletedPlatformCopies",
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
});
