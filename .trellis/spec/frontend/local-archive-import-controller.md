# Local Archive Import Controller Contract

## Scope

适用于 Central 的统一 Add Skill launcher 与本地 ZIP wizard。GitHub/deep-link 继续进入 `importIntentStore`；local ZIP 使用独立 controller，不共享或复制 GitHub wizard 状态。

## State Ownership

`useLocalArchiveImportStore` 是唯一业务状态所有者：

```ts
type Step = "choose" | "preview" | "importing" | "result";

interface LocalArchiveImportState {
  isOpen: boolean;
  step: Step;
  archivePath: string | null;
  preview: LocalArchivePreview | null;
  previewError: string | null;
  importResult: LocalArchiveImportResult | null;
  importError: string | null;
  resolution: "overwrite" | "rename" | "skip";
  renamedSkillId: string;
}
```

- Store actions 是 `preview_local_skill_archive` / `import_local_skill_archive` 的唯一调用方；两个命令必须登记在 `IPC_COMMANDS`。
- Wizard 只负责 native file picker、store action 调用和渲染，不持有平行 preview/import state。
- Launcher 只发送 `github | local_zip` intent；remote target 的 local ZIP item disabled。
- 关闭 wizard 必须完整 reset；成功 import 后刷新 Central，但不自动打开平台安装。
- Preview 必须显示 archive basename、skill metadata、冲突、计数和完整相对文件树。

## Error Contract

- Store 捕获 backend error 后只保存 `local_archive.*` code；无 code/未知 code 统一为 `local_archive.unknown`，不保存 raw payload。
- UI 使用 `backendErrors.local_archive.*` 中英文文案，显示 inline error 并 toast；禁止把 backend message、绝对路径或 entry payload插入文案。
- Store action 保持 rethrow，组件负责当前可见 surface 的反馈。

## Tests Required

- Launcher GitHub/ZIP 分流、remote disabled、Central 集成打开。
- File picker cancel/failure、preview、文件树、conflict resolution、import success/failure、Central refresh、close reset。
- Known/unknown error 都显示本地化安全文案且不出现路径、entry 或 secret payload。
- GitHub Central/Marketplace/deep-link/SSH/WSL 回归保持通过。

## Quality Check

- `pnpm vitest run src/test/components/central/LocalArchiveImportWizard.test.tsx src/test/pages/CentralSkillsView.shell.test.tsx src/test/pages/CentralSkillsView.github-import-preview.test.tsx`
- `pnpm typecheck`
- `pnpm lint`
- 搜索 `LocalArchiveImportWizard`，确认组件不直接 import IPC adapter/function。
