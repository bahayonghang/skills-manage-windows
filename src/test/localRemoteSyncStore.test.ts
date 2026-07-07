import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/ipc";
import { useLocalRemoteSyncStore } from "@/stores/localRemoteSyncStore";
import type { LocalRemoteSyncPreview } from "@/types";

const preview: LocalRemoteSyncPreview = {
  targetId: "ssh-1",
  targetLabel: "Lab",
  repoRoot: "D:/repo",
  skillsRoot: "C:/Users/alice/.skillsmanage/skills",
  repo: {
    id: "repo",
    label: "repo",
    kind: "repo",
    localPath: "D:/repo",
    remotePath: "/home/alice/.skillsmanage/repos/repo",
    fileCount: 1,
    byteCount: 10,
    localHash: "sha256-manifest:local",
    remoteHash: null,
    status: "add",
  },
  skills: [],
  totalFileCount: 1,
  totalByteCount: 10,
};

describe("localRemoteSyncStore", () => {
  beforeEach(() => {
    useLocalRemoteSyncStore.setState({
      preview: null,
      result: null,
      isPreviewing: false,
      isApplying: false,
      error: null,
    });
    vi.clearAllMocks();
  });

  it("previews sync through the backend command and stores the preview", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    await useLocalRemoteSyncStore.getState().previewSync({ targetId: "ssh-1" });

    expect(invoke).toHaveBeenCalledWith("preview_local_remote_sync", {
      request: { targetId: "ssh-1" },
    });
    expect(useLocalRemoteSyncStore.getState().preview).toEqual(preview);
    expect(useLocalRemoteSyncStore.getState().isPreviewing).toBe(false);
  });

  it("stores preview errors and clears loading state", async () => {
    vi.mocked(invoke).mockRejectedValueOnce("connection failed");

    await expect(
      useLocalRemoteSyncStore.getState().previewSync({ targetId: "ssh-1" })
    ).rejects.toBe("connection failed");

    expect(useLocalRemoteSyncStore.getState().error).toBe("connection failed");
    expect(useLocalRemoteSyncStore.getState().isPreviewing).toBe(false);
  });

  it("applies sync through the backend command and stores the result", async () => {
    const result = {
      targetId: "ssh-1",
      targetLabel: "Lab",
      syncedRepo: preview.repo,
      syncedSkills: [],
      skippedSkills: [],
      failed: [],
    };
    vi.mocked(invoke).mockResolvedValueOnce(result);

    await useLocalRemoteSyncStore.getState().applySync({ targetId: "ssh-1" });

    expect(invoke).toHaveBeenCalledWith("apply_local_remote_sync", {
      request: { targetId: "ssh-1" },
    });
    expect(useLocalRemoteSyncStore.getState().result).toEqual(result);
    expect(useLocalRemoteSyncStore.getState().isApplying).toBe(false);
  });
});
