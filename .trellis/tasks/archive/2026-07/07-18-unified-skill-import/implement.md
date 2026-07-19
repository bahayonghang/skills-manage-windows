# Implementation Plan: 统一 ZIP 导入验收修复

## 1. 执行边界

- 本任务是 2026-07-18 已归档交付物的原范围修复；2026-07-19 恢复为 `planning`。
- 已审批依赖保持不变：直接 `zip 2.4.2`（仅 `deflate`）与现有 `sha2 0.10`；不新增生产依赖。
- ZIP 仍只支持本机 Central；SSH/WSL 保持 disabled。不得修改 GitHub DTO、deep-link、数据库 schema 或扩展为多 skill/archive 格式。
- Codex inline 模式由主会话直接 implement/check。先写失败测试，再改实现。

## 2. 后端修复

1. 在 `local_archive_import/tests.rs` 建立真实临时 Central + DB 端到端 harness，先复现 overwrite DB failure 丢失旧目录与 backup 遗留。
2. 修正 rollback 顺序和错误传播；覆盖 overwrite/rename/skip、staging/backup cleanup、fingerprint mismatch 和无 repository assignment。
3. 先补 Operation Log 成功/失败测试，再在 command 层接入 best-effort 记录；日志只保存 stable code、安全 subject 与结构化计数。
4. 把 IPC error 收敛为 stable code + safe summary；不得回显绝对路径、ZIP entry、fingerprint 或 DB/IO payload。
5. 补 inventory 缺失 fixtures：symlink、encrypted、unsupported compression、文件数/压缩字节/展开字节/单文件与压缩比预算。

## 3. 前端修复

1. 先新增 `LocalArchiveImportWizard.test.tsx`，覆盖 launcher GitHub/ZIP 分流、remote disabled、file picker cancel、preview、冲突 resolution、失败、成功与关闭 reset。
2. 将 step、archive path、preview、resolution、rename、pending/result/error 收敛到独立 Zustand store/controller；wizard 只负责 file picker、action 调用和渲染。
3. 复用 `formatBackendError`，补齐中英文 `backendErrors.*`；known/unknown backend error 均不得显示 raw payload。
4. 保持 GitHub wizard、deep-link pending intent 和 Central refresh 行为不变。

## 4. 跨表面验收

1. 重跑普通 Central/Marketplace GitHub import 与 SSH/WSL target 回归。
2. 重跑 Operation Logs、Usage 和 Skill Detail 测试。
3. 在父任务 `research/` 保存 Operation Logs 与技能详情最终视觉证据；未运行项不得写成通过。
4. 使用 trellis-check 做全范围复核；全部子任务与父任务 AC 有证据后，才重新归档本子任务并返回父任务最终审查。

## 5. 验证命令

```powershell
pnpm vitest run src/test/LocalArchiveImportWizard.test.tsx src/test/CentralSkillsView.shell.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx src/test/OperationLogsView.test.tsx src/test/SkillDetailView.test.tsx
pnpm typecheck
pnpm lint
cd src-tauri; cargo test services::local_archive_import
cd src-tauri; cargo clippy -- -D warnings
git diff --check
just ci
```

## 6. 风险与原子提交边界

- 后端风险文件：`src-tauri/src/services/local_archive_import/*`、`src-tauri/src/commands/local_archive_import.rs`、Operation Log tests。
- 前端风险文件：`src/components/central/LocalArchiveImportWizard.tsx`、`SkillImportLauncher.tsx`、`src/stores/localArchiveImportSlice.ts`、中英文 i18n 和测试。
- Commit 1：后端 atomic rollback、safe errors、Operation Log 与 Rust tests。
- Commit 2：Zustand controller、wizard/i18n 与 Vitest。
- Commit 3：父任务验收证据与 AC 收敛。
