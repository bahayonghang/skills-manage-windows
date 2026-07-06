# 收拢 Local/SSH/WSL transport seam：一份操作实现三个 adapter

## Goal

定义「对 active target 执行技能操作」的 transport seam：Local / SSH / WSL 作为三个 adapter，消除 `*_impl` / `*_remote_impl` / `*_ssh_impl` 平行函数族与命令层重复的 `match active_target` 分发，使新操作只需一份实现。

## 背景与证据（2026-07-04 架构评审）

- 平行函数族：8 个 `*_ssh_impl` + 5 个 `*_remote_impl`，与基础 `*_impl` 并存（例：`install_skill_to_agent_impl` / `_remote_impl` / `_ssh_impl`；`delete_central_skills_impl` 三连）——每个技能操作写约三遍。
- `match &active_target { Local => … Ssh|Wsl => … }` 分发复制在 **14 个命令文件**（linker、skills、agents、central_store_location、central_updates/*、skill_update_inventory/*、github_import、portable_state、scanner、usage、local_remote_sync）。
- `targets/` 只提供连接原语（命令构造 + 进程执行耦合，无可注入 runner seam），不提供操作级 seam——测试面走查证实 SSH/WSL 执行半边结构性不可测（`targets/tests.rs` 38 条全是纯字符串构造测试）。

## Requirements

1. 设计并落地操作级 transport seam：interface 是技能操作（install/uninstall/delete/scan/read 等，具体清单 design 阶段盘点），adapter 是 Local / SSH / WSL。
2. **先试点后推广**（评审明确建议）：第一阶段只深化一个操作（建议 install），验证 seam 形状后再决定推广范围；试点结论写回本任务再进入第二阶段。
3. SSH 执行路径引入可注入 runner，使远程半边可以脱离真实 SSH 进程做单元测试。
4. `targets/` 连接原语退到 adapter 内部，不再被命令层直接编排。

## Constraints

- **不动 SSH remote target lifecycle**（增删改/连接管理/凭据存储）——CONTEXT.md「不要重复建议」明令，本任务只动操作执行 seam。
- remote install 默认 copy、私钥内容不落盘、password 走系统凭据库等既有语义零变化（CONTEXT.md SSH remote target 约束）。
- **硬依赖**：必须在 `07-04-central-updates-service-domain` 完成后进行（同一批命令文件先归位再收拢，避免二次返工）。
- 域错误枚举与 `#[error]` 文案契约不变。

## Acceptance Criteria

- [ ] 试点操作（install）只有一份业务实现，传输差异全部在 adapter 内（三份 `install_skill_to_agent_*` 函数族收敛，grep 验证）。
- [ ] SSH 路径存在脱离真实进程的单元测试（可注入 runner 的证明）。
- [ ] 命令层 `match active_target` 分发处数较基线（14 文件）下降，目标值由 design 定并在验收时 grep 复核。
- [ ] `cd src-tauri && cargo test` 全过；`cargo clippy -- -D warnings` 通过；远程/本地既有行为无回归（现有 installation 66 条 + local_remote_sync 7 条测试全绿）。

## Notes

- 复杂度：complex，且是本专项风险最高的一项（评审强度 Worth exploring 而非 Strong）→ 需 `design.md` + `implement.md`；design 阶段若发现 seam 形状不成立，允许以「试点结论 + 不要重复建议条目」收场。
- 排序：全专项最后一个执行。
