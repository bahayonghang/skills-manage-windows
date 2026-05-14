import { AI_PROVIDERS, type ApiProtocol, type RegionId } from "@/data/aiProviders";

export function normalizeApiProtocol(value: string | null | undefined): ApiProtocol {
  return value === "anthropic" || value === "openai" ? value : "";
}

function appendEndpointPath(base: string, suffix: string): string {
  return `${base.replace(/\/+$/, "")}${suffix}`;
}

export function resolveCustomUrl(rawUrl: string, protocol: ApiProtocol): string {
  const trimmed = rawUrl.trim();
  if (!trimmed) {
    return "";
  }

  const normalized = trimmed.replace(/\/+$/, "");
  const lower = normalized.toLowerCase();
  if (lower.endsWith("/v1/messages") || lower.includes("/anthropic/v1/messages")) {
    return normalized;
  }
  if (lower.endsWith("/v1/chat/completions")) {
    return normalized;
  }

  if (protocol === "openai") {
    return lower.endsWith("/v1")
      ? appendEndpointPath(normalized, "/chat/completions")
      : appendEndpointPath(normalized, "/v1/chat/completions");
  }

  return lower.endsWith("/v1")
    ? appendEndpointPath(normalized, "/messages")
    : appendEndpointPath(normalized, "/v1/messages");
}

export function getProviderDefaultRegion(providerId: string): RegionId {
  const provider = AI_PROVIDERS.find((item) => item.id === providerId);
  return provider?.regions[0] ?? "intl";
}

export function normalizeProviderRegion(
  providerId: string,
  value: string | null | undefined
): RegionId {
  const provider = AI_PROVIDERS.find((item) => item.id === providerId);
  const fallback = provider?.regions[0] ?? "intl";
  return provider?.regions.includes(value as RegionId) ? (value as RegionId) : fallback;
}

export function getProviderDefaultModel(providerId: string): string {
  return AI_PROVIDERS.find((item) => item.id === providerId)?.defaultModel ?? "";
}

export function resolveProviderApiUrl({
  providerId,
  region,
  customUrl,
  protocol,
}: {
  providerId: string;
  region: RegionId;
  customUrl: string;
  protocol: ApiProtocol;
}): string {
  if (providerId === "custom") {
    return resolveCustomUrl(customUrl, protocol);
  }

  const provider = AI_PROVIDERS.find((item) => item.id === providerId);
  return provider?.endpoints[region] ?? "";
}
