# Skills CLI 库存页实施清单

权威文件以本表为准。每步有最小验证。未完成 `task.py start` 前不要改这些路径以外的产品代码。

## 文件清单

| 路径 | 动作 |
| --- | --- |
| `.trellis/spec/backend/skills-cli-global.md` | list 改 lock+FS+copy；错误矩阵；doctor 非阻塞；snapshot 签名 |
| `.trellis/spec/backend/process-supervision.md` | 范围含本地 node；prepare 必须 `CREATE_NO_WINDOW` |
| `src-tauri/src/services/skills_cli/lock.rs` | 条目元数据；copy 归属 helper（可与 classify 并存） |
| `src-tauri/src/services/skills_cli/mod.rs` | `list_global` 无 runner；snapshot 类型 |
| `src-tauri/src/services/skills_cli/tests.rs` | AC4/AC5/AC6/copy vs junction |
| `src-tauri/src/services/skills_cli/argv.rs` | 仅启动器候选；不改 PIN |
| `src-tauri/src/commands/skills_cli.rs` | `skills_cli_list_global` 返回 snapshot |
| `src-tauri/src/targets/process_tree.rs` | `prepare` 调用 `hide_child_window` |
| `src-tauri/src/targets/runner.rs` | 确认 `run` 只经 prepare spawn；CLI env 若放这里则注明 |
| `src-tauri/src/targets/tests.rs` | AC10：prepare 后 flags；勿只测常量 |
| `src-tauri/src/services/central_updates/inventory/scan.rs` | R11 protect copy |
| `src-tauri/src/services/central_updates/inventory/leftover_cleanup/tests.rs` 或 scan 测试 | AC16 |
| `src/types/index.ts` | snapshot / installKind / sourceTypeBucket |
| `src/stores/skillsCliStore.ts` | 分轨、inventoryError、isRefreshing |
| `src/pages/SkillsCliView.tsx` | 布局、错误、testids |
| `src/components/skillsCli/*` | 仅当拆 KPI/SVG 时新增，禁止第二套卡片 |
| `src/fixtures/skillsCli.ts` | snapshot fixture |
| `src/i18n/locales/en.json`、`zh.json` | 新文案 |
| `src/test/pages/SkillsCliView.test.tsx` | AC1、AC2、AC7、AC8、AC9、AC11、AC12 |
| `src/test/stores/skillsCliStore.test.ts` | 分轨、刷新保留 |
| `src/lib/ipc/generatedCommandMap.ts` | **只通过** `pnpm ipc:codegen` |
| `docs/architecture/_generated/ipc-commands.md` | **只通过** `pnpm docs:gen` |
| `CONTEXT.md` | 若仍写 list spawn CLI 则改为 lock 读取 |

## 顺序与最小验证

1. **契约**  
   改两个 spec。  
   验证：人工读签名与错误矩阵；不跑 CI。

2. **隐藏窗口**  
   改 `process_tree.rs` + `tests.rs`。可选 `runner.rs` env。  
   验证：`cargo test --locked --manifest-path src-tauri/Cargo.toml targets::tests::windows_hidden`（按实际测试名）。

3. **Lock 投影 + copy 归属**  
   `lock.rs` / `mod.rs` / `tests.rs` / `commands/skills_cli.rs`。  
   验证：`cargo test --locked --manifest-path src-tauri/Cargo.toml skills_cli`。

4. **IPC 生成**  
   `pnpm ipc:codegen`  
   `pnpm ipc:codegen:check`  
   `pnpm docs:gen`  
   `pnpm docs:gen:check`  
   验证：`generatedCommandMap.ts` 中 `skills_cli_list_global` Result 为 snapshot；git diff 仅预期生成物。

5. **启动器**  
   `argv.rs` + tests。  
   验证：同 `cargo test` skills_cli 过滤 launcher。

6. **Leftover R11**  
   `scan.rs` + leftover 测试。  
   验证：`cargo test --locked --manifest-path src-tauri/Cargo.toml leftover`（按实际模块名收窄）。

7. **Store**  
   `skillsCliStore.ts` + `skillsCliStore.test.ts`。  
   验证：`pnpm test -- src/test/stores/skillsCliStore.test.ts`。

8. **页面**  
   `SkillsCliView.tsx`、i18n、fixture、Vitest。  
   验证：`pnpm test -- src/test/pages/SkillsCliView.test.tsx`。

9. **错误映射**  
   删除 list→`CliUnavailable`。  
   验证：Rust 单测 + AC13。

10. **门禁**  
    `just ci`。

11. **人工（Windows，不挡 start 后的自动化绿灯，但挡任务完成）**  
    本机打开页 → Refresh：无前台 console。单平台 copy 安装（若可）出现在库存。记录孙进程是否仍闪。

## 验证命令

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml skills_cli
cargo test --locked --manifest-path src-tauri/Cargo.toml targets
pnpm ipc:codegen
pnpm ipc:codegen:check
pnpm docs:gen
pnpm docs:gen:check
pnpm test -- src/test/pages/SkillsCliView.test.tsx src/test/stores/skillsCliStore.test.ts
just ci
```

## 回滚映射

| 步骤 | 回滚动作 |
| --- | --- |
| 2 隐藏窗口 | 可留在 mainline |
| 3–4 snapshot | 恢复 Vec 类型 + 再跑 ipc:codegen/docs:gen |
| 5 启动器 | 独立可回退 |
| 6 leftover R11 | 必须与步骤 3 同回退 |
| 7–8 UI/store | 必须与 snapshot 同回退 |
| 9 错误映射 | 与 3 同回退 |

禁止只上新 UI 仍 `Promise.all` 三条 IPC。

## `task.py start` 前核对

- [x] 规划材料已按 Codex 审阅修订（TPR-01..07）
- [ ] 用户已明确批准**本修订摘要**（阻塞 start）
- [x] `prd.md` 使用 `- R1:` / `- [ ] AC1:` 追踪格式
- [x] `implement.jsonl` / `check.jsonl` 含 copy-mode 调研
