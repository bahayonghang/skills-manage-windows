import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";

import { UpdateCenterDialog } from "@/components/central/UpdateCenterDialog";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import type { SkillUpdateInventory } from "@/types/skillUpdateInventory";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

const initialUpdateCenterState = useUpdateCenterStore.getState();
const initialCentralSkillsState = useCentralSkillsStore.getState();
const initialTargetState = useTargetStore.getState();
const initialPlatformState = usePlatformStore.getState();

function unsupportedInventory(): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    unsupported: [{ skillId: "npx-skill", reasonCode: "unknown_source" }],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-08-14T00:00:00.000Z",
  };
}

function previewResult(skillIds: string[]) {
  return {
    skillIds,
    preview: {
      previews: skillIds.map((skillId) => ({
        skill_id: skillId,
        skill_name: skillId,
        central_path: `/tmp/${skillId}`,
        copy_installations: [],
        auto_removed_agent_ids: [],
      })),
      failed: [],
    },
  };
}

function renderUnsupportedDialog(options?: {
  kind?: "local" | "ssh";
  loadPreview?: ReturnType<typeof vi.fn>;
  reset?: ReturnType<typeof vi.fn>;
}) {
  usePlatformStore.setState({
    ...initialPlatformState,
    refreshCounts: vi.fn().mockResolvedValue(undefined),
  });
  useTargetStore.setState({
    ...initialTargetState,
    activeTarget:
      options?.kind === "ssh"
        ? {
            id: "ssh-1",
            kind: "ssh",
            label: "Remote",
            isActive: true,
          }
        : {
            id: "local",
            kind: "local",
            label: "Local",
            isActive: true,
          },
  });
  useCentralSkillsStore.setState({
    ...initialCentralSkillsState,
    agents: [],
    loadUnknownSourceResetPreview: (options?.loadPreview ??
      vi.fn().mockResolvedValue(
        previewResult(["npx-skill"]),
      )) as unknown as typeof initialCentralSkillsState.loadUnknownSourceResetPreview,
    resetUnknownSourceSkills: (options?.reset ??
      vi.fn().mockResolvedValue({
        succeeded: [
          {
            skill_id: "npx-skill",
            removed_central_path: "/tmp/npx-skill",
            removed_agent_ids: [],
            retained_agent_ids: [],
          },
        ],
        failed: [],
      })) as unknown as typeof initialCentralSkillsState.resetUnknownSourceSkills,
  });
  useUpdateCenterStore.setState({
    ...initialUpdateCenterState,
    inventory: unsupportedInventory(),
    isDialogOpen: true,
    activeTab: "unsupported",
    refreshContext: { repositoryIds: [], skillIds: [], agentIds: [] },
    refreshMode: "sync",
  });
  render(<UpdateCenterDialog />);
}

describe("UpdateCenterDialog unknown-source reset", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.clearAllMocks();
    useUpdateCenterStore.setState(initialUpdateCenterState, true);
    useCentralSkillsStore.setState(initialCentralSkillsState, true);
    useTargetStore.setState(initialTargetState, true);
    usePlatformStore.setState(initialPlatformState, true);
  });

  it("shows the reset control for Local and SSH targets", () => {
    renderUnsupportedDialog({ kind: "local" });
    expect(screen.getByTestId("reset-unknown-source-skills")).toBeEnabled();
  });

  it("shows the reset control when the active target is SSH", () => {
    renderUnsupportedDialog({ kind: "ssh" });
    expect(screen.getByTestId("reset-unknown-source-skills")).toBeEnabled();
  });

  it("disables confirm when preview count is 0", async () => {
    const loadPreview = vi.fn().mockResolvedValue(previewResult([]));
    renderUnsupportedDialog({ loadPreview });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));

    const confirm = await screen.findByTestId(
      "confirm-reset-unknown-source-skills",
    );
    expect(loadPreview).toHaveBeenCalled();
    expect(confirm).toBeDisabled();
  });

  it("toasts and shows inline formatBackendError when preview is rejected", async () => {
    const loadPreview = vi.fn().mockRejectedValue(
      ipcFixtureError(
        "central.reset_failed",
        "token=ghp_secret https://github.com/private/repo C:\\secret\\path",
      ),
    );
    renderUnsupportedDialog({ loadPreview });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled();
    });
    const message = vi.mocked(toast.error).mock.calls[0]?.[0] as string;
    expect(message).toContain("无法重置缺少远端来源的中央技能");
    expect(message).not.toContain("ghp_secret");
    expect(message).not.toContain("github.com/private");
    expect(message).not.toContain("C:\\secret");

    const dialogs = screen.getAllByRole("dialog");
    const resetDialog = dialogs[dialogs.length - 1];
    expect(
      within(resetDialog).getByRole("alert").textContent,
    ).toContain("无法重置缺少远端来源的中央技能");
  });

  it("confirms reset after a successful preview", async () => {
    const reset = vi.fn().mockResolvedValue({
      succeeded: [
        {
          skill_id: "npx-skill",
          removed_central_path: "/tmp/npx-skill",
          removed_agent_ids: [],
          retained_agent_ids: [],
        },
      ],
      failed: [],
    });
    renderUnsupportedDialog({ reset });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));
    const confirm = await screen.findByTestId(
      "confirm-reset-unknown-source-skills",
    );
    fireEvent.click(confirm);

    await waitFor(() => {
      expect(reset).toHaveBeenCalledWith(["npx-skill"], []);
    });
    expect(toast.success).toHaveBeenCalled();
  });

  it("does not apply reset when the confirm dialog is cancelled", async () => {
    const reset = vi.fn();
    renderUnsupportedDialog({ reset });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));
    const confirm = await screen.findByTestId(
      "confirm-reset-unknown-source-skills",
    );
    expect(confirm).toBeEnabled();

    const resetDialog = confirm.closest("[role=\"dialog\"]");
    expect(resetDialog).toBeTruthy();
    fireEvent.click(
      within(resetDialog as HTMLElement).getByRole("button", { name: "取消" }),
    );

    await waitFor(() => {
      expect(
        screen.queryByTestId("confirm-reset-unknown-source-skills"),
      ).not.toBeInTheDocument();
    });
    expect(reset).not.toHaveBeenCalled();
  });

  it("toasts and shows inline formatBackendError when apply is rejected", async () => {
    const reset = vi.fn().mockRejectedValue(
      ipcFixtureError(
        "central.reset_failed",
        "token=ghp_secret https://github.com/private/repo C:\\secret\\path",
      ),
    );
    renderUnsupportedDialog({ reset });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));
    const confirm = await screen.findByTestId(
      "confirm-reset-unknown-source-skills",
    );
    fireEvent.click(confirm);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled();
    });
    const message = vi.mocked(toast.error).mock.calls[0]?.[0] as string;
    expect(message).toContain("无法重置缺少远端来源的中央技能");
    expect(message).not.toContain("ghp_secret");
    expect(message).not.toContain("github.com/private");
    expect(message).not.toContain("C:\\secret");

    const dialogs = screen.getAllByRole("dialog");
    const resetDialog = dialogs[dialogs.length - 1];
    expect(
      within(resetDialog).getByRole("alert").textContent,
    ).toContain("无法重置缺少远端来源的中央技能");
    expect(resetDialog).toBeInTheDocument();
  });

  it("keeps the dialog open and lists failed skill ids on partial apply", async () => {
    const reset = vi.fn().mockResolvedValue({
      succeeded: [
        {
          skill_id: "npx-skill",
          removed_central_path: "/tmp/npx-skill",
          removed_agent_ids: [],
          retained_agent_ids: [],
        },
      ],
      failed: [
        {
          skill_id: "broken-skill",
          error_code: "central_skills.delete_failed",
          error: "This Central skill could not be deleted.",
        },
      ],
    });
    renderUnsupportedDialog({ reset });

    fireEvent.click(screen.getByTestId("reset-unknown-source-skills"));
    const confirm = await screen.findByTestId(
      "confirm-reset-unknown-source-skills",
    );
    fireEvent.click(confirm);

    await waitFor(() => {
      expect(reset).toHaveBeenCalledWith(["npx-skill"], []);
    });
    expect(toast.error).toHaveBeenCalled();
    const message = vi.mocked(toast.error).mock.calls[0]?.[0] as string;
    expect(message).toContain("已删除 1 个，失败 1 个");

    const dialogs = screen.getAllByRole("dialog");
    const resetDialog = dialogs[dialogs.length - 1];
    expect(within(resetDialog).getByRole("alert").textContent).toContain(
      "broken-skill",
    );
    expect(within(resetDialog).getByRole("alert").textContent).toContain(
      "无法删除该中央技能",
    );
    expect(
      screen.getByTestId("confirm-reset-unknown-source-skills"),
    ).toBeInTheDocument();
  });
});
