import { invoke } from "@tauri-apps/api/core";
import { AI_PROVIDERS, type RegionId } from "@/data/aiProviders";

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
  details?: string;
}

export interface AiSettingsSliceState {
  aiSettings: AiSettings;
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
  "ai_api_key",
  "ai_model",
  "ai_api_url",
  "ai_tag_concurrency",
  "ai_tag_interval_ms",
  "ai_tag_stop_on_rate_limit",
];

const AI_SAVE_DEBOUNCE_MS = 800;

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

function normalizeAiSettings(
  values: Record<string, string | null | undefined>
): AiSettings {
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
    apiKey: values.ai_api_key ?? DEFAULT_AI_SETTINGS.apiKey,
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
    ai_api_key: settings.apiKey,
    ai_model: settings.model,
    ai_api_url: resolveAiApiUrl(settings),
    ai_tag_concurrency: settings.tagConcurrency,
    ai_tag_interval_ms: settings.tagIntervalMs,
    ai_tag_stop_on_rate_limit: settings.tagStopOnRateLimit ? "true" : "false",
  };
}

function parseAiConnectionError(err: unknown): AiConnectionTestResult {
  const raw = String(err);
  const lines = raw.split("\n");
  const firstLine = lines[0]?.trim() ?? raw;
  const remaining = lines.slice(1).join("\n").trim();
  const prefixedMessage = firstLine.match(/^API .*?:\s*(.*)$/)?.[1]?.trim();

  return {
    ok: false,
    msg: remaining || prefixedMessage || raw,
    details: remaining ? firstLine : undefined,
  };
}

export function createAiSettingsInitialState(): AiSettingsSliceState {
  return {
    aiSettings: { ...DEFAULT_AI_SETTINGS },
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
      try {
        const values = await invoke<Record<string, string | null>>("get_settings", {
          keys: AI_SETTING_KEYS,
        });
        set({
          aiSettings: normalizeAiSettings(values),
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

      try {
        await invoke("set_settings", { values: serializeAiSettings(settings) });
        if (sequence === aiSaveSequence) {
          set({ aiSaveStatus: "saved", aiSaveError: null } as Partial<TState>);
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
