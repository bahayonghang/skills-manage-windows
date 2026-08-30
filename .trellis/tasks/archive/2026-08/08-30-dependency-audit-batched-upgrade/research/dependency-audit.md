# Dependency Audit Research

Scan date: 2026-08-30 (Asia/Shanghai)

## 1. Authoritative Scope

`git ls-files` confirms that only these dependency surfaces are tracked:

- `.node-version`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`
- `rust-toolchain.toml`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
- `.github/dependabot.yml` and root `.github/workflows/*.yml`

Ignored/local `ref/`, `.opencode/`, VitePress cache, `node_modules/`, and Cargo target contain additional manifests but are not tracked and do not participate in the SkillPort build graph. They were inventoried but are excluded from upgrades.

## 2. Baseline Inventory

| Surface | Direct / pin count | Locked / observed size |
| --- | ---: | ---: |
| npm production | 28 | part of 1250 pnpm package entries |
| npm development | 22 | part of 1250 pnpm package entries |
| Cargo | 41 declarations / 40 unique crates | cargo-audit scanned 720 lock dependencies |
| GitHub Actions | 14 unique external actions | every root workflow ref is a 40-char SHA |
| Toolchain | Node 26, pnpm 10.12.3, Rust 1.97.0 | latest observed: Node 26.8.1, pnpm 10.34.5 / 11.24.0, Rust 1.98.0 |

Baseline `just ci` exit 0:

- Vitest: 177 files; 1966 passed, 1 skipped.
- Rust main library: 1468 passed, 7 ignored; all other binary/integration/doc test groups passed.
- Typecheck, lint, Clippy `-D warnings`, IPC codegen check, frontend build and docs build passed.

## 3. npm Findings

### Direct versions

Registry latest lookup found many compatible patch/minor updates, including Base UI 1.7.0, Fontsource 5.3.0, Lobe Icons 5.16.0, React/DOM 19.2.8, i18next 26.4.0, Lucide 1.37.0, Zustand 5.0.15, ESLint 10.9.1, typescript-eslint 8.68.0, Vite 8.2.2 and Vitest 4.1.11. No direct npm package returned a `deprecated` marker.

Complete direct npm classification:

- Compatible production updates: `@base-ui/react 1.7.0`, four Fontsource packages `5.3.0`, `@lobehub/icons 5.16.0`, `@tauri-apps/api 2.11.1`, dialog plugin `2.7.2`, `i18next 26.4.0`, `lucide-react 1.37.0`, React/DOM `19.2.8`, `react-i18next 17.0.12`, `react-router-dom 7.18.3`, `sonner 2.0.8`, `zustand 5.0.15`.
- Already latest production line: Tauri process/shell/sql/updater plugins, `class-variance-authority`, `clsx`, `cmdk`, language detector, React Markdown, Remark GFM, `tailwind-merge`, `tw-animate-css`.
- Compatible development updates: Tauri CLI `2.11.4`, Testing Library React `16.3.3`, user-event `14.6.6`, Node/React/ReactDOM types, typescript-eslint parser/plugin `8.68.0`, Vite React plugin `6.1.1`, ESLint `10.9.1`, React Refresh plugin `0.5.5`, shadcn `4.19.0`, Vite `8.2.2`, Vitest `4.1.11`.
- Already latest development line: Tailwind Vite/Tailwind `4.3.3`, React Hooks ESLint plugin `7.1.1`, VitePress `1.6.4`, YAML `2.9.0`.
- Breaking development updates: jest-dom 7, jsdom 30 and TypeScript 7, listed below.

Explicit major updates:

| Current | Latest | Classification | Project impact |
| --- | --- | --- | --- |
| `@testing-library/jest-dom 6.9.1` | 7.0.1 | Breaking | v7 requires direct `@testing-library/dom` peer and Node >=22; current Node is compatible |
| `jsdom 29.1.1` | 30.0.1 | Breaking | test runtime; project has custom PointerEvent, matchMedia, ResizeObserver, scroll and Storage seams |
| `TypeScript ~6.0.3` | 7.0.2 | Breaking | TS6 is the migration release; TS7 removes options deprecated in TS6 |

VitePress 1.6.4 is still the stable `latest`; 2.0.0-alpha.19 is the `next` line. Do not treat alpha as a routine update.

### Security

Full graph: 16 high, 32 moderate, 9 low. Production graph: 0 high/critical, 8 moderate, 5 low.

| Path family | Current vulnerable node | Stable patched target seen by pnpm audit | Notes |
| --- | --- | --- | --- |
| `jsdom -> undici` | 7.26.0 | 7.29.0 | development only |
| direct Vite | 8.0.14 | >=8.0.16 | compatible update available |
| `vitepress -> vite` | 5.4.21 | >=6.4.3 | no patched Vite 5 line; VitePress 2 remains alpha |
| `typescript-eslint -> minimatch -> brace-expansion` | vulnerable 3/4/5 closure | 5.0.9 | development only |
| `shadcn -> cosmiconfig -> js-yaml` | old 4.x | 4.3.2 | development only |
| `shadcn -> MCP SDK -> hono/fast-uri/ip-address` | old compatible versions | 4.13.5 / 3.1.6 / 10.7.0 | development only |
| `Lobe Icons -> Lobe UI -> Mermaid -> DOMPurify` | Mermaid 11.15.0, DOMPurify 3.4.6 | Mermaid 11.17.2 and newer DOMPurify fix most reports | production lock graph; one DOMPurify advisory reports no patched version |

The application imports ten Lobe icon Mono components by deep path. It does not directly import Mermaid or DOMPurify; runtime bundling exposure must be checked from the built output rather than inferred from lock presence.

## 4. Cargo Findings

Compatible dry-run would change about 207 packages, including Tauri 2.11.0 -> 2.11.5, anyhow 1.0.102 -> 1.0.104, event-listener 5.4.1 -> 5.4.2, spin 0.9.8 -> 0.9.9 and many ecosystem packages. It also removes several unmaintained old HTML/CSS parser dependencies. Because of its breadth, it must be split by dependency family.

Cargo-audit vulnerability results:

| Advisory | Lock package | Reachability | Treatment |
| --- | --- | --- | --- |
| RUSTSEC-2026-0235 | rkyv 0.7.46 | no inverse tree on current target or `--target all` | lock-only optional closure; >=0.8.17 fix cannot be selected by current optional dependency |
| RUSTSEC-2023-0071 | rsa 0.9.10 | no inverse tree on current target or `--target all` | no patched release; lock-only closure |

Informational warnings include unmaintained Linux GTK3 crates, fxhash, paste/proc-macro-error/unic crates; unsound anyhow/event-listener/glib/rand versions; and yanked spin 0.9.8. Current Windows-reachable warnings such as anyhow/event-listener/spin and Tauri build utility closures have compatible updates and should be fixed before majors.

Direct Breaking Change candidates:

| Current | Latest stable | Main migration risk |
| --- | --- | --- |
| base64 0.22.1 | 0.23.1 | decoding error detail/SIMD defaults; current `Engine` API is close |
| keyring 3.6.3 | 4.2.0 | split into keyring-core/application crate and credential-store selection |
| reqwest 0.12.28 | 0.13.4 | transport/TLS/proxy/redirect/error behavior across many services |
| sha2 0.10.9 | 0.11.0 | digest 0.11, newtypes and feature removals; stored hashes must remain identical |
| sqlx 0.8.6 | 0.9.0 | `SqlSafeStr`, dynamic query handling, SQLite feature/safety changes |
| zip 2.4.2 | 8.6.0 | removed deprecated APIs/data fields; archive security and filename behavior |

All other direct Cargo declarations are classified as follows:

- Compatible update candidates: async-trait, chrono, clap, flate2, futures-util, regex, serde, serde_json, tar, the Tauri core/build/dialog family, thiserror, Tokio, UUID and their lock closures.
- Already at the observed latest compatible/stable declaration line: fs2, minisign-verify, serde_norway, tempfile, tracing/appender/subscriber, urlencoding, walkdir, windows-sys, and the exact Tauri plugin versions not named above.
- Deliberately pinned/pre-release compatibility group: specta, specta-serde, specta-typescript and tauri-specta.

Specta and Tauri-Specta are deliberately pinned to matching `2.0.0-rc.25`; crates.io stable 1.x is not an upgrade from this compatibility group.

## 5. Toolchain And Actions

- Node is declared as major 26 and local/CI resolution can take current patch; no file pins Node 26.7.0.
- pnpm 10.12.3 is exact in package metadata/workflows; latest 10.x observed is 10.34.5, while 11.24.0 is a Breaking Change.
- Rust is exact 1.97.0; current stable is 1.98.0.
- Dependabot monitors only `github-actions` weekly.
- Root workflow comments/pins are behind current Action majors for checkout (4 -> 7), setup-node (6 -> 7), pnpm/action-setup (5 -> 6), upload/download artifact (4 -> 7/8), attest (2 -> 4), Azure login (2 -> 3), and Azure artifact-signing (1 -> 2). Pages Actions are already on the current observed majors.
- All Action refs must remain immutable full SHAs. Local tests can verify workflow shape but cannot prove hosted runner, Azure OIDC/signing or release behavior.

## 6. Deprecated API Review

- Baseline Clippy with `-D warnings`, TypeScript typecheck and ESLint produced no deprecated API diagnostics.
- No use of jest-dom's deprecated `toBeEmpty`, `toBeInTheDOM` or `toHaveDescription` matchers was found.
- TypeScript configs already use `moduleResolution: bundler`, strict mode, explicit types, no emit and modern ESM; no TS6 deprecated compiler option was found.
- Major migrations can still reveal removed upstream APIs, especially SQLx dynamic queries, keyring credential-store configuration, zip removed methods and SHA2 digest/newtype changes.

## 7. Official Sources

- RustSec advisory database / cargo-audit output: https://rustsec.org/
- npm advisory output and GitHub Advisory Database: https://github.com/advisories
- TypeScript 6 migration/deprecations for TS7: https://www.typescriptlang.org/docs/handbook/release-notes/typescript-6-0.html
- VitePress releases (stable 1.6.4, v2 alpha): https://github.com/vuejs/vitepress/releases
- VitePress maintainer discussion of v1 + Vite 6 compatibility: https://github.com/vuejs/vitepress/discussions/5072
- jest-dom v7 release: https://github.com/testing-library/jest-dom/releases/tag/v7.0.0
- Checkout v7 behavior: https://github.com/actions/checkout
- SQLx changelog: https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md
- Reqwest releases: https://github.com/seanmonstar/reqwest/releases
- Keyring v4 architecture: https://github.com/open-source-cooperative/keyring-rs/wiki/Keyring-Core
- Base64 release notes: https://github.com/marshallpierce/rust-base64/blob/master/RELEASE-NOTES.md
- SHA2 changelog: https://github.com/RustCrypto/hashes/blob/master/sha2/CHANGELOG.md
- Zip releases: https://github.com/zip-rs/zip2/releases
