# 父任务最终跨子任务验收

日期：2026-07-19  
基线：`dev` / `36b893d2` 之后的统一 ZIP 验收修复工作树  
模式：Codex inline；父任务保持 `planning`，未运行 `task.py start`

## 结论

2026-07-18 首轮父任务验收发现统一 ZIP 导入存在 overwrite rollback、Operation Log、Zustand 状态所有权、测试和错误脱敏/i18n 缺口。用户授权后，原 `07-18-unified-skill-import` 子任务从 archive 恢复并只启动一次；没有创建第五个任务，其他三个归档子任务未修改。

上述缺口现已修复并通过定向测试、全量 `just ci` 和真实 Tauri Operation Logs / Skill Detail 视觉复核。统一 ZIP 子任务已满足提交和归档条件；在该子任务归档前，父任务仍暂不可归档。子任务归档后，父任务将达到 4/4，并可进入用户授权的最终归档动作。

## AC 对照

| 父任务 AC | 状态 | 证据 |
| --- | --- | --- |
| 四个子任务完成审批、实施、验证和归档 | Pending lifecycle | TreeRaw、排版和 deep-link 已归档；统一 ZIP 的代码与验收已 green，等待本轮原子提交和归档。 |
| 统一入口复用 GitHub wizard，GitHub/ZIP 预览后写入 | Passed | `SkillImportLauncher` 只分发 intent；GitHub 继续使用现有 wizard；ZIP preview 只读，import 只由显式确认触发。 |
| TreeRaw 快路径与 archive 回退保持既有契约 | Passed | 归档子任务证据记录 15 个 `tree_fast_path` 测试、parity、selected-subtree union、typed fallback 和无 cache 决策。 |
| 深链只传递 intent，Windows cold/warm 可用 | Passed | 归档证据包含真实 NSIS scheme、cold/warm 预填、同一 PID 和窗口恢复/聚焦；native handler 不执行 Preview/Confirm/import。 |
| 排版密度、主题、accent 和三档缩放 | Passed | 归档证据覆盖 6 主题 x 14 accent、0.875/1/1.125 scale、多个 viewport 和 Central 虚拟化。 |
| 每个子任务通过定向测试和集成门禁 | Passed | 统一 ZIP：36 Rust tests、118 focused frontend tests、typecheck、lint、clippy、diff check 通过；第二次完整 `just ci` 通过。其他子任务沿用各自归档证据。 |
| 最终跨表面无语义或视觉回退 | Passed | Central/Marketplace GitHub、SSH/WSL、Usage、Operation Logs、Skill Detail 相关测试通过；真实当前工作树 Tauri Operation Logs 与 Skill Detail 截图无空白、重叠或遮挡。 |

## 统一 ZIP 修复证据

- overwrite DB failure 删除 replacement 后恢复 backup；rollback failure 以 typed `rollback_failed` 返回。
- preview 零写入、fingerprint mismatch、overwrite/rename/skip、DB failure restore、cleanup 和 unknown repository source 均有端到端测试。
- ZIP 安全矩阵覆盖 traversal、absolute/drive/UNC、duplicate/case/prefix conflict、symlink、encrypted、unsupported compression、archive/file/entry/expanded/compression-ratio budgets。
- command 边界记录 `central / local_archive.import` 成功/失败 Operation Log，错误只记录 stable code。
- IPC 使用 `local_archive.<code>:<safe summary>`；前端只保存 code 并映射中英文 i18n，不显示 raw path/entry/DB/IO payload。
- `useLocalArchiveImportStore` 独占 ZIP 状态和 typed IPC actions；wizard 不直接调用 IPC。
- 完整命令输出、首次 `just ci` 并行锁干扰、确认重跑和真实 Tauri 视觉步骤见 `unified-import-revalidation.md`。

## 最终验证结果

```text
cargo test local_archive_import
  36 passed

focused Vitest
  5 files passed; 118 tests passed

pnpm typecheck / pnpm lint / cargo clippy -- -D warnings / git diff --check
  passed

just ci (confirmation run)
  frontend: 127 files passed; 1397 passed; 1 skipped
  Rust: 874 passed; 4 ignored
  all checks passed
```

首次 `just ci` 有 8 个无关的 `central_skills` / `central_updates` 并行 mutation-lock 失败；三条代表用例单独复跑通过，第二次相同全量命令通过。失败事实保留在 `unified-import-revalidation.md`，未改写为通过。

## 真实视觉证据

- `operation-logs-tauri-final.png`：当前工作树 Tauri WebView，本机 1150 条 Operation Logs，统计、筛选、热力图和列表正常。
- `skill-detail-tauri-final.png`：当前工作树 Tauri WebView，真实 `academic-figure` detail 的 Markdown、metadata、安装状态和 repository 信息正常。
- `visual-validation-tauri-cdp.stderr.log` / `.stdout.log`：`pnpm tauri dev` 实际编译和运行日志。

浏览器 fixture 截图不计入通过证据；最终结论只使用 Tauri dev WebView2 CDP 产生的两张桌面截图。

## 已归档子任务证据

- TreeRaw：`../../archive/2026-07/07-18-github-import-manifest-fast-path/research/archive-baseline.md`
- 排版：`../../archive/2026-07/07-18-dense-typography-wcag/research/visual-validation.md`
- 深链：`../../archive/2026-07/07-18-skillport-import-deep-link/research/windows-validation.md`

## 生命周期边界

1. 按后端、前端、Trellis/验收证据原子提交本轮修复。
2. 归档 `07-18-unified-skill-import` 并记录 journal。
3. 父任务继续保持 `planning`；只在用户明确授权父任务归档后执行 archive。
