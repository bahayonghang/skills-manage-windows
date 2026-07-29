/**
 * 按命令名路由的 IPC mock（安装在 setup.ts 的 __TAURI_INTERNALS__.invoke 上）。
 *
 * - 未调用任何 mockIpcCommand* 时为宽松模式：未注册命令 resolve undefined，
 *   与旧版裸 vi.fn() 行为一致，存量顺序桩测试零影响。
 * - 一旦注册过 handler 即进入严格模式（按测试隔离，afterEach 自动复位）：
 *   未注册命令 reject 并列出已注册命令名，避免 store 新增 invoke 时静默通过。
 * - 断言用 ipcInvokeCalls / ipcInvokedCommands，按命令名取调用而非按调用次序打桩。
 */

type IpcMockHandler = (args: unknown) => unknown;

export interface IpcInvokeCall {
  command: string;
  args: unknown;
}

const handlers = new Map<string, IpcMockHandler>();
const calls: IpcInvokeCall[] = [];
let strict = false;

/** value 为函数时按 (args) => result 调用（可返回 Promise）；否则作为静态返回值。 */
export function mockIpcCommand(command: string, handler: unknown): void {
  strict = true;
  handlers.set(
    command,
    typeof handler === "function" ? (handler as IpcMockHandler) : () => handler,
  );
}

export function mockIpcCommands(map: Record<string, unknown>): void {
  for (const [command, handler] of Object.entries(map)) {
    mockIpcCommand(command, handler);
  }
}

/** 不传 command 返回全部调用记录（按发生顺序）；传入则过滤该命令。 */
export function ipcInvokeCalls(command?: string): IpcInvokeCall[] {
  if (command === undefined) {
    return [...calls];
  }
  return calls.filter((call) => call.command === command);
}

/** 按序返回全部命令名，供顺序敏感断言使用。 */
export function ipcInvokedCommands(): string[] {
  return calls.map((call) => call.command);
}

export function resetIpcMock(): void {
  handlers.clear();
  calls.length = 0;
  strict = false;
}

/** 仅供 setup.ts 安装到 __TAURI_INTERNALS__.invoke；测试代码不要直接调用。 */
export function __dispatchMockIpc(
  command: string,
  args?: unknown,
): Promise<unknown> {
  calls.push({ command, args });
  const handler = handlers.get(command);
  if (handler) {
    // 同步调用（与真实 invoke/旧版 vi.fn 一致）：测试里 handler 内捕获 resolve
    // 的模式依赖它在 invoke 当刻执行；同步抛错统一转为 rejection
    try {
      return Promise.resolve(handler(args));
    } catch (error) {
      return Promise.reject(error);
    }
  }
  if (strict) {
    const registered = [...handlers.keys()].join(", ") || "(none)";
    return Promise.reject(
      new Error(
        `[ipcMock] no handler registered for command "${command}" (registered: ${registered})`,
      ),
    );
  }
  return Promise.resolve(undefined);
}
