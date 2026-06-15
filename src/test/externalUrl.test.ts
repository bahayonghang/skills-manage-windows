import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockShellOpen = vi.hoisted(() => vi.fn());
const mockIsTauriRuntime = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: mockShellOpen,
}));

vi.mock("@/lib/tauri", () => ({
  isTauriRuntime: mockIsTauriRuntime,
}));

import { openExternalUrl } from "@/lib/externalUrl";

describe("openExternalUrl", () => {
  let windowOpenSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    windowOpenSpy = vi.spyOn(window, "open").mockImplementation(() => null);
  });

  afterEach(() => {
    windowOpenSpy.mockRestore();
  });

  it("opens http URLs with the Tauri shell in desktop runtime", async () => {
    mockIsTauriRuntime.mockReturnValue(true);

    await openExternalUrl("https://github.com/openai/skills");

    expect(mockShellOpen).toHaveBeenCalledWith("https://github.com/openai/skills");
    expect(windowOpenSpy).not.toHaveBeenCalled();
  });

  it("falls back to window.open outside Tauri", async () => {
    mockIsTauriRuntime.mockReturnValue(false);

    await openExternalUrl("https://github.com/openai/skills");

    expect(windowOpenSpy).toHaveBeenCalledWith(
      "https://github.com/openai/skills",
      "_blank",
      "noopener,noreferrer"
    );
    expect(mockShellOpen).not.toHaveBeenCalled();
  });

  it("rejects non-http URLs", async () => {
    mockIsTauriRuntime.mockReturnValue(true);

    await expect(openExternalUrl("mailto:security@example.com")).rejects.toThrow(
      "http(s)"
    );
    expect(mockShellOpen).not.toHaveBeenCalled();
    expect(windowOpenSpy).not.toHaveBeenCalled();
  });
});
