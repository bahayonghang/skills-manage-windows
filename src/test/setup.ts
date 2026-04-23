import "@testing-library/jest-dom";
import zh from "../i18n/locales/zh.json";

// ─── react-i18next mock ───────────────────────────────────────────────────────
// Resolves translation keys against zh.json so existing assertions on Chinese
// strings continue to pass without requiring a running i18n instance.

type TranslationObj = { [key: string]: TranslationObj | string };

function resolveKey(obj: TranslationObj, key: string, options?: Record<string, unknown>): string {
  const defaultValue =
    typeof options?.defaultValue === "string" ? options.defaultValue : undefined;
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
if (!global.PointerEvent) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (global as any).PointerEvent = class PointerEvent extends MouseEvent {
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
  };
}

// Mock Tauri APIs for testing
Object.defineProperty(window, "__TAURI__", {
  value: {
    core: {
      invoke: vi.fn(),
    },
  },
  writable: true,
});

Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    invoke: vi.fn(),
    transformCallback: vi.fn(),
    postMessage: vi.fn(),
  },
  writable: true,
});
