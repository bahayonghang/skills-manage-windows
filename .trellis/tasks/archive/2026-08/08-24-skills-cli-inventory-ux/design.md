# Skills CLI 库存页技术设计

## 1. Architecture

```text
SkillsCliView
  skillsCliStore.loadAll
       │
       ├─ invoke skills_cli_list_global     ──► lock v3 + FS + mapped dirs   (no spawn)
       ├─ invoke skills_cli_install_targets ──► db agents ∩ mapping          (no spawn)
       └─ invoke skills_cli_doctor          ──► node + PIN probe             (spawn, hidden)
                                                    │
Preview / add / remove ──► NodeProcessRunner ──► ProcessRunner
                                                    ProcessTreeGuard::prepare  [Windows CREATE_NO_WINDOW]
                                                    Job Object kill-on-close
```

页面首屏只依赖 list snapshot。doctor 是写路径探针。copy 模式归属算法见 `research/copy-mode-ownership.md`，不得只调用 `classify_local_path_origin == SkillsCli`。

## 2. Boundaries

| 层 | 职责 | 禁止 |
| --- | --- | --- |
| `commands/skills_cli.rs` | Local 门闩、IPC 信封、add/remove job lease 与 operation log | 归属算法、拼 argv、直接 `std::process` |
| `services/skills_cli` | lock 投影、copy/canonical 扫描、doctor/preview/add/remove、启动器 | list spawn；list→`CliUnavailable`；`npx.cmd` |
| `central_updates/inventory/scan` | R11：Local leftover 排除 lock 名下 mapped copy | 用本机 lock 保护远程 leftover |
| `targets/ProcessRunner` | prepare 隐藏窗口 + Job Object | 只给 SSH/WSL 藏窗口；只单测常量 |
| `skillsCliStore` | 唯一 `invoke`；inventory 与 doctor 分轨；`inventoryError` / `runtimeError` | 组件 `invoke`；`Promise.all` 绑死三条 |
| `SkillsCliView` | DOM 顺序、KPI/SVG、折叠安装、两种错误 UI | 前端重算所有权；视口断言当验收 |

## 3. Inventory contract

扩展 lock 解析：version 必须为 3；每个名字保留 `source` / `sourceUrl` / `sourceType`（缺省 null）。成员资格 = 名字集合。

`list_global`（无 runner）：

1. `ensure_local_target`
2. 读 lock；缺文件 → 空 `skills` 数组，仍返回路径字段
3. 对每个名字按 copy-mode 调研计算 `path`、`installKind`、`agents`、`sourceTypeBucket`
4. 不调用 `SkillsCliRunner`

`scope` 恒为 `Some("global")`。

IPC：

```ts
type SkillsCliInstallKind = "canonical" | "copy" | "missing";
type SkillsCliSourceTypeBucket =
  | "github" | "gitlab" | "git" | "mintlify"
  | "huggingface" | "local" | "well-known" | "unknown";

interface SkillsCliGlobalSkill {
  name: string;
  path: string | null;
  installKind: SkillsCliInstallKind;
  scope: string | null;
  agents: string[]; // display names, stable order = install_targets order
  source: string | null;
  sourceUrl: string | null;
  sourceType: string | null; // raw lock
  sourceTypeBucket: SkillsCliSourceTypeBucket;
}

interface SkillsCliGlobalSnapshot {
  skills: SkillsCliGlobalSkill[];
  canonicalRoot: string;
  lockPath: string;
}
```

`skills_cli_list_global`: `Vec<SkillsCliGlobalSkill>` → `SkillsCliGlobalSnapshot`。

生成步骤（权威，禁止手改）：

1. `pnpm ipc:codegen` → `src/lib/ipc/generatedCommandMap.ts`（现为 `command<undefined, SkillsCliGlobalSkill[]>`，`src/lib/ipc/generatedCommandMap.ts:65`）
2. `pnpm ipc:codegen:check`
3. `pnpm docs:gen` → `docs/architecture/_generated/ipc-commands.md` 与 schema 表
4. `pnpm docs:gen:check`

## 4. Store and page errors

```ts
set({
  isLoading: skills.length === 0 && !inventoryError,
  isRefreshing: skills.length > 0,
  inventoryError: null, // 仅在开始 inventory 请求时清本次 inventoryError
});
try {
  const snapshot = await invoke("skills_cli_list_global");
  const targets = await invoke("skills_cli_install_targets");
  set({ skills: snapshot.skills, paths: snapshot, targets, inventoryError: null, isLoading: false, isRefreshing: false });
} catch (e) {
  set({ inventoryError: backendErrorStateValue(e), isLoading: false, isRefreshing: false });
  // 不清 skills
}
try {
  const doctor = await invoke("skills_cli_doctor");
  set({ doctor, runtimeError: null });
} catch (e) {
  set({ doctor: null, runtimeError: backendErrorStateValue(e) });
}
```

页面：

| 状态 | 库存区 | 安装区 | KPI |
| --- | --- | --- | --- |
| 首次 loading | 骨架/忙，不是 empty | 不按空态展开 | 不显示假 0 作为成功普查 |
| `inventoryError` 且 `skills.length===0` | `data-testid="skills-cli-inventory-error"`，无 `skillsCli.empty` | 不按成功空库存展开 | 不显示成功 0 |
| `inventoryError` 且有旧 skills | 旧列表 + 库存错误条 | 保持折叠 | 用旧 snapshot |
| 无错误、`skills.length===0` | empty | 展开 | empty 图表 |
| `runtimeError` | 库存照常 | 安装/卸载 disabled | 照常 |
| 两者同时 | 两条独立区域，文案不得相同并重复占用 `role="alert"` 两次同一句 | | |

`CliUnavailable` 不从 `list_global` 返回。lock IO → 现有 `Io` → `internal.unexpected`。

## 5. Hidden spawn

`ProcessTreeGuard::prepare`（Windows）必须调用 `hide_child_window`。SSH/WSL 已设置则同值幂等。

自动化（AC10）：

1. 单测 `prepare(&mut command)` 之后，用 `Command` 的 Debug 字符串或测试专用记录器断言包含 `CREATE_NO_WINDOW`（`0x08000000`）。这是 **prepare 生产函数**，不是只测 `hidden_child_creation_flags()`。
2. `ProcessRunner` 测试：构造最小 `ProcessRequest`，spy/wrapper 证明 `run` 调用 `prepare`（现有 runner 测试夹具可扩展）。
3. implement 清单中的 **Windows 人工**：doctor 与 add 各一次，确认无前台 console；记录孙进程若仍闪则记 residual（不挡 AC10 自动化）。

CLI spawn 环境：`CI=1`、`npm_config_yes=true`、`npm_config_update_notifier=false`、`npm_config_fund=false`（`NodeProcessRunner` `.env`）。

## 6. Launcher

program = `node.exe`/`node`，argv[1] = `npx-cli.js`。候选：

- `node_dir/node_modules/npm/bin/npx-cli.js`
- `node_dir/lib/node_modules/npm/bin/npx-cli.js`
- Windows：`node_dir/../npm/node_modules/npm/bin/npx-cli.js`、`%ProgramFiles%\nodejs\node_modules\npm\bin\npx-cli.js`、`%APPDATA%\npm\node_modules\npm\bin\npx-cli.js`
- 现有 Unix 全局根

失败 log 打候选列表；`IpcError.message` 用固定公开句。

## 7. Frontend layout

```text
header
runtimeError?     data-testid=skills-cli-runtime-error
inventoryError?   data-testid=skills-cli-inventory-error
KPI + two SVGs
section#inventory data-testid=skills-cli-inventory
details#install   data-testid=skills-cli-install  (open iff empty success)
footer            data-testid=skills-cli-paths
```

图表仿 `ActivityPanel`。平台序 = `install_targets` 序。`sourceTypeBucket` 未知归 `unknown`。

卡片：`UnifiedSkillCard` `variant="skillsCli"`。`path` 走 path prop。copy/missing 用 i18n 短状态，不把根路径塞进 description。

## 8. Leftover (R11)

`scan.rs` `is_cli_protected`：现有 classify **或**（basename 为 lock 名 **且** 规范化 parent 等于某 mapped detected agent 的 `global_skills_dir`）。仅 `cli_lock_protect=true`。测试扩 `leftover_cleanup/tests.rs` 或 scan 单测。

## 9. Compatibility

- add/remove 成功后 `loadAll` 重读 snapshot。
- 非 Local 隐藏入口。
- `src/fixtures/skillsCli.ts` 改为 snapshot。
- 归档 spawn-ls AC 作废；argv/leftover symlink 保护/cancel AC 仍有效，AC14 守住。

## 10. Tradeoffs

| 选择 | 代价 |
| --- | --- |
| 读路径理解 copy 而不改 PIN argv | leftover 必须同步 R11，否则 UI 与清理矛盾 |
| prepare 全局隐藏 | SSH/WSL 已隐藏；本地受监督进程不应要可见 console |
| Debug 断言 creation flags | 依赖 std Debug 格式；用常量 hex 双断言降低脆弱性 |
| 不自动化孙进程窗口 | 人工一步；list 不再 spawn 已去掉 Refresh 主闪烁 |

## 11. Rollback

| 单元 | 回滚 |
| --- | --- |
| `ProcessTreeGuard` 隐藏窗口 | 可单独保留 |
| snapshot IPC | 恢复 `Vec` + 再跑 `ipc:codegen`/`docs:gen` |
| copy 归属 | 会再漏单平台安装 |
| leftover R11 | 与 copy 归属必须同进同退 |
| store 分轨 / inventoryError UI | 与 Promise.all 旧行为同进同退 |
