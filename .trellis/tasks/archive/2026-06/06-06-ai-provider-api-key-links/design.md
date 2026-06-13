# Design

## Architecture And Boundaries

This is a frontend-only settings UI change. Provider-specific key acquisition links belong with provider metadata in `src/data/aiProviders.ts`; the settings component should only resolve and render the current provider's link.

No backend, database, or secure-storage behavior changes are required.

## Data Contract

Extend `AiProvider` with an optional `apiKeyUrl` field:

```ts
apiKeyUrl?: string | Partial<Record<RegionId, string>>;
```

Use a small resolver helper near the metadata:

```ts
resolveProviderApiKeyUrl(provider, region)
```

This keeps region-specific lookup testable and avoids duplicating provider ID switches in the component.

## UI Shape

Inside the Secret Vault panel:

- Keep the existing section heading.
- Render the API Key label and "get API key" link in a compact row above the secret input.
- Use an external-link affordance and `target="_blank"` / `rel="noreferrer"` for outbound provider pages.
- Hide the link when the provider has no official key page, such as Custom.

For layout:

- Replace `grid gap-3 xl:grid-cols-2` with a vertical-only stack. Do not add a desktop breakpoint that places these two panels side by side again.
- Keep Secret Vault and Runtime Routing as separate full-width panels.
- The panel order is fixed: Secret Vault first, Runtime Routing second.
- Move the save-status row into the same vertical rhythm directly after Runtime Routing, so the highlighted configuration area reads top to bottom instead of as a two-column block.
- Keep Provider selection above the stack and Throughput Controls below it.
- Reduce empty space by letting Runtime Routing take only the vertical space it needs instead of stretching next to the taller Secret Vault panel.

## Official Link Research

Use direct provider consoles or official docs/console pages:

- Claude: `https://platform.claude.com/settings/keys`
  - Claude Help Center says the Claude Console is where API keys are created.
- Zhipu GLM China: `https://bigmodel.cn/usercenter/proj-mgmt/apikeys`
  - BigModel docs link their API Keys page from the HTTP API guide.
- Zhipu GLM International: `https://z.ai/manage-apikey/apikey-list`
  - Z.AI docs link the API Keys page and state keys are created or managed there.
- MiniMax China: `https://platform.minimaxi.com/user-center/basic-information/interface-key`
  - MiniMax China docs say pay-as-you-go API keys are created from the interface key page.
- MiniMax International: `https://platform.minimax.io/user-center/basic-information/interface-key`
  - MiniMax international docs say API keys are created from API Keys / Create new secret key.
- Kimi: `https://platform.moonshot.cn/console/api-keys`
  - Current app metadata uses the China endpoint, and Moonshot docs link this API key console.
- DeepSeek: `https://platform.deepseek.com/api_keys`
  - DeepSeek docs say an API key must be created first and use Bearer auth.
- OpenRouter: `https://openrouter.ai/keys`
  - OpenRouter docs link this page for creating API keys.

The metadata values in `src/data/aiProviders.ts` are the source of truth for exact URLs.

## Compatibility

The existing provider IDs, endpoints, models, protocols, store serialization, secret reveal behavior, and backend connection tests remain unchanged.
