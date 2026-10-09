# 为什么 `fuck-my-shit-mountain` 和 `neat-freak` 删不掉

## 结论

两次删除都已经进入 `db_committed`，收尾时要删掉的是 Zed 里的**目录符号链接**备份。`remove_any_path` 看到符号链接就调用 `fs::remove_file`。Windows 对目录符号链接的 `DeleteFile` 返回 Access Denied，错误码记成 `cleanup_file`。操作因此一直停在 `db_committed`。

再点删除会先重试这次收尾，再次失败，界面显示 “This Central operation could not be completed.” 强制删除只接受 `prepared` 且备份和 marker 都已消失的操作，所以这里没有强制删除入口。

## 本机证据

只读查询 `~/.skillsmanage/db.sqlite`，时间与截图一致（`fuck-my-shit-mountain` 的 `updated_at` 为 `2026-10-08T08:39:50Z`）。

| skill | operation | phase | last_error_code |
| --- | --- | --- | --- |
| `fuck-my-shit-mountain` | `b6c22b92-2396-469f-bf2a-8b5d38ef4e5a` | `db_committed` | `cleanup_file` |
| `neat-freak` | `930ed846-adc6-495e-bec1-31921f72daa1` | `db_committed` | `cleanup_file` |

两条 manifest 都只有 Zed 路径，没有 Central 目录：

- `C:\Users\lyh\.config\zed\skills\fuck-my-shit-mountain`
- `C:\Users\lyh\.config\zed\skills\neat-freak`

磁盘现状：

- 原 Zed 路径已经不存在。
- 备份仍在，类型是 `SymbolicLink`，属性 `Directory, ReparsePoint`，目标分别指向 `..\..\..\.skillsmanage\skills\fuck-my-shit-mountain` 和 `neat-freak`。
- 对应 `.skillport-operation-*.marker` 仍在。
- Central 目录本身仍是普通目录，技能行也被重新扫了回来。
- 备份链接被扫成另一条 skill（id 就是备份目录名），挂在 `zed` 上，`link_type=symlink`。
- 原 skill 现在只有 `central` / `native` 安装，没有仓库成员。这就是侧栏未指派来源计数为 2，以及对话框写着没有平台链接的原因。

临时目录复现：对目录符号链接调用 `File.Delete` 得到 “Access to the path is denied.”；`Directory.Delete` 可以删掉链接本身。

## 代码路径

1. 删除收集路径时先放 symlink 安装，再放 Central 目录。`build_local_delete_manifest` 用 canonicalize 后的路径去重。Zed 链接解析后等于 Central 目录，所以 manifest 只留下链接路径。见 `src-tauri/src/services/central_skills/delete.rs` 的路径收集，以及 `src-tauri/src/services/central_operation/fs.rs` 的 `build_local_delete_manifest`。
2. `stage_delete_local` 对链接做 `rename`，移动的是链接，不是 Central 目录。数据库提交后技能行被删掉，阶段变为 `db_committed`。
3. `finalize_delete_local_blocking` 要删掉备份。`remove_any_path` 对 symlink 走 `fs::remove_file`，错误码是 `cleanup_file`。见 `src-tauri/src/services/central_operation/fs.rs`。
4. 下次删除先走 `recover_selected_pending_operations_under_guard`。`db_committed` 会再次 finalize，再次 `cleanup_file`。`CentralSkillsError::CentralOperation` 被收成 “This Central operation could not be completed.” 见 `src-tauri/src/services/central_skills/delete/batch.rs` 和 `src-tauri/src/commands/skills.rs`。
5. 强制删除要求 `phase == prepared`，并且备份和 marker 都不存在。当前阶段不对，备份和 marker 还在，所以 `force_delete_eligible` 为 false。对话框只显示 `central.forceDeleteBlocked`，没有强制删除按钮。见 `src-tauri/src/services/central_operation/force_abandon.rs` 和 `src/components/central/DeleteCentralSkillDialog.tsx`。

`cleanup_file` 没有单独文案。预览把 `central_operation.cleanup_file:` 交给 `formatBackendError`，翻译缺失时就把这段原始码显示出来。

## 相邻但不是这次的对象

同库还有一条 `handoff` 的 `central_delete`，阶段也是 `db_committed`，错误是 `delete_finalize_collision`。那是原路径又出现了，和这两条的目录符号链接删除失败不是同一个错误。
