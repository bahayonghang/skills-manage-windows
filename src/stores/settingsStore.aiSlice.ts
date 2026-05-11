import { invoke, isTauriRuntime } from "@/lib/tauri";
import { AI_PROVIDERS, type RegionId } from "@/data/aiProviders";
import { parseBackendError } from "@/lib/backendError";
import type { AiApiKeyState } from "@/types";

export type AiSaveStatus = "idle" | "saving" | "saved" | "error";

export interface AiSettings {
  provider: string;
  region: RegionId;
  apiKey: string;
  model: string;
  customUrl: string;
  tagConcurrency: string;
  tagIntervalMs: string;
  tagStopOnRateLimit: boolean;
}

export interface AiConnectionTestResult {
  ok: boolean;
  msg: string;
  code?: string;
  details?: string;
}

export interface AiSettingsSliceState {
  aiSettings: AiSettings;
  aiApiKeyState: AiApiKeyState;
  aiSettingsLoaded: boolean;
  isLoadingAiSettings: boolean;
  aiSaveStatus: AiSaveStatus;
  aiSaveError: string | null;
  aiTesting: boolean;
  aiTestResult: AiConnectionTestResult | null;
}

export interface AiSettingsSliceActions {
  loadAiSettings: () => Promise<void>;
  updateAiSettings: (patch: Partial<AiSettings>) => void;
  clearAiApiKey: () => Promise<void>;
  flushAiSettings: () => Promise<void>;
  testAiConnection: () => Promise<AiConnectionTestResult>;
}

export type AiSettingsSlice = AiSettingsSliceState & AiSettingsSliceActions;

export const DEFAULT_AI_SETTINGS: AiSettings = {
  provider: "claude",
  region: "intl",
  apiKey: "",
  model: "",
  customUrl: "",
  tagConcurrency: "1",
  tagIntervalMs: "4000",
  tagStopOnRateLimit: true,
};

const AI_SETTING_KEYS = [
  "ai_provider",
  "ai_region",
  "ai_model",
  "ai_api_url",
  "ai_tag_concurrency",
  "ai_tag_interval_ms",
  "ai_tag_stop_on_rate_limit",
];

const AI_SAVE_DEBOUNCE_MS = 800;


const BROWSER_FIXTURE_AI_API_KEY_STATE: AiApiKeyState = {
  configured: false,
  storageState: "missing",
  error: null,
};

const BROWSER_FIXTURE_AI_SETTINGS: Record<string, string> = {
  ai_provider: DEFAULT_AI_SETTINGS.provider,
  ai_region: DEFAULT_AI_SETTINGS.region,
  ai_model: DEFAULT_AI_SETTINGS.model,
  ai_api_url: DEFAULT_AI_SETTINGS.customUrl,
  ai_tag_concurrency: DEFAULT_AI_SETTINGS.tagConcurrency,
  ai_tag_interval_ms: DEFAULT_AI_SETTINGS.tagIntervalMs,
  ai_tag_stop_on_rate_limit: DEFAULT_AI_SETTINGS.tagStopOnRateLimit ? "true" : "false",
};


let aiSaveTimer: ReturnType<typeof setTimeout> | null = null;
let aiSaveSequence = 0;

type SliceSet<TState> = (
  partial:
    | Partial<TState>
    | ((state: TState) => Partial<TState>)
) => void;

interface AiSettingsStoreState extends AiSettingsSlice {
  error: string | null;
}

function resolveAiApiUrl(settings: AiSettings): string {
  if (settings.provider === "custom") {
    return settings.customUrl;
  }

  const provider = AI_PROVIDERS.find((item) => item.id === settings.provider);
  return provider?.endpoints[settings.region] ?? "";
}

function normalizeAiSettings(values: Record<string, string | null | undefined>): AiSettings {
  const provider = values.ai_provider || DEFAULT_AI_SETTINGS.provider;
  const providerMeta = AI_PROVIDERS.find((item) => item.id === provider);
  const fallbackRegion = providerMeta?.regions[0] ?? DEFAULT_AI_SETTINGS.region;
  const rawRegion = values.ai_region || fallbackRegion;
  const region = (providerMeta?.regions.includes(rawRegion as RegionId)
    ? rawRegion
    : fallbackRegion) as RegionId;

  return {
    provider,
    region,
    apiKey: DEFAULT_AI_SETTINGS.apiKey,
    model: values.ai_model ?? providerMeta?.defaultModel ?? DEFAULT_AI_SETTINGS.model,
    customUrl: values.ai_api_url ?? DEFAULT_AI_SETTINGS.customUrl,
    tagConcurrency: values.ai_tag_concurrency ?? DEFAULT_AI_SETTINGS.tagConcurrency,
    tagIntervalMs: values.ai_tag_interval_ms ?? DEFAULT_AI_SETTINGS.tagIntervalMs,
    tagStopOnRateLimit:
      values.ai_tag_stop_on_rate_limit == null
        ? DEFAULT_AI_SETTINGS.tagStopOnRateLimit
        : values.ai_tag_stop_on_rate_limit !== "false" &&
          values.ai_tag_stop_on_rate_limit !== "0",
  };
}

function serializeAiSettings(settings: AiSettings): Record<string, string> {
  return {
    ai_provider: settings.provider,
    ai_region: settings.region,
    ai_model: settings.model,
    ai_api_url: resolveAiApiUrl(settings),
    ai_tag_concurrency: settings.tagConcurrency,
    ai_tag_interval_ms: settings.tagIntervalMs,
    ai_tag_stop_on_rate_limit: settings.tagStopOnRateLimit ? "true" : "false",
  };
}

function parseAiConnectionError(err: unknown): AiConnectionTestResult {
  const parsed = parseBackendError(err);
  const prefixedMessage = parsed.message.match(/^API .*?:\s*(.*)$/)?.[1]?.trim();

  return {
    ok: false,
    msg: prefixedMessage || parsed.message,
    code: parsed.code ?? undefined,
    details: parsed.details ?? undefined,
  };
}

export function createAiSettingsInitialState(): AiSettingsSliceState {
  return {
    aiSettings: { ...DEFAULT_AI_SETTINGS },
    aiApiKeyState: {
      configured: false,
      storageState: "missing",
      error: null,
    },
    aiSettingsLoaded: false,
    isLoadingAiSettings: false,
    aiSaveStatus: "idle",
    aiSaveError: null,
    aiTesting: false,
    aiTestResult: null,
  };
}

export function createAiSettingsSlice<TState extends AiSettingsStoreState>(
  set: SliceSet<TState>,
  get: () => TState
): AiSettingsSliceActions {
  return {
    loadAiSettings: async () => {
      set({ isLoadingAiSettings: true, aiSaveError: null } as Partial<TState>);
      if (!isTauriRuntime()) {
        set({
          aiSettings: normalizeAiSettings(BROWSER_FIXTURE_AI_SETTINGS),
          aiApiKeyState: BROWSER_FIXTURE_AI_API_KEY_STATE,
          aiSettingsLoaded: true,
          isLoadingAiSettings: false,
          aiSaveStatus: "saved",
        } as Partial<TState>);
        return;
      }
      try {
        const [values, aiApiKeyState] = await Promise.all([
          invoke<Record<string, string | null>>("get_settings", {
            keys: AI_SETTING_KEYS,
          }),
          invoke<AiApiKeyState>("get_ai_api_key_state"),
        ]);
        set({
          aiSettings: normalizeAiSettings(values),
          aiApiKeyState,
          aiSettingsLoaded: true,
          isLoadingAiSettings: false,
          aiSaveStatus: "saved",
        } as Partial<TState>);
      } catch (err) {
        set({
          error: String(err),
          aiSaveError: String(err),
          aiSettingsLoaded: true,
          isLoadingAiSettings: false,
          aiSaveStatus: "error",
        } as Partial<TState>);
      }
    },

    updateAiSettings: (patch) => {
      const current = get();
      const next = { ...current.aiSettings, ...patch };
      set({
        aiSettings: next,
        aiSaveStatus: current.aiSettingsLoaded ? "saving" : current.aiSaveStatus,
        aiSaveError: null,
        aiTestResult: null,
      } as Partial<TState>);

      if (!current.aiSettingsLoaded) {
        return;
      }

      if (aiSaveTimer) {
        clearTimeout(aiSaveTimer);
      }

      aiSaveTimer = setTimeout(() => {
        aiSaveTimer = null;
        void get().flushAiSettings();
      }, AI_SAVE_DEBOUNCE_MS);
    },

    clearAiApiKey: async () => {
      set({ aiSaveStatus: "saving", aiSaveError: null } as Partial<TState>);
      if (!isTauriRuntime()) {
        set({
          aiSettings: { ...get().aiSettings, apiKey: "" },
          aiApiKeyState: BROWSER_FIXTURE_AI_API_KEY_STATE,
          aiSaveStatus: "saved",
          aiSaveError: null,
          aiTestResult: null,
        } as Partial<TState>);
        return;
      }
      try {
        const apiKeyState = await invoke<AiApiKeyState>("clear_ai_api_key");
        set({
          aiSettings: { ...get().aiSettings, apiKey: "" },
          aiApiKeyState: apiKeyState,
          aiSaveStatus: "saved",
          aiSaveError: null,
          aiTestResult: null,
        } as Partial<TState>);
      } catch (err) {
        set({
          aiSaveStatus: "error",
          aiSaveError: String(err),
          error: String(err),
        } as Partial<TState>);
        throw err;
      }
    },

    flushAiSettings: async () => {
      if (!get().aiSettingsLoaded) {
        return;
      }

      if (aiSaveTimer) {
        clearTimeout(aiSaveTimer);
        aiSaveTimer = null;
      }

      const sequence = ++aiSaveSequence;
      const settings = get().aiSettings;
      set({ aiSaveStatus: "saving", aiSaveError: null } as Partial<TState>);

      if (!isTauriRuntime()) {
        if (sequence === aiSaveSequence) {
          set({
            aiSettings: { ...settings, apiKey: "" },
            aiApiKeyState: settings.apiKey.trim()
              ? { configured: true, storageState: "session", error: null }
              : get().aiApiKeyState,
            aiSaveStatus: "saved",
            aiSaveError: null,
          } as Partial<TState>);
        }
        return;
      }

      try {
        const savedApiKeyState = settings.apiKey.trim()
          ? await invoke<AiApiKeyState>("set_ai_api_key", { value: settings.apiKey })
          : get().aiApiKeyState;
        await invoke("set_settings", { values: serializeAiSettings(settings) });
        if (sequence === aiSaveSequence) {
          const latest = get().aiSettings;
          set({
            aiSettings: { ...latest, apiKey: "" },
            aiApiKeyState: savedApiKeyState,
            aiSaveStatus: "saved",
            aiSaveError: null,
          } as Partial<TState>);
        }
      } catch (err) {
        if (sequence === aiSaveSequence) {
          set({
            aiSaveStatus: "error",
            aiSaveError: String(err),
            error: String(err),
          } as Partial<TState>);
        }
        throw err;
      }
    },

    testAiConnection: async () => {
      await get().flushAiSettings();
      set({ aiTesting: true, aiTestResult: null, error: null } as Partial<TState>);

      if (!isTauriRuntime()) {
        const testResult = { ok: false, msg: "AI connection is unavailable in the browser fixture." };
        set({ aiTesting: false, aiTestResult: testResult } as Partial<TState>);
        return testResult;
      }

      try {
        const result = await invoke<string>("explain_skill", {
          content: "Test connection. Reply with: OK",
        });
        const testResult = { ok: true, msg: result.slice(0, 60) };
        set({ aiTesting: false, aiTestResult: testResult } as Partial<TState>);
        return testResult;
      } catch (err) {
        const testResult = parseAiConnectionError(err);
        set({ aiTesting: false, aiTestResult: testResult } as Partial<TState>);
        return testResult;
      }
    },
  };
}

export function resetAiSettingsSliceForTests() {
  if (aiSaveTimer) {
    clearTimeout(aiSaveTimer);
    aiSaveTimer = null;
  }
  aiSaveSequence = 0;
}
