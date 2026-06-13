# Optimize AI provider API key links

## Goal

Improve the Settings > Integrations AI Provider section so users can find the correct API key page for the currently selected provider directly next to the API Key field, while making the section more compact by stacking the key and runtime panels vertically.

## Requirements

- Add a clickable "get API key" link near the API Key label for each built-in AI provider.
- The link target must come from official provider pages discovered and verified during planning, not from guesses or third-party posts.
- Providers with separate China and International regions should use the region-appropriate key page when the selected region changes.
- The Custom provider should not imply an official key page.
- Replace the current side-by-side Secret Vault / Runtime Routing layout with a vertical layout to reduce empty horizontal space.
- Treat the highlighted middle configuration area as one vertical flow: Secret Vault first, Runtime Routing second, save status third.
- Keep the Provider selector card above this flow and Throughput Controls below it; neither should be merged into the credential/runtime stack.
- Keep user-visible copy in i18n resources for Chinese and English.
- Keep changes scoped to AI Provider settings UI and provider metadata.

## Acceptance Criteria

- [x] Claude, Zhipu GLM, MiniMax, Kimi, DeepSeek, and OpenRouter show a clickable API key acquisition link beside the API Key label.
- [x] Zhipu GLM and MiniMax use the selected region to choose the China or International key page where applicable.
- [x] Custom provider hides the acquisition link.
- [x] Secret Vault and Runtime Routing render as stacked full-width panels in this exact order: Secret Vault, then Runtime Routing.
- [x] The stack remains vertical at wide desktop widths; no `xl` or larger breakpoint reintroduces the side-by-side layout.
- [x] The AI save status row stays directly below Runtime Routing and above Throughput Controls.
- [x] The highlighted middle area has less empty space than the screenshot's two-column layout.
- [x] Existing API key reveal, replacement, clear, provider switching, and connection-test behavior still works.
- [x] Settings view tests cover the new link behavior and no existing tests regress.
- [x] Final verification includes at least `pnpm typecheck`, `pnpm lint`, relevant SettingsView tests, and `just ci`.

## Notes

- Current implementation evidence:
  - Provider metadata lives in `src/data/aiProviders.ts`.
  - AI Provider UI lives in `src/components/settings/AiSettingsSection.tsx`.
  - User-facing strings live in `src/i18n/locales/zh.json` and `src/i18n/locales/en.json`.
  - Existing SettingsView coverage includes AI key reveal, provider switching, custom protocol, OpenRouter URL, and key clearing.
- Verification evidence:
  - `pnpm exec vitest run src/test/SettingsView.test.tsx` passed with 96 tests.
  - `pnpm typecheck` passed.
  - `pnpm lint` passed.
  - `just ci` passed, including web typecheck/lint/sizecheck/test and Rust clippy/test.
  - Browser verification at `http://127.0.0.1:24200/settings/integrations` confirmed Secret Vault, Runtime Routing, and Throughput Controls share the same x/width and are vertically ordered.
  - Screenshot saved at `.trellis/tasks/06-06-ai-provider-api-key-links/settings-ai-provider-visible.png`.
- Spec update judgment:
  - No `.trellis/spec` update needed. This task did not add or change backend commands, APIs, DB schema, storage contracts, or cross-layer behavior; it reused existing frontend component and metadata patterns.
