import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauriBridge from "@/lib/tauri";
import {
  formatAppUpdateBytes,
  getAppUpdateProgressPercent,
  useAppUpdateStore,
} from "@/stores/appUpdateStore";

const updaterMocks = vi.hoisted(() => ({
  check: vi.fn(),
}));

const processMocks = vi.hoisted(() => ({
  relaunch: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: updaterMocks.check,
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: processMocks.relaunch,
}));

describe("useAppUpdateStore", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    updaterMocks.check.mockReset();
    processMocks.relaunch.mockReset();
    vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(true);
    useAppUpdateStore.getState().reset();
  });

  it("marks browser and test environments as unsupported", async () => {
    vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useAppUpdateStore.getState().checkForUpdate();

    expect(useAppUpdateStore.getState().status).toBe("unsupported");
    expect(useAppUpdateStore.getState().error).toBeNull();
    expect(updaterMocks.check).not.toHaveBeenCalled();
  });

  it("sets upToDate when check returns null", async () => {
    updaterMocks.check.mockResolvedValueOnce(null);

    await useAppUpdateStore.getState().checkForUpdate();

    expect(updaterMocks.check).toHaveBeenCalledOnce();
    expect(useAppUpdateStore.getState()).toMatchObject({
      status: "upToDate",
      latestVersion: null,
      releaseNotes: null,
      hasChecked: true,
    });
  });

  it("stores update metadata when an update is available", async () => {
    updaterMocks.check.mockResolvedValueOnce({
      currentVersion: "0.10.7",
      version: "0.10.8",
      body: "Bug fixes",
      downloadAndInstall: vi.fn(),
    });

    await useAppUpdateStore.getState().checkForUpdate();

    expect(useAppUpdateStore.getState()).toMatchObject({
      status: "available",
      currentVersion: "0.10.7",
      latestVersion: "0.10.8",
      releaseNotes: "Bug fixes",
    });
  });

  it("accumulates download progress and relaunches after install", async () => {
    const downloadAndInstall = vi.fn(async (onEvent?: (event: unknown) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: 100 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 25 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 50 } });
      onEvent?.({ event: "Finished" });
    });
    updaterMocks.check.mockResolvedValueOnce({
      currentVersion: "0.10.7",
      version: "0.10.8",
      body: "Bug fixes",
      downloadAndInstall,
    });

    await useAppUpdateStore.getState().checkForUpdate();
    await useAppUpdateStore.getState().installUpdate();

    expect(downloadAndInstall).toHaveBeenCalledOnce();
    expect(processMocks.relaunch).toHaveBeenCalledOnce();
    expect(useAppUpdateStore.getState()).toMatchObject({
      status: "installing",
      progress: { downloaded: 100, total: 100 },
    });
  });

  it("stores errors from updater failures", async () => {
    updaterMocks.check.mockRejectedValueOnce(new Error("metadata missing"));

    await useAppUpdateStore.getState().checkForUpdate();

    expect(useAppUpdateStore.getState()).toMatchObject({
      status: "error",
      error: "metadata missing",
      hasChecked: true,
    });
  });

  it("clears stale update metadata before a new check", async () => {
    updaterMocks.check
      .mockResolvedValueOnce({
        currentVersion: "0.10.7",
        version: "0.10.8",
        body: "Bug fixes",
        downloadAndInstall: vi.fn(),
      })
      .mockRejectedValueOnce(new Error("metadata missing"));

    await useAppUpdateStore.getState().checkForUpdate();
    await useAppUpdateStore.getState().checkForUpdate();

    expect(useAppUpdateStore.getState()).toMatchObject({
      status: "error",
      latestVersion: null,
      releaseNotes: null,
      error: "metadata missing",
    });
  });

  it("formats progress helpers", () => {
    expect(getAppUpdateProgressPercent({ downloaded: 64, total: 128 })).toBe(50);
    expect(getAppUpdateProgressPercent({ downloaded: 64, total: null })).toBe(0);
    expect(formatAppUpdateBytes(1536)).toBe("1.5 KB");
    expect(formatAppUpdateBytes(0)).toBe("0 B");
    expect(formatAppUpdateBytes(null)).toBe("—");
  });
});
