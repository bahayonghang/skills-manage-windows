export type RegionId = "cn" | "intl";
export type ApiProtocol = "" | "anthropic" | "openai";

export interface ApiProtocolOption {
  id: ApiProtocol;
  labelKey: string;
  descriptionKey: string;
}

export const API_PROTOCOLS: ApiProtocolOption[] = [
  {
    id: "",
    labelKey: "settings.aiProtocolAuto",
    descriptionKey: "settings.aiProtocolAutoDesc",
  },
  {
    id: "anthropic",
    labelKey: "settings.aiProtocolAnthropic",
    descriptionKey: "settings.aiProtocolAnthropicDesc",
  },
  {
    id: "openai",
    labelKey: "settings.aiProtocolOpenAi",
    descriptionKey: "settings.aiProtocolOpenAiDesc",
  },
];

export interface AiProvider {
  id: string;
  name: { zh: string; en: string };
  labelKey: string;
  regions: RegionId[]; // which regions are supported
  endpoints: Partial<Record<RegionId, string>>; // API base URL per region
  defaultModel: string;
}

export const AI_PROVIDERS: AiProvider[] = [
  {
    id: "claude",
    labelKey: "settings.aiProviders.claude",
    name: { zh: "Claude", en: "Claude" },
    regions: ["intl"],
    endpoints: {
      intl: "https://api.anthropic.com/v1/messages",
    },
    defaultModel: "claude-sonnet-4-20250514",
  },
  {
    id: "glm",
    labelKey: "settings.aiProviders.glm",
    name: { zh: "智谱 GLM", en: "Zhipu GLM" },
    regions: ["cn", "intl"],
    endpoints: {
      cn: "https://open.bigmodel.cn/api/anthropic/v1/messages",
      intl: "https://api.z.ai/api/anthropic/v1/messages",
    },
    defaultModel: "glm-5",
  },
  {
    id: "minimax",
    labelKey: "settings.aiProviders.minimax",
    name: { zh: "MiniMax", en: "MiniMax" },
    regions: ["cn", "intl"],
    endpoints: {
      cn: "https://api.minimaxi.com/anthropic/v1/messages",
      intl: "https://api.minimax.io/anthropic/v1/messages",
    },
    defaultModel: "MiniMax-M2.7",
  },
  {
    id: "kimi",
    labelKey: "settings.aiProviders.kimi",
    name: { zh: "Kimi", en: "Kimi" },
    regions: ["cn"],
    endpoints: {
      cn: "https://api.moonshot.cn/anthropic/v1/messages",
    },
    defaultModel: "kimi-k2.5",
  },
  {
    id: "deepseek",
    labelKey: "settings.aiProviders.deepseek",
    name: { zh: "DeepSeek", en: "DeepSeek" },
    regions: ["cn"],
    endpoints: {
      cn: "https://api.deepseek.com/anthropic/v1/messages",
    },
    defaultModel: "deepseek-v4-flash",
  },
  {
    id: "openrouter",
    labelKey: "settings.aiProviders.openrouter",
    name: { zh: "OpenRouter", en: "OpenRouter" },
    regions: ["intl"],
    endpoints: {
      intl: "https://openrouter.ai/api/v1/messages",
    },
    defaultModel: "anthropic/claude-sonnet-4.6",
  },
  {
    id: "custom",
    labelKey: "settings.aiProviders.custom",
    name: { zh: "自定义", en: "Custom" },
    regions: ["cn", "intl"],
    endpoints: {},
    defaultModel: "",
  },
];

export const REGION_LABELS: Record<RegionId, { zh: string; en: string }> = {
  cn: { zh: "国内", en: "China" },
  intl: { zh: "国际", en: "International" },
};

export const REGION_LABEL_KEYS: Record<RegionId, string> = {
  cn: "settings.aiRegions.cn",
  intl: "settings.aiRegions.intl",
};
