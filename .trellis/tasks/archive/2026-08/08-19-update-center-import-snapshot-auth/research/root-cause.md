# Update Center 新增导入失败：本机与代码证据

## 结论

现场失败发生在 Update Center 的 `Apply selected` 新增导入阶段，不是最初的 repository refresh。Refresh 已经成功获取并检查 repository snapshot，Apply 却为每个新增 repository 再次走通用 GitHub import acquisition；当时新的访问被 GitHub 拒绝，所有 addition 导入为 0。与此同时，鉴权分类在 domain error 边界丢失 `used_auth`，所以 UI 只能显示泛化的 token 建议。

永久修复必须同时解决：

1. Refresh 和 Apply 使用同一个不可变 commit/snapshot authority；
2. cache 失效后按固定 commit 恢复并校验，而不是按 branch重取；
3. 保留“请求是否实际使用 token”的 typed事实并输出准确 stable code。

## 本机 Operation Log 时间线

本机应用日志显示（Asia/Shanghai，2026-08-19）：

| 时间 | Operation | 结果 |
| --- | --- | --- |
| 14:55:10 | Update Center refresh | 主流程成功并产生新增项；另有 2 个 repository archive transport failure |
| 14:57:18 | Retry repositories | `bahayonghang` transport failure；`yaojingang` access denied |
| 14:58:07 | Apply selected | updates 3/3；import additions 4/0；4 个 `access_denied` |
| 14:58:18 | Apply selected | import additions 5/0；5 个 `access_denied` |

截图右下角的 `github:kkkkhazix-khazix-skills-main: GitHub denied access...` 与最后一次 Apply 的 repository item failure一致。数据库中该 repository配置和 `leader` pending addition仍存在，说明新增发现已成功，失败发生在导入 acquisition；成功后才删除 pending row 的现有语义也保住了重试入口。

现场随后在 Integrations & Keys 中显示：GitHub token 已配置、当前 session可用，并通过 `Test token`。这证明后续可发认证请求；截图没有证明 14:58 的失败请求是否使用 token，因此任务不能把历史失败武断归类为 token权限错误。

## Repository 事实

- `kkkkhazix/khazix-skills` 是可公开读取的 repository，默认 branch 为 `main`，清单中的 `leader` 路径存在。
- 本机 repository row 的 owner/repo/branch 与 pending addition关联正确，没有证据表明 repository id错配或 pending row孤立。
- 故障时匿名 GitHub core API额度已经耗尽；这能解释新增 acquisition容易失败，但不是持久修复，因为配置 token、缓存驱逐、私有 repository和权限拒绝仍需要准确区分。

## 代码调用图

### Refresh 已有 snapshot cache

- `src-tauri/src/services/central_updates/snapshots/mod.rs` 明确把 cache定义为 check/update共享的短期 snapshot，并按 repository去重 acquisition。
- Cache TTL 为 10 分钟，默认最多 8 entries/256 MiB；oversized snapshot只供当前请求使用，应用重启后全部丢失。
- `src-tauri/src/services/central_updates/inventory/mod.rs` 的 refresh 使用 `snapshots` 发现 remote additions并写 `skill_repository_pending_additions`。

### Apply additions 绕开 cache

- `src-tauri/src/commands/skill_update_inventory.rs` 已把 authenticated client、token和 `snapshots_cache` 传入 apply service。
- `src-tauri/src/services/central_updates/inventory/mod.rs` 的 `import_additions` 循环只加载 repository URL，然后调用：
  - Local：`import_github_repo_skills_with_auth`
  - SSH/WSL：`import_github_repo_skills_remote_with_auth`
- 这两条通用 import路径会重新 resolve/acquire repository；该分支没有读取传入的 `snapshots_cache`。
- 同一函数后续的 updates分支会把 `snapshots_cache` 传给 `update_central_skills_impl`，证明缺口局限在 additions而非整个 Apply。

### 通用 import确实重新获取

- `src-tauri/src/services/github_import/import.rs` 在没有 snapshot authority 的路径重新 resolve repo，并走 tree或archive acquisition。
- `src-tauri/src/services/github_import/remote.rs` 的 non-preview helper创建一个新 remote workspace；workspace-only importer本身不会重新获取，但 Apply当前没有 refresh-time immutable identity可传。
- Immutable preview模块已经具备 commit resolution、pinned ref、repository digest和 snapshot-only local/remote import，可复用其契约而不发明平行 hash语义。

### `used_auth` 在错误转换时丢失

- `src-tauri/src/services/github_import/types.rs` 的 `GitHubAccessDenial` 包含 `used_auth: bool`，Display 文案能区分已认证与匿名请求。
- `src-tauri/src/services/github_import/error.rs::from_denial` 把所有 authentication/permission denial压成 `AccessDenied(String)`；随后 `ipc_error_code()` 固定返回 `github_import.access_denied`。
- `src-tauri/src/ipc_error.rs` 的 legacy string mapper能从英文句子识别 `configured_token_failed`，但 Update Center `SkillUpdateApplyFailure::from_github_import` 直接使用 typed `ipc_error_code()`，绕过该兼容规则。
- `src-tauri/src/services/central_updates/inventory/types.rs` 还把 `error.to_string()` 放入 item failure public error，导致动态 detail与稳定 code可能不一致。

## 已排除的单点原因

- 不是 repository不存在或 branch错误：public repository/main/path均可验证。
- 不是 Refresh整体失败：日志和 pending row证明新增清单已生成。
- 不是只需“再配置一次 token”：当前 token已测试成功，但代码仍会二次 acquisition，且 cache miss/branch移动/私有权限语义没有修复。
- 不是只需把 cache TTL调大：进程重启、LRU、容量和oversized路径仍会失去 bytes，且 branch一致性仍无持久化身份。

## 规划约束

- 不读取、打印或保存真实 token。
- 不直接修改用户 live database或 Central目录。
- 测试必须使用 fake GitHub endpoint、FakeRunner、内存数据库和临时目录。
- 任务保持 planning；本文是实施证据，不是授权开始改产品代码。
