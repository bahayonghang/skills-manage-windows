# CI And Supply-chain Evidence

Collected on 2026-07-28 from the live `dev` checkout. No product file or lockfile
was changed during this research.

## Workflow Baseline

- `.github/workflows/ci.yml:27-71` defines job id `ci`, display name `just-ci`,
  and runner `windows-2022`. It checks out `inputs.checkout_ref || github.sha` and
  runs `node scripts/run-ci.mjs`.
- `.github/workflows/ci.yml:73-239` has Windows/Linux/macOS package smoke jobs,
  each guarded by direct `workflow_dispatch`.
- `.github/workflows/release-desktop.yml:67` reuses `ci.yml` at a frozen SHA;
  changes must preserve that release dependency.
- `src/test/contracts/ciWorkflowContract.test.ts` and
  `src/test/contracts/releaseWorkflowContract.test.ts` already parse YAML and
  assert the trigger, frozen checkout, stable `just-ci`, and release ordering.
- The existing Linux package jobs contain the GTK/WebKit/libsoup system package
  list needed to compile the Tauri crate on Ubuntu.

## External Action Refs

`git ls-remote` resolved the currently referenced movable refs to these commits:

| Action | Existing ref | Resolved commit |
| --- | --- | --- |
| `actions/checkout` | `v4` | `11d5960a326750d5838078e36cf38b85af677262` |
| `actions/setup-node` | `v6` | `249970729cb0ef3589644e2896645e5dc5ba9c38` |
| `actions/upload-artifact` | `v4` | `ea165f8d65b6e75b540449e92b4886f43607fa02` |
| `actions/download-artifact` | `v4` | `d3f86a106a0bac45b974a628896c90dbdf5c8093` |
| `pnpm/action-setup` | `v5` | `a8198c4bff370c8506180b035930dea56dbd5288` |
| `dtolnay/rust-toolchain` | `stable` | `4cda84d5c5c54efe2404f9d843567869ab1699d4` |
| `Swatinem/rust-cache` | `v2` | `42dc69e1aa15d09112580998cf2ef0119e2e91ae` |
| `peaceiris/actions-gh-pages` | `v4` | `329bcc8f12caed2cefe5a5b80781499a6f3b361b` |

All external `uses:` occurrences across `ci.yml`, `release-desktop.yml`, and
`docs.yml` use only these eight Actions. Local reusable workflow calls must not
be subjected to the SHA rule.

## JavaScript Audit Baseline

`pnpm audit --prod --json` reported:

- high: 9
- moderate: 15
- low: 5
- critical: 0

The direct dependency graph explains the actionable baseline:

- `shadcn@4.8.1` is in production dependencies but has no source import or
  package script usage. Its CLI-only graph owns the Hono, fast-uri,
  brace-expansion and js-yaml advisories. Latest compatible `4.15.0` exists,
  but it should first be classified as a development dependency.
- `react-router-dom@7.15.1` has a compatible stable update to `7.18.1`, fixing
  four listed router advisories. `GHSA-qwww-vcr4-c8h2` requires an unavailable
  stable `8.3.0` line; this desktop app uses BrowserRouter/navigation only and
  has no RSC/data action use, so a short exact exception is justified.
- `@lobehub/icons@5.8.0` pulls `@lobehub/ui` and PostCSS. Current `5.15.0`
  declares a smaller dependency set without that UI chain.
- The verifier must use `github_advisory_id`, not npm numeric IDs, as the stable
  exception key.

## Rust Audit Baseline

`cargo audit --json` exited 1 with seven vulnerabilities:

| Advisory | Package | Current | Remediation evidence |
| --- | --- | --- | --- |
| RUSTSEC-2026-0194/0195 | quick-xml | 0.38.4 | `plist 1.10.0` selects `quick-xml 0.41.0` |
| RUSTSEC-2026-0185 | quinn-proto | 0.11.14 | precise update to 0.11.15 succeeds |
| RUSTSEC-2026-0098/0099/0104 | rustls-webpki | 0.103.11 | precise update to 0.103.13 succeeds |
| RUSTSEC-2023-0071 | rsa | 0.9.10 | exact expiring exception; no patched release |

The same output contained 17 unmaintained, 4 unsound, and 1 yanked warning.
Those are visible debt but are distinct from `vulnerabilities.list`; this task
reports them without turning the initial security gate into an unrelated
dependency-modernization project.

An unrestricted `cargo update --dry-run` would change roughly 165 packages.
The focused commands above each update only the affected closure. Application
declarations can disable SQLx and `tauri-plugin-sql` defaults and use the narrower
`derive` feature, but `tauri-plugin-sql 2.4.0` itself still unconditionally enables
SQLx default features. `cargo tree -e features` proves that internal edge; the app
uses SQLite only, while the RSA advisory has no patched version. This justifies a
separate exact exception rather than pretending the vulnerability disappeared.

## Design Consequences

- Keep Windows `just-ci` stable and add a separate Ubuntu/macOS source matrix.
- Keep package smoke manual/release-only.
- Run live audits in a dedicated job; test parsing/policy through fixtures in
  the ordinary `just ci` chain.
- Use one checked JSON exception manifest shared by both ecosystems. Invalid,
  expired, duplicate, cross-ecosystem, or unused entries fail closed.
- Prefer dependency removal/reclassification and precise updates over blanket
  exceptions or a broad lockfile refresh.
