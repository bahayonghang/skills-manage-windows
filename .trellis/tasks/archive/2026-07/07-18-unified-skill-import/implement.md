# Implementation Plan: 统一入口与安全 ZIP 导入

## 1. 审批门

- 启动前确认 `zip` crate 的版本、许可和安全维护状态；SHA-256 复用现有 `sha2`，不新增第二个哈希依赖。
- 确认 MVP 远程行为仍为“SSH/WSL 禁用 ZIP”，不得在实现中静默扩张为上传协议。

## 2. 实施顺序

1. 为 ZIP inventory/candidate 写失败测试：根、wrapper、多候选、traversal、absolute、case collision、prefix collision、symlink、encrypted、budget、compression ratio。
2. 实现 typed error、纯 inventory 与 candidate 解析；复用 scanner frontmatter 和 skill id 规则。
3. 为 preview command 写“零写入”和冲突测试，实现带 SHA-256 + byte length fingerprint 的 preview DTO 与 command/store typed map。
4. 为 import 写 fingerprint 相同/内容变化/长度变化、overwrite/rename/skip、staging failure、backup restore、DB consistency、Operation Log 脱敏测试；fingerprint 不匹配必须早于 staging 和 Central mutation。
5. 实现 import service；所有 IO 放入 blocking FS 边界，有界读取 archive 一次并用同一 bytes 做 fingerprint 与完整校验。
6. 添加 `SkillImportLauncher`，把现有 GitHub CTA 接入，不改变 wizard 内部状态。
7. 实现 ZIP wizard 的 choose/preview/confirm/result 与 remote disabled 状态，同步中英文 i18n。
8. 补 Central 集成测试，确认 GitHub flow、刷新 Central、平台 InstallDialog 无回归，并锁定无 repository assignment 的 archive skill 在更新中心为 unsupported 而非 error/remote-missing。
9. 更新相关 frontend/backend spec，尤其记录 local archive 安全契约和统一 intent 边界。

## 3. 定向验证

```powershell
pnpm vitest run src/test/LocalArchiveImportWizard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx
pnpm typecheck
pnpm lint
cd src-tauri; cargo test services::local_archive_import
cd src-tauri; cargo clippy -- -D warnings
git diff --check
just ci
```

实际测试文件名以实现落点为准，但必须保留 GitHub wizard 现有回归集。

## 4. 风险文件

- `src/components/central/CentralSkillsShell.tsx`
- `src/components/central/CentralSkillDialogs.tsx`
- `src/stores/*Import*`
- `src/lib/ipc/commandMap.ts`
- `src-tauri/src/commands/mod.rs` / `src-tauri/src/lib.rs`
- `src-tauri/src/services/central_mutation*`
- `src/i18n/locales/en.json` / `zh.json`
- `src-tauri/Cargo.toml` / `Cargo.lock`

## 5. 回滚点

- Commit 1：后端 parser/preview/import + tests。
- Commit 2：frontend launcher/wizard/i18n + tests。
- 若后端安全门未通过，不提交或启用 frontend ZIP intent。
