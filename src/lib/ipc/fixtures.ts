import type { CommandArgs, CommandResult, IpcCommandMap } from "./commandMap";

/** 浏览器演示态下，某命令没有注册 fixture 时的显式失败（fail loud，缺口可定位）。 */
export class IpcFixtureMissingError extends Error {
  constructor(command: string) {
    super(`[ipc] no browser fixture registered for command "${command}"`);
    this.name = "IpcFixtureMissingError";
  }
}

type AnyFixtureHandler = (args: unknown) => unknown;

/** 已类型化命令的 fixture handler：参数与返回值跟随 IpcCommandMap 推导。 */
export type IpcFixtureHandlers = {
  [K in keyof IpcCommandMap]?: (
    args: CommandArgs<K>,
  ) => CommandResult<K> | Promise<CommandResult<K>>;
};

const fixtureRegistry = new Map<string, AnyFixtureHandler>();

export function registerIpcFixtures(handlers: IpcFixtureHandlers): void {
  for (const [command, handler] of Object.entries(handlers)) {
    fixtureRegistry.set(command, handler as AnyFixtureHandler);
  }
}

/** 命令尚未入 IPC_COMMANDS 时的逃生口；优先类型化命令后改用 registerIpcFixtures。 */
export function registerUntypedIpcFixture(
  command: string,
  handler: (args: unknown) => unknown,
): void {
  fixtureRegistry.set(command, handler);
}

export function hasIpcFixture(command: string): boolean {
  return fixtureRegistry.has(command);
}

export function dispatchIpcFixture<T>(
  command: string,
  args: unknown,
): Promise<T> {
  const handler = fixtureRegistry.get(command);
  if (!handler) {
    return Promise.reject(new IpcFixtureMissingError(command));
  }
  return Promise.resolve().then(() => handler(args)) as Promise<T>;
}

export function clearIpcFixturesForTest(): void {
  fixtureRegistry.clear();
}
