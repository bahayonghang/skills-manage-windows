# Implementation Plan

## Checklist

1. 后端补码
   - 在 `GithubImportError::ipc_error_code` 加入 `NoImportableSkills => github_import.no_importable_skills`。
   - 不改 `#[error]` Display，不改 `preview_snapshot_code`。
   - 在 `IpcError::legacy_code_message` 加入 reviewed 静态句。
   - 在 `github_import/tests.rs` 增加 envelope 断言：code、前缀、不泄漏路径/token。
   - 在 `ipc_error.rs` 测试加入该 code 的 from_legacy_boundary 断言。

2. 前端 IPC 与 i18n
   - `src/lib/ipc/errors.ts` `canonicalMessage` 加入同一 code 和同一英文静态句。
   - `en.json` / `zh.json` 增加 `backendErrors.github_import.no_importable_skills`。
   - 向导测试：coded envelope 显示 i18n 文案，不含 `See runtime logs`，不含 PAT 提示。
   - `ipc.test.ts` 增加该 coded string 的 normalize 用例。

3. Runtime Log 保留 code
   - `errorToDetails` / `recordIpcFailure` 在 `IpcInvokeError` 上抄写 `code`。
   - `runtimeLogger.test.ts` 用 `IpcInvokeError({ code: "github_import.no_importable_skills", ... })` 断言 details 含该 code。

4. 分支回归
   - 确认现有向导测试：默认选中 `main`，Preview 发送 `"main"`。
   - 无回归则不改 `GitHubRepoImportWizardInput.tsx`。

5. Spec
   - 更新 `.trellis/spec/backend/github-import-preview-contract.md`。
   - 需要时在 `.trellis/spec/backend/index.md` 保持现有链接，不新增独立 spec 文件。

6. 校验
   - 跑针对性 Rust / Vitest。
   - 跑 `just ci`。
   - 声明完成前写明未做桌面包复现 `HERO-Anti-OverDefense` 网络预览。

## Likely Files

- `src-tauri/src/services/github_import/error.rs`
- `src-tauri/src/services/github_import/tests.rs`
- `src-tauri/src/ipc_error.rs`
- `src/lib/ipc/errors.ts`
- `src/lib/runtimeLogger.ts`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`
- `src/test/components/marketplace/GitHubRepoImportWizard.test.tsx`
- `src/test/runtime/ipc.test.ts`
- `src/test/runtime/runtimeLogger.test.ts`
- `.trellis/spec/backend/github-import-preview-contract.md`

## Watch-Only Files

- `src-tauri/src/services/github_import/preview.rs`
- `src-tauri/src/services/github_import/types.rs`
- `src-tauri/src/commands/github_import.rs`
- `src/components/marketplace/GitHubRepoImportWizardInput.tsx`
- `src/components/marketplace/githubImportWizardUtils.ts`
- `src/stores/marketplaceStore.githubImportSlice.ts`

## Review Gates

- `NoImportableSkills` 不再变成 `internal.unexpected`。
- 向导内联错误是 i18n “no importable skills”，不是 Runtime Log 兜底句。
- Runtime Log details 含 `github_import.no_importable_skills`。
- `#[error]` Display 与 CLI NotFound 映射不变。
- 默认分支仍是 `main`。
- 无 schema / DTO / 命令签名变化。

## Validation Commands

```powershell
cd src-tauri; cargo test github_import:: -- --test-threads=1
cd src-tauri; cargo test ipc_error -- --test-threads=1
pnpm test -- src/test/components/marketplace/GitHubRepoImportWizard.test.tsx src/test/runtime/ipc.test.ts src/test/runtime/runtimeLogger.test.ts
just ci
```

## Rollback

只回退本任务列出的映射、i18n、Runtime Log `code` 字段和对应测试。不回退 `d676bff4` 分支单选。
