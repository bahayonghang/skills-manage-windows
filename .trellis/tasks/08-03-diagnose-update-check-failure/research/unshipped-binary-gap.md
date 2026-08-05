# 检查更新仍然失败：修复未进入用户运行的二进制

## 结论

R12 的 archive canonical redirect 状态机在工作区源码中已实现且覆盖全部真实跳转形状，但
用户 2026-08-04 23:07 运行的是 2026-08-03 21:03 编译的已安装 release 版本。该版本只包含
R1/R3（一跳 302 + 稳定错误码），不包含 R12（numeric 301 链 + owner/repo 大小写等价）。
24 个仓库中有 6 个只在 R12 下才被接受，因此仍稳定失败为
`github_import.archive_redirect_rejected`。这不是新的代码缺陷。

## 运行时证据

`C:\Users\lyh\.skillsmanage\logs\skillport-2026-08-04.log`：

| 时间（UTC）         | 错误                                                  |
| ------------------- | ----------------------------------------------------- |
| 2026-08-03T16:21:39 | `The operation failed. See runtime logs for details.` |
| 2026-08-04T01:01:11 | 同上                                                  |
| 2026-08-04T15:07:15 | `GitHub repository archive redirect was rejected.`    |

最后一次失败的前端 stack 引用 `assets/index-DszjRlTa.js`、`CentralSkillsView-C6TiaQ8i.js`、
`button-1DQcN2i4.js`。这三个哈希存在于 `C:\Users\lyh\AppData\Local\SkillPort\skillport.exe`
与 `src-tauri/target/release/skillport.exe`，不存在于 `src-tauri/target/debug/skillport.exe`。
因此该次运行使用的是已安装 release 版本。

## 二进制与源码时间线

| 时间（本地）           | 事件                                                 |
| ---------------------- | ---------------------------------------------------- |
| 2026-08-03 18:00–18:24 | `error.rs` / `ipc_error.rs` / `mod.rs` 修改（R1/R3） |
| 2026-08-03 21:03:39    | `skillport_lib.d` 写入，release 库编译完成           |
| 2026-08-03 21:06       | 已安装 `skillport.exe` 产生                          |
| 2026-08-03 23:46:15    | `raw_http.rs` 修改（`GitHubEndpointProvenance`）     |
| 2026-08-03 23:57:52    | `archive.rs` 修改（numeric 301 状态机、大小写等价）  |
| 2026-08-04 00:14       | 仅 debug 构建 `target/debug/skillport.exe`，未安装   |

已安装二进制包含字符串 `GitHub repository archive redirect was rejected`（2 处），证明它含
R1/R3。R12 的两个源文件修改时间晚于 release 链接时间 2 小时 40 分以上，因此不在该二进制中。

`git show HEAD:src-tauri/src/services/github_import/archive.rs` 不含 `MOVED_PERMANENTLY`、
`CodeloadIdentity`、`eq_ignore_ascii_case`、`repositories`；整个 428 行 archive 修复目前只存在
于工作区，未提交。

## 真实跳转形状复测（2026-08-04，未认证、禁用自动跳转）

对 `skill_repositories` 中 24 个 `source_type='github'` 仓库逐个请求
`https://api.github.com/repos/{owner}/{repo}/tarball/{branch}`：

| 形状                                                                    | 数量 | 旧版本（21:03 构建） | 当前源码 |
| ----------------------------------------------------------------------- | ---- | -------------------- | -------- |
| 302 → codeload，owner/repo 与输入完全一致                               | 18   | 接受                 | 接受     |
| 302 → codeload，仅 ASCII 大小写规范化                                   | 4    | 拒绝                 | 接受     |
| 301 → `api.github.com/repositories/{id}/tarball/{ref}` → 302 → codeload | 2    | 拒绝                 | 接受     |

大小写规范化的 4 个：`kkkkhazix/khazix-skills` → `KKKKhazix/…`、`leonxlnx/taste-skill` →
`Leonxlnx/…`、`tw93/kami` → `tw93/Kami`、`xinian-dada/fuck_my_shit_mountain` →
`XiNian-dada/Fuck_My_Shit_Mountain`。

numeric 301 的 2 个：`emilkowalski/skill` → id 1183325896 → `emilkowalski/skills`；
`zephyrwang6/space-gpt-image2-design` → id 1222169973 → `SpaceZephyr/space-GPT-image2-design`。

未观察到 query、fragment、userinfo、非 443 端口、非 HTTPS、非 GitHub host 或 ref 变化。

## 当前源码对这些形状的覆盖

- `archive.rs:419-433` 的 `CodeloadIdentity::SameRepository` 使用 `eq_ignore_ascii_case`，覆盖
  4 个大小写规范化仓库。
- `archive.rs:209-241` 的 `MOVED_PERMANENTLY` 分支要求
  `provenance == TrustedDirect`，随后校验 numeric API Location 并要求第二跳为 302，
  `CodeloadIdentity::Canonicalized` 对改名后的 owner/repo 只做安全 component 校验，覆盖 2 个
  numeric 301 仓库。改名后标识
  `SpaceZephyr` / `space-GPT-image2-design` 与 `emilkowalski` / `skills` 均通过
  `validate_repo_owner` / `validate_repo_name`。
- 既有测试
  `archive_redirect_validator_accepts_only_exact_codeload_location`（含 `OpenAI/SKILLS`）与
  `archive_redirect_follows_trusted_numeric_canonicalization_with_scoped_bearer`（含
  `CanonicalOwner/renamed-repo`）分别对应这两类真实形状。

## 已排除

- 不是 PAT 问题：`/rate_limit` 早前已验证 200/5000。
- 不是仓库形状新变化：全部 24 个 branch 为 `main` 或 `master`，无 tag、无带斜杠分支、无
  pinned SHA。
- 不是资源预算：`archive_bytes` 128 MiB、`archive_expanded_bytes` 256 MiB、
  `archive_files` 20000，远高于这些技能仓库体积。
- 不是 mirror 回退：直连 `api.github.com` 24 次请求全部成功返回跳转，未触发回退条件。

## 待执行动作

1. 用当前工作区源码重新构建 release 并覆盖安装
   `C:\Users\lyh\AppData\Local\SkillPort\skillport.exe`。
2. 重启应用后重跑“检查全部”，确认 24 个仓库全部完成、inventory 落盘、`unsupported` 分类
   出现。
3. 该实机结果是 AC23 中“修复后真实 24-repository 探针不再发现 policy mismatch”的剩余证据。
