# 目录符号链接删除收尾

## Boundary

行为差在 `remove_any_path`：它把所有符号链接都交给 `fs::remove_file`。Windows 上目录符号链接必须用目录删除接口，而且不能递归进目标。

修复只放在 `src-tauri/src/services/central_operation/fs.rs` 的本地收尾。不改阶段机、强制删除资格、远程脚本、IPC 或对话框。

## Removal rule

`remove_any_path` 继续先取 `symlink_metadata`。

- Windows 目录符号链接：`FileTypeExt::is_symlink_dir()` 为 true 时调用 `fs::remove_dir`。`Metadata::is_dir()` 对这些重解析点为 false，不能用来分流。`RemoveDirectoryW` 只去掉链接。失败仍记 `cleanup_file`。
- 其他符号链接或普通文件：继续 `fs::remove_file`，错误码仍是 `cleanup_file`。
- 真实目录：继续 `fs::remove_dir_all`，错误码仍是 `cleanup_directory`。

Unix 上 `symlink_metadata` 不会把符号链接报成目录，也没有 `is_symlink_dir`，所以仍走 `remove_file`，只 unlink 链接。不要用 `remove_dir_all` 处理符号链接，否则可能跟着链接删掉 Central 目录。

## Retry flow

不新增修复入口。现有删除已经会先恢复选中 skill 的未完成操作。

1. `db_committed` 再次进入 `finalize_delete_local`。
2. 原 Zed 路径已经不在，备份链接还在，marker 和指纹匹配。
3. 新的删除规则去掉备份链接和 marker，阶段变为 `completed`。
4. 同一次确认删除继续为当前 Central 目录创建新的删除日志。此时安装记录只剩 Central 本体，所以会删掉真正的目录。

强制删除仍由 `force_abandon.rs` 拒绝非 `prepared` 和仍有备份或 marker 的操作。

## Compatibility

- 不迁移数据库，不改 manifest 形状。
- 已写上的 `cleanup_file` 不用改名。修复后的下一次收尾成功就不会再读这个错误。
- 远程收尾脚本保持 `rm -rf -- "$backup"`。没有尾部斜杠时，它删除链接而不是目标。
- 路径去重保持现状。符号链接和它的目标在 canonicalize 后是同一路径，manifest 只保留先出现的链接；这正是 Central 目录还在的原因，也是收尾后还要再删一次当前副本的原因。

## Rollback

只回退 `remove_any_path` 及其测试。没有数据迁移可回滚。回退后，这两条操作会再次停在 `cleanup_file`。
