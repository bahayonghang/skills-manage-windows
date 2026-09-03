# Design

## Canonical Flow

`ipc_registry.rs runtime entry + Rust command signature/Serde DTO → __skillport_generated_commands → ipc_codegen.rs → generatedCommandMap.ts → commandMap.ts → invoke.ts → stores`

`ipc_registry.rs` 继续唯一拥有 runtime handler/policy，`generatedCommandMap.ts` 只包含 contract metadata，`invoke.ts` 继续唯一拥有 transport、fixture routing 与 error normalization。[R2][R5][R7]

## Migration Batches

六批以审计时 47 个 allowlist 项为冻结输入；实施首步重测后只允许解释名称增删，不静默改批次。[R1][R3]

1. Collections（6）：`create_collection`、`get_collections`、`get_collection_detail`、`add_skill_to_collection`、`update_collection`、`export_collection`。Rust owner `commands/collections.rs`；frontend owner `src/stores/collectionStore.ts`；test `src/test/stores/collectionStore.test.ts`。
2. Projects（8）：`pick_project_folder`、`add_project`、`list_projects`、`rename_project`、`set_project_pinned`、`rescan_project`、`get_project_skills`、`list_projects_using_skill`。Rust owner `commands/projects.rs`；frontend owner `src/stores/projectsStore.ts` 与 `skillDetailStore.ts`；tests `projectsStore.test.ts`/`skillDetailStore.test.ts`。
3. Settings/runtime/scanner（7）：`get_app_runtime_info`、`record_frontend_runtime_log`、`get_scan_directories`、`add_scan_directory`、`set_scan_directory_active`、`get_settings`、`set_settings`。Rust owners `commands/app_runtime.rs`、`commands/logs.rs`、`commands/settings.rs`；frontend owners `appUpdateStore.ts`、`runtimeLogger.ts`、`settingsStore*.ts`；对应 runtime/store tests。
4. Marketplace/skills.sh/agents（8）：`get_agents`、`list_registries`、`add_registry`、`search_marketplace_skills`、`search_skills_sh`、`resolve_skills_sh_url`、`browse_skills_sh_directory`、`read_skills_sh_file`。Rust owners `commands/agents/mod.rs`、`commands/marketplace.rs`；frontend owners `centralSkillsStore.listSlice.ts`、`marketplaceStore.*Slice.ts`；tests `centralSkillsStore.test.ts`/`marketplaceStore.test.ts`。
5. Central repositories/tags/reviews（12）：`preview_delete_central_skills`、`preview_delete_skill_repository`、`get_skill_repositories`、`create_or_update_skill_repository`、`assign_skills_to_repository`、`set_skill_repository_pinned`、`get_skill_tags`、`create_skill_tag`、`assign_skill_tags`、`get_pending_ai_tag_reviews`、`accept_ai_tag_review`、`skip_ai_tag_review`。Rust owners `commands/skills.rs`、`commands/central_metadata.rs`；frontend owners `centralSkillsStore.{list,install,metadata,update}Slice.ts`；test `centralSkillsStore.test.ts`。
6. AI explanation/jobs（6）：`bulk_suggest_skill_tags`、`cancel_ai_tag_job`、`get_skill_explanation`、`explain_skill`、`explain_skill_stream`、`refresh_skill_explanation`。Rust owners `commands/central_metadata.rs`、`commands/marketplace.rs`；frontend owners `centralSkillsStore.updateSlice.ts`、`skillDetailStore.ts`、`marketplaceStore.*Slice.ts`；tests `centralSkillsStore.test.ts`/`skillDetailStore.test.ts`/`marketplaceStore.test.ts`。

## Change List

- `src-tauri/src/ipc_registry.rs::__skillport_generated_commands`：逐批加入上述 command path；`__skillport_runtime_commands` 不复制、不改 policy。[R2][R3][R5]
- 上述 Rust command owner 及其既有 request/result DTO owner：仅补 codegen 所需 `specta::Type`/Serde metadata，参数名继续由 `ipc_codegen.rs::lower_camel` 规则生成。[R2]
- `src/lib/ipc/generatedCommandMap.ts`：每批仅由 `pnpm ipc:codegen` 刷新。[R2][R6]
- `src/lib/ipc/commandMap.ts::{UNTYPED_IPC_COMMANDS,IPC_COMMANDS}`：每批删除对应 allowlist 项；最后删除 allowlist export。现有 `HANDWRITTEN_IPC_COMMANDS` 不做顺带迁移。[R3][R4]
- `src/lib/ipc/invoke.ts::{invoke,invokeRaw}`：仅在最终批归零后删除任意 string overload，并把 raw self-logging 通道约束为 `keyof IpcCommandMap`。[R4][R7]
- 各批列出的 store 调用点：删除 `invoke<T>` 显式返回泛型，由 command key 推导 args/result，不改 store 状态机。[R3]
- `src/test/contracts/ipcCommandCoverage.test.ts`、`src/test/runtime/ipc.test.ts` 与各批 store tests：维护 parity、负向 type contract、fixture routing 与领域行为。[R3-R6]
- `docs/architecture/_generated/ipc-commands.md`：只由 `pnpm docs:gen` 刷新/检查。[R6]

## Contract

1. Batch graduation：一个 command 只有在 Rust DTO 可 codegen、生成物更新、调用点无显式泛型、allowlist 删除且定向测试全绿后才算毕业。[R2][R3]
2. Transitional compatibility：前五批实施期间保留现有 untyped overload 仅服务尚未毕业的 allowlist；已毕业命令不能同时存在于 allowlist/handwritten/generated 任意两处。[R3][R5]
3. Final closure：最后一批归零后在同一提交删除 allowlist 与 string overload；type fixtures 固化错误命令/参数/返回类型失败方向。[R4]
4. Isolation：runtime registry policy、backend-only set、generated/handwritten disjoint 和 target-specific registry 规则沿用现有 coverage tests。[R5]
5. Generation：Rust/Serde 是新增 47 个 contract 的唯一事实来源，checked artifact 不调用 Tauri，也不含 args/result `unknown`。[R2][R6]

## Compatibility

- command 名、wire 参数名、Serde rename、返回 JSON、error normalization 和运行时 handler 不变；改变仅限编译期类型覆盖。
- 每批未迁移命令继续使用现有 fallback，允许批次独立提交/revert；已迁移命令不增加 alias 或 legacy signature。
- `record_frontend_runtime_log` 保留递归保护的 raw transport，但 raw function 在最终态同样以 command key 推导类型。

## Verification Boundary

- 自动验证：baseline 分区、codegen byte drift、typed map parity/disjoint、负向 type fixtures、fixture/runtime adapter、各域 store 行为、target isolation、docs/check 稳定性。[AC1-AC28]
- 人工检查：每批 generated diff 与 Rust signature 对应，调用点没有 `as unknown`/`any` 绕过。
- 外部证据：Windows WebView2 实际 IPC、真实 SSH/WSL target 和 provider/network command 仍为 `UNVERIFIED`；Rust/TS fixture 不替代这些验收。

## Rollback

- 每个领域批次是独立 rollback unit：成组 revert 该批 Rust derives/registry entry、生成物、allowlist 删除、调用点和测试，不触碰已完成其他批。
- 最终 fallback-removal 是第七个独立 rollback unit；若 typecheck 发现漏项，只回退该 final commit，已毕业批次保持 typed。
- 禁止用恢复 command alias、第二 wrapper 或 `any` assertion 作为 rollback 修复。

## Considered but Not Chosen

- 不一次性迁移 47 个命令：会失去可定位、可提交和可回滚边界。
- 不新建 TypeScript schema/validator：Rust/Serde codegen 已是现有权威路径。
- 不顺带迁移现有 handwritten typed commands：它们已经受 typed/parity 合同保护，不属于剩余 allowlist 问题。
