# Implementation Plan

1. Add provider API key URL metadata and a resolver helper in `src/data/aiProviders.ts`.
   - Verify with SettingsView assertions that region-specific URLs resolve through the rendered link.

2. Update `AiSettingsSection` to render the key acquisition link next to the API Key label and stack Secret Vault / Runtime Routing vertically.
   - Remove the `xl:grid-cols-2` two-column grid for this area.
   - Keep the order as Secret Vault, Runtime Routing, then the AI save-status row.
   - Leave Provider selection above and Throughput Controls below.
   - Verify API key input remains accessible by label and reveal/clear controls still work.

3. Add Chinese and English i18n strings for the link label and accessible label.
   - Verify no hard-coded user-facing text is introduced.

4. Extend `SettingsView.test.tsx` for provider link behavior:
   - Built-in provider shows the link.
   - Custom provider hides it.
   - Region-specific provider updates the link target.

5. Run validation:
   - `pnpm test -- src/test/SettingsView.test.tsx`
   - `pnpm typecheck`
   - `pnpm lint`
   - Browser or screenshot verification of the Settings > Integrations AI Provider section at desktop width.
   - `just ci`
