# 执行计划

## Checklist

1. 在 `src-tauri/src/services/central_operation/fs.rs` 的 `remove_any_path` 中，让 Windows 目录符号链接（`FileTypeExt::is_symlink_dir`，不要用 `Metadata::is_dir`）走 `fs::remove_dir`。其他符号链接和普通文件仍走 `fs::remove_file`，真实目录仍走 `fs::remove_dir_all`。错误码保持 `cleanup_file` / `cleanup_directory`。Unix 目录符号链接仍走 `remove_file`。
2. 在同文件测试中增加一条本地收尾：用 `crate::test_support::symlink_dir` 建目录符号链接，`stage_delete_local` 后再 `finalize_delete_local`。断言备份链接和 marker 消失，目标目录和其中的文件仍在。
3. 不改 `force_abandon.rs`、远程 `REMOTE_FINALIZE_DELETE`、对话框、i18n 和数据库。

## Validation

```bash
cargo test -p skillport --lib services::central_operation::fs::tests -- --test-threads=8
cargo fmt --all -- --check
```

Windows 上这条测试必须实际创建目录符号链接。若本机策略禁止创建，记录失败原因，不要改成跳过测试来假装通过。

用户侧验收要等包含该修复的构建：再次确认删除这两条 skill。预期旧收尾完成，Central 副本被删除。本规划不直接改本机技能数据。

## Spec follow-up

实现并验证后，在 `.trellis/spec/backend/fs-db-operation-journal.md` 补一句：本地收尾删除目录符号链接时使用不跟随目标的目录删除，禁止 `remove_dir_all`。

## Rollback

还原 `fs.rs` 的删除分支和新增测试。
