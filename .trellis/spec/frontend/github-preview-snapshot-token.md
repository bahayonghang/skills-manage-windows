# GitHub Preview Snapshot Token Contract（渲染层）

## Scope

适用于 GitHub 仓库导入 wizard 及其全部调用方（Marketplace 三 Tab、Central Add
Skill、deep-link/import intent、官方源预览）。后端把 preview 注册为不可变
snapshot，渲染层持有的 `previewId` 是唯一能读取/导入这份内容的凭证。

后端契约见
[GitHub Import Preview Contract](../backend/github-import-preview-contract.md)。

## State Ownership

`marketplaceStore.githubImportSlice` 是 `previewId` 的唯一所有者，随
`GitHubRepoPreview` 一起存放，不单独复制到组件本地 state：

```ts
interface GitHubRepoPreview {
  repo: GitHubRepoRef;
  skills: GitHubSkillPreview[];
  previewId: string;         // 必填，无 optional fallback
  resolvedCommitSha: string;
  snapshotDigest: string;
  expiresAt: string;         // RFC 3339
}
```

- `previewId` / `resolvedCommitSha` / `snapshotDigest` / `expiresAt` 以及
  `GitHubSkillPreviewFile.sha256` 在 TS 类型里都是必填。`previewWorkspaceId`
  及其 optional fallback 已删除，禁止恢复。
- `preview_github_repo_import`、`import_github_repo_skills`、
  `fetch_github_skill_markdown`、`discard_github_repo_preview_snapshot` 四个命令
  必须登记在 `IPC_COMMANDS`（已从 `UNTYPED_IPC_COMMANDS` 移出，受 ratchet 测试
  保护）。见 [IPC adapter 约定](./ipc-adapter.md)。
- 组件不直接 `invoke()`；`invoke("import_github_repo_skills")` 全仓只允许一个
  调用点，由 `src/test/contracts/githubPreviewSnapshotContract.test.ts` 断言。

## Discard 契约

token 不会跨应用重启存活，且后端只在远程 preview 创建时回收过期存储，因此渲染层
必须显式 discard，否则会留下临时目录：

| 时机 | 行为 |
| --- | --- |
| `resetGitHubImport` / wizard 关闭 | discard 当前 `previewId` |
| 发起新的 preview | 先 discard 旧 `previewId`，再请求新 preview |
| 切换 target | discard 当前 `previewId` |
| import 成功 | 后端已消费 token，清空本地 preview，不再重用 |
| import 失败 | **保留** preview 与 token，允许原样重试 |

- 只读列举场景（`previewGitHubRepoSkills`，供官方源预览用）拿到 preview 后立即
  discard：它不会 import，持有 token 只会占用存储。
- discard 是 fire-and-forget 的清理动作，失败不得阻塞用户当前操作。

## Error Contract

后端对 snapshot 生命周期失败返回 `github_import.<code>:<英文摘要>` 信封，其余错误
保持历史 Display 文案（PAT 引导等既有 toast 不受影响）。

- `formatGitHubImportError` 先 `normalizeMessage` 剥掉 `Error:` 前缀，再
  `formatBackendError`。顺序不能颠倒——stringified `Error` 否则匹配不到 code 而
  漏出原文。
- `formatGitHubImportToast` 只翻译 `github_import.preview*` 信封，其他 message
  原样返回；避免把恰好含冒号的无关文案（如 `skills.sh: ...`）截断。
- 六个 code 都必须有中英文案：`preview_missing`、`preview_expired`、
  `preview_mismatch`、`preview_integrity`、`preview_busy`、
  `preview_commit_unresolved`，统一落在 `backendErrors.github_import.*`，语义都是
  "请重新预览"。
- confirm 步骤失败时目标面板不会打开，inline error 区域也不在当前 step 上，
  所以必须走 toast——且必须经 `formatGitHubImportToast`，不能 `String(err)`。
  参见 [前端异步动作失败反馈约定](./async-error-feedback.md)。
- 禁止把 token、workspace 路径、digest 或文件内容插入任何用户可见文案或日志。

## UI 展示

- wizard 展示短 commit SHA 与本地化 `expiresAt`，让用户知道自己确认的是哪个快照。
- provenance/元信息行使用 `text-muted-foreground` 与既有 `text-destructive-text`
  token，不写 `dark:` 明暗二元适配，也不用原生 Tailwind 调色板表达状态色。

## Tests Required

- `src/test/contracts/githubPreviewSnapshotContract.test.ts`：零
  `previewWorkspaceId` 引用、零 `discard_github_repo_preview_workspace` 引用、
  单一 import 调用点。
- `src/test/components/marketplace/githubImportWizardUtils.test.ts`：coded 信封
  翻译、`Error:` 前缀、非 coded 文案原样透传。
- store 测试：reset/新 preview/target change/close 都发出 discard；import 成功
  清空 preview；import 失败保留 token 可重试。
- wizard 测试：短 SHA 与 expiry 渲染；六个 code 都显示中英"重新预览"文案。
- 官方源预览与 Central sync 调用方回归。

## Quality Check

- `pnpm vitest run src/test/contracts/githubPreviewSnapshotContract.test.ts src/test/components/marketplace src/test/stores/marketplaceStore.test.ts src/test/pages/MarketplaceOfficialPreview.test.tsx`
- `pnpm typecheck`
- `pnpm lint`
- `rg previewWorkspaceId src` 必须为零命中。
