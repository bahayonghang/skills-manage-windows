import { registerIpcFixtures } from "@/lib/ipc";

const READY_STATUS = { phase: "ready" } as const;

export function registerStartupFixtures(): void {
  registerIpcFixtures({
    get_startup_status: () => READY_STATUS,
    retry_startup: () => READY_STATUS,
    rebuild_startup_database: () => READY_STATUS,
    exit_startup: () => undefined,
  });
}
