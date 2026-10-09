# Rules 独立复核

日期：2026-10-08。范围：Rules 新增前后端、文件安全、IPC、草稿、UI 和用户授权的 pnpm 版本策略修改。

## 已修复问题

| 文件 | 问题 | 修复与证据 |
| --- | --- | --- |
| `src/components/rules/RuleEditor.tsx` | 固定标题、模式和工具区的总高度超过详情高度时，工具操作被 AppShell 裁切 | 详情区内部滚动，正文保持最小可编辑高度。独立查看主会话复验截图 `tmp/rules-ui/en-900x600-max-scale-tools.png`，OMP 操作可通过详情滚动到达；默认 1200×800 截图无裁切 |
| `src/stores/rulesStore.ts` | Local→远程→Local 后晚到 mutation 仍被接受；旧 save 可清除草稿，旧 finally 可清除新 pending | 订阅目标变化并更新 generation。成功、失败、刷新及 cleanup 核对捕获的 generation。草稿保留。新增 2 项延迟响应回归，覆盖 Rules 页面未挂载时切换目标 |
| `services/rules/files.rs` | 保留名称和名为 `.md` 的嵌套目录进入中央扫描，导致整页加载失败 | 扫描排除非法/保留名称和目录。原对象保留；直接写入仍校验名称。新增扫描回归 |
| `services/rules/mod.rs` | 一个工具使用自定义根时，另一个工具的有效导入也被拒绝 | 导入预览只读取支持的工具。新增隔离 home 单工具回归，断言未创建 OMP 目录 |
| `services/rules/format.rs`、`mod.rs`、`lifecycle.rs` | 外部移除中央文件的 `alwaysApply: true` 后，文件仍被判定可写和可启用 | 中央共享格式单独要求 `alwaysApply: true`；缺失时只读并拒绝保存/启用。Claude 普通正文来源仍可导入。新增外部移除头部回归 |
| `services/rules/files.rs`、`lifecycle.rs` | 恢复和写入把任意 metadata 错误视为缺失；恢复直接入口的 receipt 校验弱于列表入口 | 只有 `NotFound` 表示缺失。receipt 共用 UUID、名称、phase、工具数量/唯一性和字段组合校验。未知 phase、重复工具回归断言原链接保留 |
| `services/rules/files.rs` | Windows 设备名称遗漏 `COM¹`、`COM²`、`LPT³` 等上标数字 | 名称校验覆盖上标数字和多重扩展名；既有名称测试增加 3 项输入。依据 [Microsoft 文件命名规范](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) |
| `commands/rules.rs` | 含 mutation bool 的公共执行器使 6 个 mutation 边界缺少明确的 typed lifecycle 调用，IPC 契约失败 | 拆分读取执行器和写入 `run_operation`。复用 Local 校验；写入执行器实际调用现有 observability primitive，持有现有 Local guard。保留 IPC 契约检查，未扩大 allowlist |
| `commands/rules.rs` | 全部导入条目失败时摘要没有区分零成功 | 摘要明确全失败、部分成功、全成功；保留现行批次 DTO 的 `Partial` API。计数回归覆盖 3 种结果组合，不记录文件名或正文 |

## 首次失败与修复后复验

未将修复前失败记为通过。

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked services::rules
修复前新增回归：15 passed; 4 failed; 0 ignored; 1568 filtered out

imports_from_supported_tool_when_other_tool_uses_custom_root
  left: Unsupported; right: Imported
central_files_without_always_apply_metadata_are_read_only
  left: Supported; right: Unsupported
scans_skip_reserved_files_and_nested_rule_directories
  Err(TargetConflict)
corrupt_receipt_phase_and_duplicate_tools_block_recovery
  expected Err(RecoveryBlocked)

扩大 IPC 契约检查：PASS (94) FAIL (1)
  ipc command coverage ratchet keeps registered operation commands on the typed lifecycle owner
修复后同一 9 文件检查：PASS (95) FAIL (0)

修复后 Rules：19 passed; 1583 filtered out; 0.35s
```

首轮原始输出保存在当前机器的 RTK tee：
`1791511877_cargo_test.log` 与 `1791512146_vitest_run.log`。

## 验证

| 命令 / 证据 | 状态 | 边界 |
| --- | --- | --- |
| `pnpm typecheck` | PASS，exit 0 | 最新 reviewer 修改后的 TypeScript |
| `pnpm lint` | PASS，`ESLint: No issues found` | 最新 reviewer 修改后的生产和测试源码 |
| `pnpm exec vitest run`：Rules store、Rules page、Sidebar、IPC coverage、i18n、frontend architecture、Rust boundary、developer experience、doctor 共 9 文件 | PASS，95 tests | 包括 2 项新增旧 mutation 回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked services::rules` | PASS，19 tests | 隔离 home；实际 Windows 文件链接、双入口更新、原字节备份、占用文件、部分删除/停用和恢复 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked commands::rules::tests` | PASS，1 test，exit 0 | 3 种导入结果组合的安全摘要与统计 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS，exit 0 | 最新 Rust 源码 |
| `git diff --check` | PASS，exit 0 | 当前 tracked diff；新文件另由格式和语言门槛检查 |
| pnpm 配置与文档复核 | PASS | 无 repository pnpm pin，Actions 使用 latest；Node/Rust pin 保留；workspace overrides 与未修改 lockfile 一致，run/exec 不隐式安装 |
| UI 独立截图检查 | PASS | 已阅 `en-1200x800.png`、`en-900x600-max-scale-tools.png`；其余尺寸/语言/状态证据由主会话浏览器探针记录 |
| `just ci`、Windows NSIS 构建 | 主会话负责 | 定向检查不替代正式门槛 |

使用 `CARGO_TARGET_DIR` 指向隔离临时编译目录。未修复或更改仓库旧 target 链接、ACL、系统安装或 PATH。未改真实 Claude/OMP 规则目录，未初始化真实 `.skillport`。

## 验收覆盖与剩余证据

- AC1、AC3、AC8、AC9：页面/store/导航测试与主会话浏览器证据；晚到 mutation 和小窗口滚动问题已修复。
- AC2、AC4、AC5、AC6：Rust 临时 home 验证正文/完整备份、重复导入、真实双工具文件链接、回读更新、外部链接保留和恢复受阻。真实用户 6 个文件没有被导入或接管。
- AC7：主会话已有隔离 Claude CLI 初始化发现与实际 OMP helper/parser/bucket 链接发现证据。模型行为遵守规则、provider 调用仍为 `UNVERIFIED`。
- AC10：完整 `just ci`、新 NSIS 输出等待主会话证据。原生 Tauri WebView/IPC 交互、安装/升级/卸载、hosted CI、Linux/macOS 仍须分别报告。浏览器 fixture 不证明这些层。
- 实际 symlink 权限不足场景未切换系统开发者模式或提权配置。当前证据为实际链接成功、Windows 文件占用失败、错误映射和浏览器权限错误状态。

没有未修复的确定产品问题。任务保持 `in_progress`；未提交、push、PR、归档或发布。
