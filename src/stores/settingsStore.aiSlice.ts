import { invoke, isTauriRuntime } from "@/lib/ipc";
import { AI_PROVIDERS, type ApiProtocol, type RegionId } from "@/data/aiProviders";
import {
  normalizeApiProtocol,
  normalizeProviderRegion,
  resolveProviderApiUrl,
} from "@/lib/aiProviderConfig";
import { parseBackendError } from "@/lib/backendError";
import type { AiApiKeyState } from "@/types/credentials";

export type AiSaveStatus = "idle" | "saving" | "saved" | "error";

export const AI_BROWSER_FIXTURE_TEST_CODE = "browser_fixture";

export interface AiSettings {
  provider: string;
  region: RegionId;
  apiKey: string;
  model: string;
  customUrl: string;
  protocol: ApiProtocol;
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
  aiRawSettings: Record<string, string | null | undefined>;
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
  switchAiProvider: (providerId: string) => Promise<void>;
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
  protocol: "",
  tagConcurrency: "1",
  tagIntervalMs: "4000",
  tagStopOnRateLimit: true,
};

const GLOBAL_AI_SETTING_KEYS = [
  "ai_provider",
  "ai_tag_concurrency",
  "ai_tag_interval_ms",
  "ai_tag_stop_on_rate_limit",
];

const PROVIDER_SCOPED_SETTING_NAMES = [
  "ai_region",
  "ai_model",
  "ai_api_url",
  "ai_custom_base_url",
  "ai_protocol",
];

const LEGACY_AI_SETTING_KEYS = ["ai_region", "ai_model", "ai_api_url", "ai_protocol"];

const AI_SETTING_KEYS = [
  ...GLOBAL_AI_SETTING_KEYS,
  ...LEGACY_AI_SETTING_KEYS,
  ...AI_PROVIDERS.flatMap((provider) =>
    PROVIDER_SCOPED_SETTING_NAMES.map((name) => providerScopedSettingKey(name, provider.id))
  ),
];

const AI_SAVE_DEBOUNCE_MS = 800;


const BROWSER_FIXTURE_AI_API_KEY_STATE: AiApiKeyState = {
  configured: false,
  storageState: "missing",
  fingerprint: null,
  error: null,
};

const BROWSER_FIXTURE_AI_SETTINGS: Record<string, string> = {
  ai_provider: DEFAULT_AI_SETTINGS.provider,
  ai_region: DEFAULT_AI_SETTINGS.region,
  ai_model: DEFAULT_AI_SETTINGS.model,
  ai_api_url: DEFAULT_AI_SETTINGS.customUrl,
  ai_custom_base_url: DEFAULT_AI_SETTINGS.customUrl,
  ai_protocol: DEFAULT_AI_SETTINGS.protocol,
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

function providerScopedSettingKey(name: string, providerId: string): string {
  return `${name}__${providerId}`;
}

function scopedValue(
  values: Record<string, string | null | undefined>,
  providerId: string,
  name: string
): string | null | undefined {
  return values[providerScopedSettingKey(name, providerId)] ?? values[name];
}

function normalizeProviderApiUrl(providerId: string, value: string): string {
  const normalized = value.trim().replace(/\/+$/, "");
  if (providerId === "openrouter" && normalized === "https://openrouter.ai/api/v1/messages") {
    return "https://openrouter.ai/api/v1/chat/completions";
  }
  return value;
}

function normalizeAiSettings(values: Record<string, string | null | undefined>): AiSettings {
  const provider = values.ai_provider || DEFAULT_AI_SETTINGS.provider;
  const providerMeta = AI_PROVIDERS.find((item) => item.id === provider);
  const region = normalizeProviderRegion(provider, scopedValue(values, provider, "ai_region"));
  const customUrl =
    scopedValue(values, provider, "ai_custom_base_url") ??
    scopedValue(values, provider, "ai_api_url") ??
    DEFAULT_AI_SETTINGS.customUrl;
  const protocol =
    providerMeta?.defaultProtocol ??
    normalizeApiProtocol(scopedValue(values, provider, "ai_protocol"));

  return {
    provider,
    region,
    apiKey: DEFAULT_AI_SETTINGS.apiKey,
    model:
      scopedValue(values, provider, "ai_model") ??
      providerMeta?.defaultModel ??
      DEFAULT_AI_SETTINGS.model,
    customUrl: normalizeProviderApiUrl(provider, customUrl),
    protocol: protocol || DEFAULT_AI_SETTINGS.protocol,
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
  const resolvedUrl = resolveProviderApiUrl({
    providerId: settings.provider,
    region: settings.region,
    customUrl: settings.customUrl,
    protocol: settings.protocol,
  });

  return {
    ai_provider: settings.provider,
    [providerScopedSettingKey("ai_region", settings.provider)]: settings.region,
    [providerScopedSettingKey("ai_model", settings.provider)]: settings.model,
    [providerScopedSettingKey("ai_api_url", settings.provider)]: resolvedUrl,
    [providerScopedSettingKey("ai_custom_base_url", settings.provider)]: settings.customUrl,
    [providerScopedSettingKey("ai_protocol", settings.provider)]: settings.protocol,
    ai_tag_concurrency: settings.tagConcurrency,
    ai_tag_interval_ms: settings.tagIntervalMs,
    ai_tag_stop_on_rate_limit: settings.tagStopOnRateLimit ? "true" : "false",
  };
}

function parseAiConnectionError(err: unknown): AiConnectionTestResult {
  const parsed = parseBackendError(err);
  const prefixedMessage = parsed.message.match(/^API .*?:\s*(.*)$/)?.[1]?.trim();

  return asAiConnectionTestResult({
    ok: false,
    msg: prefixedMessage || parsed.message,
    code: parsed.code ?? undefined,
    details: parsed.details ?? undefined,
  });
}

function asAiConnectionTestResult(result: {
  ok: boolean;
  msg: string;
  code?: string | null;
  details?: string | null;
}): AiConnectionTestResult {
  return {
    ok: result.ok,
    msg: result.msg,
    code: result.code ?? undefined,
    details: result.details ?? undefined,
  };
}

export function createAiSettingsInitialState(): AiSettingsSliceState {
  return {
    aiSettings: { ...DEFAULT_AI_SETTINGS },
    aiRawSettings: {},
    aiApiKeyState: {
      configured: false,
      storageState: "missing",
      fingerprint: null,
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
          aiRawSettings: BROWSER_FIXTURE_AI_SETTINGS,
          aiApiKeyState: BROWSER_FIXTURE_AI_API_KEY_STATE,
          aiSettingsLoaded: true,
          isLoadingAiSettings: false,
          aiSaveStatus: "saved",
        } as Partial<TState>);
        return;
      }
      try {
        const values = await invoke("get_settings", {
          keys: AI_SETTING_KEYS,
        });
        const loadedSettings = normalizeAiSettings(values);
        const aiApiKeyState = await invoke("get_ai_api_key_state", {
          provider: loadedSettings.provider,
        });
        set({
          aiSettings: loadedSettings,
          aiRawSettings: values,
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

    switchAiProvider: async (providerId) => {
      if (aiSaveTimer) {
        clearTimeout(aiSaveTimer);
        aiSaveTimer = null;
        await get().flushAiSettings();
      }

      const current = get();
      const nextSettings = {
        ...current.aiSettings,
        ...createAiProviderSwitchPatch(providerId, current.aiRawSettings),
      };
      set({
        aiSettings: nextSettings,
        aiApiKeyState: BROWSER_FIXTURE_AI_API_KEY_STATE,
        isLoadingAiSettings: true,
        aiSaveStatus: current.aiSettingsLoaded ? "saving" : current.aiSaveStatus,
        aiSaveError: null,
        aiTestResult: null,
      } as Partial<TState>);

      if (!isTauriRuntime()) {
        set({
          isLoadingAiSettings: false,
          aiSaveStatus: "saved",
        } as Partial<TState>);
        return;
      }

      try {
        const [aiApiKeyState] = await Promise.all([
          invoke("get_ai_api_key_state", { provider: providerId }),
          current.aiSettingsLoaded
            ? invoke("set_settings", { values: serializeAiSettings(nextSettings) })
            : Promise.resolve(),
        ]);
        set({
          aiApiKeyState,
          aiRawSettings: {
            ...get().aiRawSettings,
            ...serializeAiSettings(nextSettings),
          },
          isLoadingAiSettings: false,
          aiSaveStatus: "saved",
          aiSaveError: null,
        } as Partial<TState>);
      } catch (err) {
        set({
          isLoadingAiSettings: false,
          aiSaveStatus: "error",
          aiSaveError: String(err),
          error: String(err),
        } as Partial<TState>);
        throw err;
      }
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
        const apiKeyState = await invoke("clear_ai_api_key", {
          provider: get().aiSettings.provider,
        });
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
            aiRawSettings: {
              ...get().aiRawSettings,
              ...serializeAiSettings(settings),
            },
            aiApiKeyState: settings.apiKey.trim()
              ? { configured: true, storageState: "session", fingerprint: null, error: null }
              : get().aiApiKeyState,
            aiSaveStatus: "saved",
            aiSaveError: null,
          } as Partial<TState>);
        }
        return;
      }

      try {
        let savedApiKeyState = get().aiApiKeyState;
        if (settings.apiKey.trim()) {
          savedApiKeyState = await invoke("set_ai_api_key", {
            provider: settings.provider,
            value: settings.apiKey,
          });
          if (sequence === aiSaveSequence) {
            set({
              aiSettings: { ...get().aiSettings, apiKey: "" },
              aiApiKeyState: savedApiKeyState,
            } as Partial<TState>);
          }
        }
        await invoke("set_settings", { values: serializeAiSettings(settings) });
        if (sequence === aiSaveSequence) {
          const latest = get().aiSettings;
          set({
            aiSettings: { ...latest, apiKey: "" },
            aiRawSettings: {
              ...get().aiRawSettings,
              ...serializeAiSettings(settings),
            },
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
        const testResult: AiConnectionTestResult = {
          ok: false,
          code: AI_BROWSER_FIXTURE_TEST_CODE,
          msg: "",
        };
        set({ aiTesting: false, aiTestResult: testResult } as Partial<TState>);
        return testResult;
      }

      try {
        const testResult = asAiConnectionTestResult(await invoke("test_ai_connection"));
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

export function getNextAiProviderScopedSettings(
  providerId: string,
  values: Record<string, string | null | undefined>
): AiSettings {
  return normalizeAiSettings({ ...values, ai_provider: providerId });
}

export function createAiProviderSwitchPatch(
  providerId: string,
  values: Record<string, string | null | undefined> = {}
): Partial<AiSettings> {
  const next = getNextAiProviderScopedSettings(providerId, values);
  return {
    provider: providerId,
    region: next.region,
    apiKey: "",
    model: next.model,
    customUrl: next.customUrl,
    protocol: next.protocol,
  };
}

export function resetAiSettingsSliceForTests() {
  if (aiSaveTimer) {
    clearTimeout(aiSaveTimer);
    aiSaveTimer = null;
  }
  aiSaveSequence = 0;
}
