# Implementation Plan — 全项目依赖审计与分批升级

## 0. Start Gate

- [x] 用户已在看到本规划最终摘要后的新消息中明确批准实施。
- [x] 运行 `task.py validate` 并完成人工 artifact/repository 交叉检查；只有通过后运行 `task.py start`。
- [x] 重新确认 `git status --short`，只允许本任务规划文件为已知改动。
- [x] 按 `trellis-before-dev` 重新加载 quality/frontend/backend 规范与本研究报告。
- [x] 记录当日 `pnpm audit --json`、`pnpm audit --prod --json`、`cargo audit`、registry 与 `cargo update --dry-run` 基线。

## Common Gate For Every Batch

1. 记录本批允许修改的精确文件和依赖族。
2. 使用包管理器的精确包名更新；禁止无审查的全图 `pnpm update --latest` 或 `cargo update`。
3. 检查 manifest、lock diff、重复版本、feature/peer graph 和生成物漂移。
4. 运行本批聚焦检查。
5. 运行 `just ci`。
6. 运行 `just audit`，并额外保存完整 `pnpm audit` 与 Cargo warnings 摘要。
7. 只有上述门槛全部通过才记录批次稳定并进入下一批。

失败时立即停止后续批次，定位并在本批最小范围修复；先重跑失败检查，再重跑步骤 5–6。若需要新增 audit exception、扩大 scope 或无法获得本批关键平台证据，暂停请求用户决定。

## Batch 1 — Low: Targeted Security And Lock Hygiene

目标：先消除有稳定 patch 的传递漏洞/unsound/yanked 版本，不混入主版本迁移。

- [x] pnpm 按父依赖族更新并审查目标：`undici >=7.29.0`、direct Vite `>=8.0.16`、`brace-expansion >=5.0.9`、`js-yaml >=4.3.2`、`hono >=4.13.5`、`fast-uri >=3.1.6`、`ip-address >=10.7.0`、`mermaid >=11.17.2`、可修复的 DOMPurify 版本。
- [x] 更新触发这些闭包的直接包到当前兼容线（如 jsdom 29、shadcn 4、typescript-eslint 8、Vite 8、Lobe Icons 5），避免全局 override；若某个传递项无法自然收敛，再评估最窄 selector override。
- [x] Cargo 精确更新 `anyhow`、`event-listener`、`spin` 及解除旧 Tauri utility closure 所需的 Tauri patch 组；逐组审查，不接受一次 207-package 的无差别 lock diff。
- [x] 复核 `rkyv` / `rsa` 的普通目标和 `--target all` 可达性，更新 `security/dependency-audit-exceptions.json` 的事实说明（只有在 advisory 仍被报告时保留），并同步 `.trellis/spec/quality/ci-quality-gate.md`。
- [x] 聚焦：dependency audit script tests、developer-experience contract、`cargo tree --duplicates` / inverse trees。
- [x] 完整门槛：`just ci` + `just audit`。

Batch 1 evidence (2026-08-30): production npm audit 0; full npm audit only retains the planned Batch 3 VitePress/Vite 5 path (1 high, 3 moderate); `rkyv` and `rsa` are absent from normal and `--target all` inverse trees; focused tests, lint, typecheck, frozen lockfile, Vite/VitePress builds, `just ci`, `just audit`, and `git diff --check` all passed. `just audit` reports 2 blocking Cargo advisories covered by 2 exact exceptions expiring 2026-11-30.

## Batch 2 — Low To Medium: Compatible Direct Dependencies

目标：更新剩余同 major/minor 的直接依赖，分离 UI 资源和工具链主版本。

- [x] pnpm runtime group：React/React DOM 19.2、i18next/react-i18next、router 7、Zustand 5、Tauri JS 2 等兼容更新。
- [x] pnpm development group：Testing Library 16/14、types、ESLint 10 / typescript-eslint 8、Vite plugin 6、Vitest 4、shadcn 4 等兼容更新；暂不进入 jest-dom 7、jsdom 30、TypeScript 7。
- [x] Cargo direct compatible group：Tauri 2.11 patch family、Tokio 1、serde/serde_json、chrono、clap、regex、uuid、flate2/tar 等，按功能族拆小 lock diff。
- [x] 聚焦：typecheck/lint、Tauri capability/IPC checks、受影响 Rust module tests。
- [x] 完整门槛：`just ci` + `just audit`。

Batch 2 evidence (2026-08-30): 19 intended same-major/minor npm direct upgrades and compatible Rust direct families were applied without business-source changes. Independent review added the omitted compatible `futures-util` 0.3.34 and `serde` 1.0.229 families; targeted Cargo dry-run then reported 0 pending packages. Focused frontend/Rust tests, frozen install, peer/duplicate review, lint, typecheck, capability/IPC checks, clippy, `just ci`, `just audit`, and `git diff --check` all passed. No Batch 3 assets, Batch 4 toolchain, or Batch 5+ breaking dependency entered this batch.

## Batch 3 — Medium: UI Assets And Documentation Security

- [x] 将四个 Fontsource 5.2.x 精确/范围声明升级到 5.3.0，核对 CSS entry、font weight/variable axes 与现有 typography/font contract。
- [x] 完成 Base UI 1.7、Lucide 1.x、Lobe Icons 5.16 等行为/资源更新；验证 10 个深路径 Mono icon import 仍存在。
- [x] 如果 VitePress 1.6.4 仍锁定存在 high advisory 的 Vite 5，尝试仅限 `vitepress>vite` 的 Vite 6.4.3+ override；不采用 VitePress 2 alpha。
- [x] 聚焦：font/typography/icon contracts、交互测试、`pnpm docs:site:build`。
- [x] 完整门槛：`just ci` + `just audit`。

Batch 3 evidence (2026-08-30): Fontsource 5.3.0, Base UI 1.7.0, Lucide 1.37.0, and Lobe Icons 5.16.0 passed font, typography, icon, interaction, typecheck, lint, production build, and docs build contracts. All 10 deep Mono import paths remain present. The only override is `vitepress>vite` 6.4.3; VitePress remains 1.6.4 while the app/test graph remains on Vite 8.2.2. Full and production npm audits are 0. The first full CI attempt hit three transient shared Central mutation-lock timeouts; each exact test passed, the full Rust suite passed (1482/7 ignored), then the complete `just ci` rerun and `just audit` passed. No unrelated Rust source fix was made.

## Batch 4 — Medium: Pinned Toolchain And GitHub Actions

- [x] pnpm 10.12.3 -> 当前 10.x（扫描时 10.34.5），同步 `packageManager`、workflow setup version、doctor 与 contract tests；不升级 pnpm 11。
- [x] Rust 1.97.0 -> 1.98.0，同步 `rust-toolchain.toml`、所有 workflow rust-toolchain SHA/comment 与 contract tests。
- [x] 逐个升级 Action full SHA：checkout 7、setup-node 7、pnpm/action-setup 6、upload/download artifact 7/8、attest 4、Azure login 3、artifact signing 2；保持 Pages action 已在当前 major 时只做 SHA patch。
- [x] 审查 checkout v7 的 fork checkout 默认拒绝、artifact 名称/覆盖语义、release/context/signing job 权限与 runner 最低版本。
- [x] 聚焦：workflow/release/developer-experience contract tests、`just doctor`、YAML parse。
- [x] 完整门槛：`just ci` + `just audit`。Hosted Actions 实际运行保持 `UNVERIFIED`，因为本任务无 push 权限。

Batch 4 evidence (2026-08-30): pnpm 10.34.5, Rust 1.98.0, and reviewed GitHub Action full-SHA/current-major pins were synchronized across workflows, doctor, contracts, and bilingual/public docs. The removed attest predicate subaction was migrated to direct `actions/attest` v4 SLSA provenance with `create-storage-record: false`; release order and least-privilege permissions were preserved. Rust 1.98 exposed eight new `chunks_exact_to_as_chunks` Clippy errors; seven source files received behavior-equivalent `as_chunks` migrations with remainder/error/byte-order behavior independently reviewed. Focused tests, doctor, YAML/contracts, just check, Clippy, complete `just ci`, `just audit`, and diff checks passed under isolated pnpm 10.34.5 plus pre-existing stable Rust 1.98. Hosted Actions/Azure signing/release remain `UNVERIFIED`.

## Batch 5 — Medium Breaking: Test And Compiler Stack

每个子批独立运行完整门槛；不得三项一次升级。

- [x] 5A `@testing-library/jest-dom 7`：添加其要求的 `@testing-library/dom` direct peer，确认 Vitest setup import，替换任何在升级后暴露的 deprecated matcher（当前扫描为 0）。
- [x] 5B `jsdom 30`：重点验证 PointerEvent、matchMedia、ResizeObserver、scrollIntoView、Storage 与 location fixture seam。
- [ ] 5C `TypeScript 7`（DEFERRED）：TS 7.0.2 stable 不提供仓库所需的旧 programmatic API，且 typescript-eslint 8.68.0 peer 明确要求 TypeScript `<6.1.0`。不引入未批准的 TS7 + TS6 双编译器架构。
- [ ] 每个子批：对应聚焦测试 + `just ci` + `just audit`。

Batch 5A evidence (2026-08-30): jest-dom 7.0.1 and direct Testing Library DOM 10.4.1 use a single resolved DOM graph; Vitest setup was corrected to the explicit `/vitest` entry. Deprecated matcher scan was 0, all 177 frontend test files, typecheck, lint, build, full/production npm audits, complete `just ci`, and `just audit` passed. The first full CI exposed a recurrent Central contention-test setup timeout. Root cause was a test holder acquiring both the real file lock and the process-global test mutex; the holder now uses the raw file-lock guard while the operation under test retains the real target guard. Exact test 20/20, Central Updates 163/2 ignored, full Rust lib 1468/7 ignored, Clippy, independent review, and the complete CI rerun passed. The holder/operation guard ownership rule was strengthened in `backend/central-mutation-lock.md`.

Batch 5B evidence (2026-08-30): jsdom 30.0.1 is a single resolved version and its Node engine is satisfied by Node 26.7. PointerEvent is native; existing conditional matchMedia/ResizeObserver/scrollIntoView shims remain necessary; Storage, URL/history, and non-configurable location seams passed. Independent review fixed one UI race by waiting for the rendered alert instead of only the mock call. Focused tests, all 177 frontend files (1966/1 skipped), typecheck, lint, build, full/production npm audits, complete `just ci`, `just audit`, and diff checks passed.

Batch 5C evidence (2026-08-30): direct TypeScript 7 replacement was not performed. Independent verification confirmed TS 7.0.2 is stable, but typescript-eslint 8.68.0 requires TypeScript `<6.1.0`, TS 7.0 lacks the programmatic API used by `capabilitycheck`, and the official compatibility path requires a separately approved dual-compiler architecture. No files changed; TS6 deprecated-option scan was 0 and typecheck, lint, capabilitycheck, focused contracts, complete `just ci`, and `just audit` passed. Unblock requires official peer/API support or explicit approval for the dual-toolchain design.

## Batch 6 — Medium To High Breaking: Leaf Rust Libraries

每个子批独立完整门槛。

- [x] 6A `base64 0.22 -> 0.23`：当前使用 `STANDARD` + `Engine`，验证 minisign fixture 编解码字节不变。
- [x] 6B `sha2 0.10 -> 0.11`：适配 digest 0.11/newtype 变化，运行 checksum/migration/archive/update snapshot tests，逐字节比较历史 hash。
- [x] 6C `zip 2 -> 8`：适配 removed/deprecated API，运行所有 local archive/GitHub archive hostile fixture、路径逃逸、symlink、预算、Stored/Deflated tests。
- [x] 每个子批：`cargo fmt`、聚焦 Rust tests、`just ci` + `just audit`。

Batch 6A evidence (2026-08-30): direct base64 0.23.1 explicitly disables new default `simd-unsafe` and enables only `std`, preserving the prior alloc/std feature surface. `STANDARD`/`Engine` remained compatible. Hardcoded historical minisign wrappers, pre-upgrade raw bytes, and independent verification of `b"test"` passed non-tautological byte-equivalence review. Focused verifier/signing tests, fmt, Clippy, feature/inverse/duplicate trees, complete `just ci`, `just audit`, and diff checks passed; transitive base64 0.22.1 remains only in upstream closures.

Batch 6B evidence (2026-08-30): direct sha2 0.11.0 disables unused default features and uses one crate-owned lowercase byte encoder after digest 0.11 removed `LowerHex`; transitive sha2 0.10.9 remains only in SQLx/Tauri closures. Published empty/`abc` and leading-zero vectors, seven migration checksums, GitHub snapshot, local archive, Central mutation/update/recovery, and Skills CLI digests preserve exact historical bytes, framing, lowercase text, prefixes, and truncation. Independent review added fixed Central manifest, recovery-log file fingerprint, and path-token vectors and removed the unused `const-oid` closure. The first complete CI attempt failed only because the new inline test raised a production file from 800 to 803 lines; the test was moved to a dedicated same-module test file without changing the budget or behavior. Exact regression, size gate, fmt, Clippy, focused suites, dependency trees, complete `just ci`, `just audit`, and diff checks then passed; no new SHA2 advisory was reported.

Batch 6C evidence (2026-08-30): direct zip 8.6.0 uses only `deflate-flate2-zlib-rs`; updater-owned zip 4.6.1 remains transitive. Existing read/write APIs compiled unchanged, while tests exposed zip 8 raw-filename deduplication before caller enumeration. A minimal classic central-directory boundary/count verifier now anchors the real EOCD, rejects shadow/fake-size EOCD, multi-disk, count/offset/length/trailing-data mismatches, and fails closed on ZIP64, which is unnecessary under the unchanged 20,000-entry budget. NUL, traversal, Unix special entries, duplicate names, CRC/corruption/truncation, Unicode/CP437/non-UTF8, Stored/Deflated, and size/ratio budgets are covered without adding a full parser or new direct dependency. Independent review found and closed the comment-shadow bypass and removed the unintended Zopfli closure. Local archive 45/45, GitHub archive snapshots, fmt, Clippy, size, feature/tree/audit/diff checks, complete `just ci`, and `just audit` passed; no new zip advisory was reported.

## Batch 7 — High Breaking: Runtime Boundaries

每个依赖单独升级、单独完整门槛；若迁移扩张为业务重构则回退并拆新任务。

- [ ] 7A `reqwest 0.12 -> 0.13`（DEFERRED）：审查 ClientBuilder/TLS/proxy/redirect/error changes，验证 GitHub import redirect、auth isolation、no-proxy、retry、timeout 与 error explanation。
- [ ] 7B `keyring 3 -> 4`（DEFERRED）：按 keyring-core/native store 新结构选择最小跨平台 features，保持 SecretStore 与 DPAPI/session fallback，不扩大明文路径。真实 credential read-back 标为 `UNVERIFIED`。
- [ ] 7C `sqlx 0.8 -> 0.9`（DEFERRED）：处理 `SqlSafeStr`、dynamic query、SQLite feature/runtime 与 error API；完整运行 migration、integrity、recovery journal、repository 和 E2E tests，并检查与 `tauri-plugin-sql` 的重复 SQLx 版本。
- [ ] 每个子批：聚焦 tests + `just ci` + `just audit`。

Batch 7A evidence (2026-08-30): a reqwest 0.13.4 trial migration was fully reverted with no net manifest, lock, or source diff. The application direct reqwest 0.12.28 currently resolves native TLS while retaining charset, HTTP/2, system proxy, and an explicit rustls feature; `tauri-plugin-updater` independently uses reqwest 0.13.2 with rustls/ring. Cargo would unify both consumers on one reqwest 0.13 feature set, forcing either the application or updater to change TLS provider/root-verifier behavior. The plugin command path exposes no global client hook, so preserving both boundaries requires an updater refactor/fork, forced dual versions, or an explicitly approved TLS behavior change. After exact rollback, all-target locked Cargo check, fmt, and diff checks passed. No full CI/audit was repeated because the batch left no change; the prior stable Batch 6C full gates remain the baseline.

Batch 7B evidence (2026-08-30): a keyring 4.2.0 `v1` trial migration was fully reverted with no net manifest, lock, or source diff. Independent review established that the existing `keyring = "3"` declaration enables no native-store feature and therefore uses the process-local mock store; keyring 4 `v1` would newly activate Windows Credential Manager, macOS Keychain, and Linux Secret Service. Even though the Windows target remains `{user}.{service}` and the API/MSRV are compatible, this is a credential persistence product migration rather than a safe dependency-only upgrade. Unlock requires explicit approval plus real three-platform migration, failure/fallback, and existing-entry evidence. No real user credential was read, written, or deleted. After exact rollback, all-target locked Cargo check, fmt, and diff checks passed; the prior stable Batch 6C full gates remain the baseline.

Batch 7C evidence (2026-08-30): a SQLx 0.9.0 trial migration was fully reverted with no net manifest, lock, schema, or source diff. The 6 lifetime API changes and 48 `SqlSafeStr` sites were classified without finding user-controlled SQL tokens, and the SQLx 0.9/tauri-plugin-sql 0.8 graph could share `libsqlite3-sys`; however, the required compatibility edits touch migration v1/v2 files whose published checksum source is their own file text. Updating those checksums or refactoring the source contract would rewrite immutable migration history and exceeds a dependency-only batch. Unlock requires an independent design that separates executable migration compatibility from frozen checksum source, followed by existing-database validation. After exact rollback, migration checksum tests passed 10/10, all-target locked Cargo check, fmt, and diff checks passed, and the dependency graph returned to SQLx 0.8.6 only.

## Deferred Dependencies

- [ ] VitePress 2 alpha：等待 stable/beta 与正式 migration guide。
- [ ] pnpm 11：独立工具链任务评估 lockfile/config/CI 行为。
- [ ] Specta/Tauri-Specta：当前锁定 v2 RC 兼容组；crates.io 的 1.x stable 不是升级目标，等待兼容的 v2 stable。
- [ ] 无稳定修复且不在实际 bundle/import path 的 Lobe UI Mermaid/DOMPurify lock warning：记录并跟踪上游；若需要彻底移除，另开品牌图标依赖瘦身任务。

## Final Integration Gate

- [x] 再次运行 `just ci` 与 `just audit`。
- [x] 运行 `just build`，确认 `outputs/` 中最新 Windows NSIS 存在并记录文件名/大小/hash。
- [x] 汇总最终 direct versions、剩余 advisory、例外到期日、跳过/未验证平台面。
- [x] `git diff --check` 与工作树归属审计；不提交、不 push、不归档，等待用户单独授权。

Final integration evidence (2026-08-30): the first final CI attempt exposed two parallel-test isolation defects, not dependency API regressions: a redaction assertion used the common substring `301`, and remote target mutation tests wrote lock files under the real user app-data directory. The redaction fixture now uses complete high-entropy sentinels; SSH/WSL target tests derive process-scoped temporary lock paths while Local production-path contention coverage remains intact. Exact regressions, Central mutation tests, and the full Rust lib suite passed (1480/7 ignored), then complete `just ci` and `just audit` passed. `just build` produced `outputs/SkillPort_1.0.2_x64-setup.exe`, 13,776,251 bytes, SHA-256 `F9A20D3E3B37C3A736F72269AB3938A99677C6C3ACF857295D7B6127EA82C1C3`. The build proves an unsigned local Windows x64 NSIS bundle only; hosted Actions, Authenticode, updater signing/metadata, Azure signing, real credential stores, macOS/Linux native stores, and live provider/production delivery remain `UNVERIFIED`.
