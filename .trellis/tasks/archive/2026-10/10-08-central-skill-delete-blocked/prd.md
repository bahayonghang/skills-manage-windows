# 分析并修复 Central skill 无法删除

## Goal

让 `fuck-my-shit-mountain` 和 `neat-freak` 能用现有的「删除 Central skill」完成删除。删除收尾必须能去掉 Windows 上的目录符号链接备份，并且不能跟着链接删掉它指向的 Central 目录。

## Background

这两条 skill 各有一条未结束的 `central_delete`，阶段是 `db_committed`，`last_error_code` 是 `cleanup_file`。manifest 只记录了指向 Central 目录的 Zed 目录符号链接。暂存时改名的是链接本身，Central 目录还在；数据库提交之后，收尾用 `fs::remove_file` 删除这个备份链接。Windows 对目录符号链接的文件删除接口返回 Access Denied。

因此再点删除会重试同一次收尾并显示 “This Central operation could not be completed.” 强制删除只接受 `prepared` 且备份和 marker 都已消失的操作，所以没有强制删除入口。

两条 skill 没有仓库成员，所以出现在未指派来源里。这只解释分组。

本机证据、磁盘类型和代码位置见 `research/root-cause.md`。

## Requirements

1. 本地删除收尾遇到目录符号链接备份时，只删除链接和对应 marker，链接目标目录及其内容保持不动。
2. 普通文件、文件符号链接、以及真实目录的现有清理方式保持不变。
3. 对已经停在 `db_committed` 且失败码为 `cleanup_file` 的删除，用户再次确认删除时，先把这次收尾做完，再按当前 Central 副本继续删除。
4. 强制删除的资格不变：阶段不是 `prepared`，或备份、marker 仍在时，仍然不能强制删除。

## Acceptance Criteria

- [x] Windows 目录符号链接作为删除备份时，收尾成功后链接和 marker 消失，目标目录仍在。
- [x] 文件符号链接仍走文件删除，真实目录仍整棵删除。
- [x] 与这两条 skill 相同形状的 `db_committed` 删除，再次确认删除可以先完成旧收尾，再删除当前 Central 副本。代码路径已核对；本机这两条 skill 要等包含该修复的构建后再点删除。
- [x] 强制删除仍然拒绝非 `prepared` 阶段，以及备份或 marker 仍在的操作。
- [x] 上述目录符号链接收尾有 Rust 测试，使用现有的 `test_support::symlink_dir`。

## Out of Scope

- 放宽强制删除，或改 Operation Logs 的对账规则。
- 修改 SSH/WSL 收尾脚本。Linux `rm -rf` 删除的是符号链接本身。
- 手工清理本机数据库、备份链接或 marker。
- 为 `cleanup_file` 补文案。
- 处理 `handoff` 的 `delete_finalize_collision`。
- 专门删除扫描残留的非 Central 备份 skill 行。链接去掉后，下一次对应代理扫描会按缺失路径清理。
