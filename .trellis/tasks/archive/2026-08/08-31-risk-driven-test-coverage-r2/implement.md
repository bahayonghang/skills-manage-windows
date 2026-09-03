# Implementation Plan: 风险导向测试覆盖补强（第二轮）

## 1. Baseline And Guardrails

- [ ] 读取 backend/frontend/quality 相关 spec 与两份 r2 research。
- [ ] 记录初始 `git status`，保留无关改动。
- [ ] 确认聚焦过滤表达式当前已能发现非零测试（新名字落地后再跑一遍）。

## 2. Critical Module A — Central store relocate

- [ ] 在 `services::central_store_location::tests` 增加 agents UPDATE 失败、后续 skills REPLACE 失败、前缀碰撞、去掉 trigger 后重试。
- [ ] 若回归失败：四表单一事务、前缀安全改写、补偿删除新建目标目录。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked services::central_store_location::tests`，确认非零。
- [ ] 再跑名称过滤（如 `central_store_location_`）确认新用例被发现。

## 3. Critical Module B — ensure_centralized

- [ ] 增加 upsert 失败补偿、存在 SKILL.md 但非 central 时的 retry 修复、Local install 入口失败。
- [ ] 若回归失败：去掉“仅看 SKILL.md”的短路，或在存在文件时仍修复 DB；copy 失败补偿。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked ensure_centralized`，确认非零。

## 4. High Module C — SSH/WSL create/update

- [ ] 抽出 probe 之后的 persist helper；用 MemoryCredentialBackend + settings trigger + cache-init 失败覆盖 R3。
- [ ] 若 `commands.rs` 将超过 800 行，把 helper 移到 sibling module。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked targets::tests`，并确认 `create_ssh_target_` / `update_ssh_target_` / `create_wsl_target_` 名称过滤非零。

## 5. High Module D — Target ID parity

- [ ] 表测非法/合法 ID；断言 quarantine/reject 与 `sanitize_target_id` 一致，且无根外目录。
- [ ] 若回归失败：让 `validate_target_ids` 使用与 `sanitize_target_id` 相同的字符类。
- [ ] 运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked targets::config::tests` 与相关 `sanitize_target_id` / `remote_cache_db_path` 过滤，确认非零。

## 6. Backend Gate (after A–D)

- [ ] `cd src-tauri; cargo fmt --all -- --check`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`

## 7. High Module E — Central metadata / review

- [ ] 在 `centralSkillsStore.test.ts` 增加 create/unassign/accept/skip/load/bulkSuggest 空输入、首命令失败、refresh 失败 + `requiresCentralReload`。
- [ ] 若回归失败：store 增加 reload-required；`loadAiTagReviews` 写入 error。
- [ ] 运行：`pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "repository|tag|review|bulkSuggest"`，确认非零。

## 8. High Module F — GitHub PAT

- [ ] 增加 save/clear/test 拒绝与成功路径的 sentinel 递归断言。
- [ ] 运行：`pnpm exec vitest run src/test/stores/settingsStore.test.ts -t "GitHubPat"`。

## 9. High Module G — Central install/delete/toggle refresh failure

- [ ] 表驱动 mutation 成功 + `get_central_skills` 拒绝；断言 reload-required。
- [ ] 运行：`pnpm exec vitest run src/test/stores/centralSkillsStore.test.ts -t "installSkill|batchInstall|deleteCentralSkill|togglePlatformLink"`。

## 10. Medium Module H — Release metadata producers

- [ ] 新增 `src/test/scripts/releaseMetadataGeneration.test.ts`。
- [ ] 若回归失败：`findAsset` 拒绝 0/>1 匹配；`readSignature` 拒绝空内容。
- [ ] 运行：`pnpm exec vitest run src/test/scripts/releaseMetadataGeneration.test.ts src/test/scripts/releasePreflight.test.ts src/test/contracts/releaseWorkflowContract.test.ts`。

## 11. Medium Module I — Update Center apply + inventory

- [ ] apply 成功、inventory 拒绝：`requiresInventoryReload`、loading 清、rethrow。
- [ ] 运行：`pnpm exec vitest run src/test/stores/updateCenterStore.test.ts`。

## 12. Frontend Gate

- [ ] `pnpm typecheck`
- [ ] `pnpm lint`
- [ ] `pnpm test`

## 13. Independent Check And Completion Gate

- [ ] 派发 `trellis-check`：spec、回归质量、测试盲区、无意义覆盖。
- [ ] 按同范围反馈修正并重跑受影响聚焦测试。
- [ ] 运行最终 `just ci`。
- [ ] `git diff --check` 与最终 `git status`。
- [ ] 分别记录通过、失败、跳过、零测试过滤和外部/原生环境 `UNVERIFIED`。

## Risky Files / Rollback Points

- `src-tauri/src/services/central_store_location/mod.rs`：迁根事务、前缀改写、FS 补偿。
- `src-tauri/src/services/installation/centralize.rs`：exists 短路与 upsert 补偿。
- `src-tauri/src/targets/commands.rs`（及可能的 sibling persist module）：create/update 回滚。
- `src-tauri/src/targets/config.rs`：target ID 字符类。
- `src/stores/centralSkillsStore.metadataSlice.ts` / `installSlice.ts` / types：reload-required。
- `src/stores/settingsStore.ts`：PAT 失败路径（预期仅测试）。
- `src/stores/updateCenterStore.ts`：apply 后 inventory 失败。
- `scripts/release/generate-latest-json.mjs`：空签名与重复资产 fail-closed。

任何修复若需要 schema migration、新依赖、尚未定义的公开成功载荷脱敏、并发 flush 机制或跨模块 journal，停止并回到规划。

## Follow-up Before Start

- [ ] `prd.md` / `design.md` / `implement.md` 已对齐 research。
- [ ] `implement.jsonl` / `check.jsonl` 已替换 seed 行。
- [ ] 用户批准本轮规划摘要后才执行 `task.py start`。
