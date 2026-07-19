# 统一 ZIP 导入验收修复复核

日期：2026-07-19  
工作树基线：`dev` / `36b893d2` 之后的未提交修复  
运行模式：Codex inline；父任务保持 `planning`

## 修复结论

- overwrite 在 DB upsert 失败后先删除 replacement，再恢复 backup；rollback remove/rename 失败返回 `rollback_failed`，不吞错。
- `import_local_skill_archive` 在 command 边界记录 `central / local_archive.import` 成功或失败 Operation Log。日志仅记录 stable code、resolution 和计数，不记录 archive path、fingerprint、entry 或底层错误。
- IPC 统一返回 `local_archive.<code>:<safe summary>`；Zustand controller 只保存 code，前端映射中英文 i18n，未知错误不显示 raw payload。
- `useLocalArchiveImportStore` 是 ZIP preview/import 状态与 IPC action 的唯一所有者；wizard 只负责 native picker、dispatch 和渲染。
- preview/import 继续复用同一份 archive bytes 做 fingerprint 与完整安全重验；archive skill 保持 unknown/local repository assignment。

## 自动验证

```text
cd src-tauri; cargo test local_archive_import
  36 passed; 0 failed

pnpm vitest run src/test/LocalArchiveImportWizard.test.tsx src/test/CentralSkillsView.shell.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx src/test/OperationLogsView.test.tsx src/test/SkillDetailView.test.tsx
  5 files passed; 118 tests passed

pnpm typecheck
  exit 0

pnpm lint
  exit 0

cd src-tauri; cargo clippy -- -D warnings
  exit 0

git diff --check
  exit 0; only Windows LF/CRLF notices

just ci (first run)
  failed: 8 central_skills / central_updates tests contended on the shared Central mutation path
  local_archive_import tests passed in that run

cargo test services::central_skills::tests::test_delete_central_skill_rejects_non_central_skill -- --exact --nocapture
cargo test services::central_skills::tests::test_batch_delete_central_skills_dedupes_and_merges_copy_agents -- --exact --nocapture
cargo test services::central_updates::inventory::tests::apply_delete_missing_removes_skill -- --exact --nocapture
  each representative test passed in isolation

just ci (confirmation run)
  exit 0
  frontend: 127 files passed; 1397 passed; 1 skipped
  Rust: 874 passed; 4 ignored
  typecheck, lint, sizecheck, entrypoint check and clippy passed
```

首次 `just ci` 失败未被改写为通过；第二次相同命令 green，且三条代表失败用例单独复跑均 green。该现象记录为既有并行 mutation-lock 测试干扰，不作为本修复的伪通过依据。

## 真实 Tauri 视觉证据

启动当前工作树：

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9222'
pnpm tauri dev
```

Tauri dev 编译并运行 `target\debug\skillport.exe`，WebView2 CDP 页面为 `http://localhost:24200/dashboard`。通过该真实 WebView 操作，而非浏览器 fixture：

- 打开 Operation Logs，读取本机 `1150` 条记录，统计、筛选、热力图和列表均正常显示。
- 打开 Central 中 `academic-figure` 的 Skill Detail，Markdown、frontmatter、安装状态、repository metadata 和文件树均正常加载。
- 两个表面没有空白、文本重叠或控件遮挡；没有触发导入、更新、安装或删除。

证据文件：

- `operation-logs-tauri-final.png`
- `skill-detail-tauri-final.png`
- `visual-validation-tauri-cdp.stderr.log`
- `visual-validation-tauri-cdp.stdout.log`

早期浏览器 fixture 截图和失败的 14x14 捕获未计入通过证据，并已从验收目录删除。

## 生命周期边界

统一 ZIP 子任务的产品、测试、spec 与证据满足归档条件，并已通过 `94548fee` 重新归档。父任务现为 `planning [4/4 done]`，最终跨子任务验收已通过；父任务本身继续保持 planning，等待用户明确的归档指令。
