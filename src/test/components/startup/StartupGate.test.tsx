import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { StartupGate } from "@/components/startup/StartupGate";
import {
  __resetMainWindowReadyForTest,
} from "@/lib/ipc";
import {
  resetStartupStoreForTests,
  useStartupStore,
} from "@/stores/startupStore";
import { ipcInvokeCalls, mockIpcCommand, mockIpcCommands } from "@/test/support/ipcMock";

const showWindow = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ show: showWindow }),
}));

const RECOVERY_STATUS = {
  phase: "recovery_required" as const,
  issue: "database_open_failed" as const,
  diagnostic: "corrupt" as const,
  canRebuild: true,
  backupCreated: false,
};

describe("StartupGate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __resetMainWindowReadyForTest();
    resetStartupStoreForTests();
  });

  it("shows the hidden window and mounts the app only after ready", async () => {
    mockIpcCommand("get_startup_status", { phase: "ready" });

    render(
      <StartupGate>
        <div data-testid="app-shell">app</div>
      </StartupGate>,
    );

    expect(screen.queryByTestId("app-shell")).not.toBeInTheDocument();
    expect(await screen.findByTestId("app-shell")).toBeInTheDocument();
    await waitFor(() => expect(showWindow).toHaveBeenCalledTimes(1));
  });

  it("keeps the app unmounted in recovery and enters it after retry", async () => {
    mockIpcCommands({
      get_startup_status: RECOVERY_STATUS,
      retry_startup: { phase: "ready" },
    });

    render(
      <StartupGate>
        <div data-testid="app-shell">app</div>
      </StartupGate>,
    );

    expect(await screen.findByRole("heading", { name: "本地数据需要恢复" })).toBeInTheDocument();
    expect(screen.queryByTestId("app-shell")).not.toBeInTheDocument();
    expect(screen.getByText("完整性检查表明数据库文件已损坏。")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "重新检查" }));

    expect(await screen.findByTestId("app-shell")).toBeInTheDocument();
    expect(ipcInvokeCalls("retry_startup")).toHaveLength(1);
  });

  it("offers no rebuild for fatal directory failures", async () => {
    mockIpcCommand("get_startup_status", {
      phase: "fatal",
      issue: "data_directory_unavailable",
    });

    render(<StartupGate><div>app</div></StartupGate>);

    expect(await screen.findByRole("heading", { name: "SkillPort 无法完成启动" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重新检查" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "备份并重建数据库" })).not.toBeInTheDocument();
  });

  it("shows a safe inline error without rendering the rejected IPC message", async () => {
    mockIpcCommands({
      get_startup_status: RECOVERY_STATUS,
      retry_startup: () => {
        throw new Error("C:\\private\\db.sqlite sqlx raw failure");
      },
    });

    render(<StartupGate><div>app</div></StartupGate>);
    fireEvent.click(await screen.findByRole("button", { name: "重新检查" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("操作未完成");
    expect(document.body.textContent).not.toContain("db.sqlite");
    expect(document.body.textContent).not.toContain("sqlx");
  });

  it("disables both mutations while rebuild is active", async () => {
    let resolveRebuild!: (value: { phase: "ready" }) => void;
    mockIpcCommands({
      get_startup_status: RECOVERY_STATUS,
      rebuild_startup_database: () =>
        new Promise((resolve) => {
          resolveRebuild = resolve;
        }),
    });

    render(<StartupGate><div data-testid="app-shell">app</div></StartupGate>);
    const rebuild = await screen.findByRole("button", { name: "备份并重建数据库" });
    fireEvent.click(rebuild);

    expect(screen.getByRole("button", { name: "重新检查" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "正在备份并重建" })).toBeDisabled();
    expect(ipcInvokeCalls("rebuild_startup_database")).toHaveLength(1);

    resolveRebuild({ phase: "ready" });
    expect(await screen.findByTestId("app-shell")).toBeInTheDocument();
  });

  it("shows a safe fatal surface when initial status loading fails", async () => {
    mockIpcCommand("get_startup_status", () => {
      throw new Error("raw startup error");
    });

    render(<StartupGate><div data-testid="app-shell">app</div></StartupGate>);

    expect(await screen.findByText(/无法读取启动状态/)).toBeInTheDocument();
    expect(screen.queryByTestId("app-shell")).not.toBeInTheDocument();
    expect(useStartupStore.getState().actionError).toBe("load");
    await waitFor(() => expect(showWindow).toHaveBeenCalledTimes(1));
  });
});
