# Design

## Scope

本任务只修一层映射：`NoImportableSkills` 必须作为 reviewed IPC 失败到达向导和 Runtime Log。不改候选发现、不改 Skill 模型、不改预览采集。

## Current Data Flow

```text
Preview import
  -> preview_github_repo_import
  -> discover candidates
  -> empty set
  -> GithubImportError::NoImportableSkills
  -> to_ipc_error() = raw Display
  -> IpcError::from_legacy_boundary
  -> internal.unexpected
  -> wizard: "See runtime logs for details."
  -> recordIpcFailure details.error = { name, message, stack }
       code 丢失
```

`preview_github_repo_import` 不调用 `with_operation_log`。Operation layer 不会出现这条失败。这是既有只读预览契约，本任务保持不变。

## Target Data Flow

```text
NoImportableSkills
  -> ipc_error_code = github_import.no_importable_skills
  -> to_ipc_error() = github_import.no_importable_skills:<Display>
  -> IpcError::from_legacy_boundary
  -> code + reviewed static message
  -> wizard formatBackendError -> i18n
  -> recordIpcFailure details.error.code = github_import.no_importable_skills
```

## Backend

在 `GithubImportError::ipc_error_code` 的 Candidate discovery 分组加入：

```rust
Self::NoImportableSkills => "github_import.no_importable_skills",
```

`#[error("{}", NO_IMPORTABLE_SKILLS_ERROR)]` 保持原句。CLI 继续用 Display 映射 `CliApiError::NotFound`。

`IpcError::legacy_code_message` 增加：

```text
github_import.no_importable_skills
  => "This GitHub repository does not contain an importable skill."
```

`to_ipc_error()` 已有逻辑会输出 `github_import.no_importable_skills:<Display>`。`from_legacy_boundary` 只保留 code，用 reviewed 句替换 Display。动态细节不会进入 `IpcError.message`。

不改 `preview_snapshot_code()`。`NoImportableSkills` 不是 snapshot lifecycle，向导不得因此显示 “preview again”。

## Frontend

`src/lib/ipc/errors.ts` `canonicalMessage` 增加同一 code 和同一英文静态句，与 Rust `legacy_code_message` 逐字一致。

`src/i18n/locales/en.json` / `zh.json` 的 `backendErrors.github_import` 增加：

- en: `This repository has no importable skill. SkillPort needs a SKILL.md at the repository root or in a supported skills directory.`
- zh: `该仓库没有可导入的技能。SkillPort 需要仓库根目录或受支持的 skills 目录中存在 SKILL.md。`

向导已走 `formatGitHubImportError` -> `formatBackendError`。coded envelope 会命中 i18n。不要给这条失败加 PAT 提示；`looksLikeGitHubAuthGuidance` 只认 rate_limited / access_denied / configured_token_failed。

`recordIpcFailure` / `errorToDetails`：当 error 带有字符串 `code`（`IpcInvokeError`）时，details 写入 `code`。只抄静态 code，不抄 args 里的用户字符串。敏感 args 继续走现有 redaction。

## Branch

当前 `GitHubRepoImportWizardInput` 在 `githubBranch === ""` 时选中 `main`，Preview 发送 `"main"`。R4 / AC5 是回归断言，不是新 UI。无回归则不改分支控件。

## Compatibility

- 无 DTO、schema、命令签名变化。不跑 `pnpm docs:gen`。
- 现有 “uncoded preview failures keep historical message” 测试继续覆盖真正未编码的字符串。新增 coded envelope 测试，不改那条历史字符串用例的语义。
- 其他未编码变体本任务不补码。

## Spec

在 `.trellis/spec/backend/github-import-preview-contract.md` 增加 “Preview Domain Error IPC Coding” 场景，锁定：

- `NoImportableSkills` 必须有 `github_import.no_importable_skills`
- IPC message 是 reviewed 静态句
- Runtime Log 必须带该 code
- 不得把该失败收成 `internal.unexpected`
