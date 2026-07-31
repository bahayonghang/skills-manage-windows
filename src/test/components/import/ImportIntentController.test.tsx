import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { StrictMode, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";

const { mockInvoke, mockListen, listenerRef } = vi.hoisted(() => ({
  mockInvoke: vi.fn(() => Promise.resolve()),
  mockListen: vi.fn(),
  listenerRef: {
    current: null as null | ((event: { payload: unknown }) => void),
  },
}));

vi.mock("@/lib/ipc", () => ({
  invoke: mockInvoke,
  isTauriRuntime: () => true,
  listen: mockListen,
}));

import { ImportIntentController } from "@/components/import/ImportIntentController";
import { resetImportIntentListenerForTest } from "@/lib/importIntentListener";
import {
  openImportIntent,
  useImportIntentStore,
} from "@/stores/importIntentStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";

function LocationProbe() {
  const location = useLocation();
  return <output aria-label="current route">{location.pathname}</output>;
}

function TestRouter({ children }: { children?: ReactNode }) {
  return (
    <MemoryRouter initialEntries={["/marketplace"]}>
      <ImportIntentController />
      <LocationProbe />
      <Routes>
        <Route path="*" element={children ?? null} />
      </Routes>
    </MemoryRouter>
  );
}

async function emitImportIntent(payload: unknown) {
  await waitFor(() => expect(listenerRef.current).not.toBeNull());
  act(() => listenerRef.current?.({ payload }));
}

describe("ImportIntentController", () => {
  beforeEach(() => {
    resetImportIntentListenerForTest();
    listenerRef.current = null;
    mockInvoke.mockReset().mockResolvedValue(undefined);
    mockListen.mockReset().mockImplementation((_event, handler) => {
      listenerRef.current = handler;
      return Promise.resolve(() => undefined);
    });
    useImportIntentStore.getState().resetForTest();
    useMarketplaceStore.getState().resetGitHubImport();
  });

  afterEach(() => {
    cleanup();
  });

  it("registers the event listener before ready, then routes and prefills without preview or import IPC", async () => {
    render(<TestRouter />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith(
        "skillport://import-intent",
        expect.any(Function),
      );
      expect(mockInvoke).toHaveBeenCalledWith(
        "mark_import_intent_frontend_ready",
      );
    });
    expect(mockListen.mock.invocationCallOrder[0]).toBeLessThan(
      mockInvoke.mock.invocationCallOrder[0],
    );

    await emitImportIntent({ source: "https://github.com/owner/repo" });

    expect(screen.getByLabelText("current route")).toHaveTextContent("/central");
    expect(useImportIntentStore.getState()).toMatchObject({
      githubWizardOpen: true,
      githubSource: "https://github.com/owner/repo",
      githubBranch: "",
      pendingSources: [],
    });
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "preview_github_repo_import",
      expect.anything(),
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "import_github_repo_skills",
      expect.anything(),
    );
  });

  it("keeps one listener and ready handshake across StrictMode remounts", async () => {
    render(
      <StrictMode>
        <TestRouter />
      </StrictMode>,
    );

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
  });

  it("preserves a dirty wizard and lets the user consume or discard pending intents after closing", async () => {
    openImportIntent({
      kind: "github",
      source: "https://github.com/current/repo",
    });
    render(<TestRouter />);

    await emitImportIntent({ source: "https://github.com/next/repo" });

    expect(useImportIntentStore.getState()).toMatchObject({
      githubWizardOpen: true,
      githubSource: "https://github.com/current/repo",
      pendingSources: ["https://github.com/next/repo"],
    });

    act(() => useImportIntentStore.getState().setGitHubBranch("dev"));
    act(() => useImportIntentStore.getState().setGitHubWizardOpen(false));
    act(() => useImportIntentStore.getState().consumePendingIntent());
    expect(useImportIntentStore.getState()).toMatchObject({
      githubWizardOpen: true,
      githubSource: "https://github.com/next/repo",
      githubBranch: "",
      pendingSources: [],
    });

    act(() => useImportIntentStore.getState().setGitHubWizardOpen(false));
    act(() =>
      openImportIntent({
        kind: "github",
        source: "https://github.com/discard/repo",
      }),
    );
    act(() => useImportIntentStore.getState().discardPendingIntent());
    expect(useImportIntentStore.getState().pendingSources).toEqual([]);
  });

  it("deduplicates pending sources and keeps the newest eight in FIFO order", () => {
    openImportIntent({
      kind: "github",
      source: "https://github.com/current/repo",
    });

    for (let index = 1; index <= 9; index += 1) {
      openImportIntent({
        kind: "github",
        source: `https://github.com/owner/repo-${index}`,
      });
    }
    openImportIntent({
      kind: "github",
      source: "https://github.com/owner/repo-9",
    });

    expect(useImportIntentStore.getState().pendingSources).toEqual(
      Array.from(
        { length: 8 },
        (_, index) => `https://github.com/owner/repo-${index + 2}`,
      ),
    );
  });

  it("treats a manual branch as dirty session state", () => {
    act(() => useImportIntentStore.getState().setGitHubBranch("dev"));

    expect(
      openImportIntent({
        kind: "github",
        source: "https://github.com/owner/repo",
      }),
    ).toBe("pending");
    expect(useImportIntentStore.getState()).toMatchObject({
      githubSource: "",
      githubBranch: "dev",
      pendingSources: ["https://github.com/owner/repo"],
    });
  });

  it("ignores invalid event payloads without navigation or state changes", async () => {
    render(<TestRouter />);

    for (const payload of [
      null,
      {},
      { source: 42 },
      { source: "http://github.com/owner/repo" },
      { source: "https://user@github.com/owner/repo" },
      { source: "https://github.com/owner/repo?token=secret" },
      { source: "https://github.com/owner/repo\nattack" },
      { source: "https://github.com/owner/repo/%2e%2e/attack" },
      { source: "https://github.com/owner/repo/%250Aattack" },
      {
        source: "https://github.com/owner/repo",
        token: "super-secret",
      },
    ]) {
      await emitImportIntent(payload);
    }

    expect(screen.getByLabelText("current route")).toHaveTextContent(
      "/marketplace",
    );
    expect(useImportIntentStore.getState()).toMatchObject({
      githubWizardOpen: false,
      githubSource: "",
      githubBranch: "",
      pendingSources: [],
    });
  });
});
