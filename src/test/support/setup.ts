import "@testing-library/jest-dom";
import zh from "@/i18n/locales/zh.json";
import { __dispatchMockIpc, resetIpcMock } from "./ipcMock";

// ─── react-i18next mock ───────────────────────────────────────────────────────
// Resolves translation keys against zh.json so existing assertions on Chinese
// strings continue to pass without requiring a running i18n instance.

type TranslationObj = { [key: string]: TranslationObj | string };

function resolveKey(
  obj: TranslationObj,
  key: string,
  options?: Record<string, unknown>,
): string {
  const defaultValue =
    typeof options?.defaultValue === "string"
      ? options.defaultValue
      : undefined;
  const parts = key.split(".");
  let result: TranslationObj | string = obj;
  for (const part of parts) {
    if (result && typeof result === "object") {
      result = (result as TranslationObj)[part];
    } else {
      return defaultValue ?? key;
    }
  }
  if (typeof result !== "string") return defaultValue ?? key;
  // Handle simple {{var}} interpolation
  if (options) {
    return result.replace(/\{\{(\w+)\}\}/g, (_match, varName) => {
      const val = options[varName];
      return val !== undefined ? String(val) : `{{${varName}}}`;
    });
  }
  return result;
}

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      resolveKey(zh as unknown as TranslationObj, key, options),
    i18n: {
      changeLanguage: vi.fn(),
      language: "zh",
    },
  }),
  initReactI18next: { type: "3rdParty", init: vi.fn() },
  Trans: ({ children }: { children: React.ReactNode }) => children,
}));

// Polyfill PointerEvent for base-ui components in jsdom
// base-ui's Checkbox/Radio use PointerEvent internally which jsdom doesn't support
if (!globalThis.PointerEvent) {
  class TestPointerEvent extends MouseEvent {
    pointerId: number;
    width: number;
    height: number;
    pressure: number;
    tangentialPressure: number;
    tiltX: number;
    tiltY: number;
    twist: number;
    pointerType: string;
    isPrimary: boolean;

    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
      this.width = init.width ?? 1;
      this.height = init.height ?? 1;
      this.pressure = init.pressure ?? 0;
      this.tangentialPressure = init.tangentialPressure ?? 0;
      this.tiltX = init.tiltX ?? 0;
      this.tiltY = init.tiltY ?? 0;
      this.twist = init.twist ?? 0;
      this.pointerType = init.pointerType ?? "";
      this.isPrimary = init.isPrimary ?? false;
    }
  }

  Object.defineProperty(globalThis, "PointerEvent", {
    value: TestPointerEvent,
    configurable: true,
  });
}

if (!window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

// Polyfill ResizeObserver for cmdk / Radix popovers in jsdom.
if (!(globalThis as { ResizeObserver?: unknown }).ResizeObserver) {
  class TestResizeObserver {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  Object.defineProperty(globalThis, "ResizeObserver", {
    value: TestResizeObserver,
    configurable: true,
    writable: true,
  });
}

// Polyfill Element.scrollIntoView for cmdk's auto-scroll-to-selected behavior.
if (typeof Element !== "undefined" && !Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = function scrollIntoView(): void {};
}

function createTestStorage(): Storage {
  const values = new Map<string, string>();

  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key: string) {
      return values.has(key) ? values.get(key)! : null;
    },
    key(index: number) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key: string) {
      values.delete(key);
    },
    setItem(key: string, value: string) {
      values.set(key, String(value));
    },
  };
}

const testLocalStorage = createTestStorage();

Object.defineProperty(window, "localStorage", {
  value: testLocalStorage,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "localStorage", {
  value: testLocalStorage,
  configurable: true,
  writable: true,
});

// Mock Tauri APIs for testing
Object.defineProperty(window, "__TAURI__", {
  value: {
    core: {
      invoke: vi.fn(),
    },
  },
  configurable: true,
  writable: true,
});

Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    // 命令路由 dispatcher（普通函数而非 vi.fn，免疫 vi.resetAllMocks）：
    // 语义见 src/test/support/ipcMock.ts —— 宽松模式兼容存量测试，注册后按命令名严格路由
    invoke: (command: string, args?: unknown) =>
      __dispatchMockIpc(command, args),
    transformCallback: vi.fn(),
    postMessage: vi.fn(),
  },
  configurable: true,
  writable: true,
});

afterEach(() => {
  resetIpcMock();
});
