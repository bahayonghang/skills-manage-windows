# AI Explanation

AI Explanation generates a short, plain-language summary of what a skill does. Use it to triage unfamiliar skills before installing, or to draft documentation around your own skills.

## How it works

When you click **Explain** on a skill detail page, SkillPort:

1. Reads the SKILL.md content (frontmatter + body).
2. Sends it to the AI provider configured in Settings → AI.
3. Streams the response back into the panel.
4. Caches the explanation per skill so the next view is instant.

You can re-run the explanation at any time. Re-running invalidates the cache and overwrites the stored result.

## Supported providers

| Provider | Default endpoint | Notes |
|----------|------------------|-------|
| Anthropic Claude | `api.anthropic.com` | Native Anthropic message format. Skips `thinking` blocks. |
| GLM (Zhipu) | `open.bigmodel.cn` | OpenAI-compatible chat completions. |
| MiniMax | `api.minimax.chat` | OpenAI-compatible chat completions. |
| Kimi (Moonshot) | `api.moonshot.cn` | OpenAI-compatible chat completions. |
| DeepSeek | `api.deepseek.com` | OpenAI-compatible chat completions. |
| OpenRouter | `openrouter.ai/api/v1` | Proxy for many models with one key. |
| Custom | configurable | Any OpenAI-compatible endpoint. |

The provider list lives in `src/data/aiProviders.ts` and is updated with each release. Both China-region and international endpoints are pre-filled where applicable.

## Configuration

Settings → AI exposes:

- **Provider** — picks one of the entries above.
- **API key** — stored locally in the SQLite settings table; not encrypted at rest.
- **Model** — provider-specific model name (for example `claude-haiku-4-5`, `glm-4.5-flash`).
- **API base URL** — override when you front the provider with a proxy.
- **Temperature** and other prompt parameters where applicable.

Switching the provider does not invalidate cached explanations; only re-running an explanation does.

## Privacy

- Network calls happen only when you press **Explain**.
- The skill content (text only) is sent to the configured endpoint.
- No telemetry; nothing is sent to SkillPort or third parties beyond the provider you chose.

## Where to go next

- See AI provider config alongside the rest: [Settings](./settings).
- Translate platform docs and theme tokens: [i18n and Themes](./i18n-and-themes).

---

Last reviewed: 2026-05-04
