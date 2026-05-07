# 阶段 4 — UX 与首屏

## 目标

让"扫描"对用户从黑盒变白盒：缓存先显示、进度阶段可见、可取消、切 target 平滑过渡。

## 任务清单

### 4.1 缓存先显示（stale-while-revalidate）

文件：`src/stores/platformStore.ts` `hydrateShell` 与 `refreshScanInBackground`

现状：`hydrateShell` 已经走 `loadBootstrapState` 读 DB 缓存，但前端某些视图（如 `Enabled platforms`）依赖 `is_detected` 字段，DB 字段没刷新前显示空。

要做：

```text
启动序列：
1. hydrateShell()
   ├─ loadBootstrapState() 读 DB 缓存 ← 已有
   └─ set { agents, skillsByAgent, ..., scanState: "stale" }
                                       ^^^^^^^^^^^^^^^^^^^^^^
                                       新增 stale 标志位
2. UI 立刻渲染所有数据，但状态条显示"上次扫描 3 分钟前 · 刷新中"
3. refreshScanInBackground() 跑完后再 set 新数据
```

新增 `scanState` 取值：
- `idle`（已是最新）
- `stale`（显示缓存，正在后台刷）
- `refreshing`（缓存为空或被废，没法显示数据）
- `error`
- `cancelled`

`StatusTile` 的 SCAN STATE 文案据此区分。

### 4.2 切 target 不闪烁

文件：`src/stores/platformStore.ts` `resetForTargetChange`

阶段 1.4 朴素实现是清空。这里改为：

```text
resetForTargetChange:
  保留 agents/skillsByAgent 当作"上次缓存"
  scanState = "stale"
  scanGeneration += 1（让依赖此值的 UI 触发刷新）
  isLoading = false
  ↓
initialize() 起新 scan
↓
新数据回来后用 scanGeneration 比对，最后到的 scan 写入数据
```

`scanGeneration` 用于丢弃乱序响应。

### 4.3 后端 emit 进度事件

文件：
- `src-tauri/src/services/scanner/mod.rs`
- `src-tauri/src/commands/scanner.rs`

在 scan 关键节点 emit Tauri 事件：

```text
事件名：scan:progress
载荷：
{
  generation: u64,
  phase: "probing" | "discovering" | "reading" | "persisting" | "done" | "error" | "cancelled",
  current: u32,
  total: u32,
  detail: Option<String>,  // 当前正在扫的 root
}
```

阶段 2 批量化后，progress 主要是：
- probing（5%）
- discovering（30% — find 阶段）
- reading（70% — 批读）
- persisting（90% — 写 DB）
- done（100%）

不要在 reading 阶段每条 SKILL.md emit 一次，按 100 条或 1 秒节流。

前端 listener：

```text
src/stores/platformStore.ts
  listen('scan:progress', ({ generation, phase, current, total, detail }) => {
    if (generation !== currentGeneration) return;
    set({ scanProgress: { phase, current, total, detail } });
  });
```

### 4.4 取消按钮

文件：
- 新增 IPC：`src-tauri/src/commands/scanner.rs` `pub async fn cancel_scan()`
- 后端：`AppState` 持有 `Arc<Mutex<Option<CancellationToken>>>`，scan 启动时存 token，cancel_scan 调 `token.cancel()`
- 前端：Sidebar 底部 `Scanning...` 旁加 `<button onClick={cancelScan}>×</button>`

```text
平台 store action:
  cancelScan() {
    invoke('cancel_scan');
    // scan_all_skills 应该返回 Err("cancelled by user")
    // refreshScanInBackground 的 catch 处理：
    //   不要把 cancelled 当 error；scanState 改 "stale"（保留缓存）
  }
```

阶段 3 的 `run_command(cmd, cancel_token)` 接到 token cancel 立即关 channel。

### 4.5 SCAN STATE UI 重新设计

文件：`src/components/dashboard/DashboardShell.tsx` 与 `src/components/layout/TopBar.tsx`

```text
StatusTile（Dashboard 上的"SCAN STATE"卡）：
  idle:        ✓ 上次扫描 3 分钟前
  stale:       ⟳ 显示缓存 · 刷新中（discovering 12/30）
  refreshing:  ⟳ 首次扫描中（discovering 12/30）
  error:       ! 失败 · 点重试
  cancelled:   ⊘ 已取消 · 点重新扫描

TopBar 右上角 Scan indicator：
  仅在 stale/refreshing 时显示
  显示当前 phase 的简短文本：probing / scanning / reading / saving
  鼠标悬停显示详细 detail（当前 root 路径）
  紧挨着加一个 ×（cancel） 按钮，hover 出现
```

### 4.6 错误展示

scan_state="error" 时：
- StatusTile 显示错误简述
- 点开弹小卡片显示完整 error message + "查看日志"按钮（跳到 /logs）
- 不要静默失败；不要 toast 后立刻消失

## 文件改动清单

```text
src-tauri/src/commands/scanner.rs                +cancel_scan IPC, +emit progress
src-tauri/src/services/scanner/mod.rs            +emit hooks（注入 ProgressReporter）
src-tauri/src/services/scanner/progress.rs       +50 行（新）
src-tauri/src/lib.rs                             AppState 加 cancel_token slot
src/stores/platformStore.ts                      +scanProgress 字段，+cancelScan，+listener
src/types/index.ts                               +ScanProgress type
src/components/dashboard/DashboardShell.tsx      StatusTile 重设计
src/components/layout/TopBar.tsx                 Scan indicator + cancel 按钮
src/i18n/locales/zh.ts / en.ts                   +新文案
+测试：cancel 流程、stale 显示、scanGeneration 乱序
```

## 风险

| 风险                                | 缓解                                     |
|-------------------------------------|------------------------------------------|
| emit 频率过高拖前端                  | 节流 100 条或 1 秒一次                   |
| cancel 后状态不一致                  | 单一 path：cancel → backend Err("cancelled") → frontend stale |
| scanGeneration 比对漏掉             | 单测：模拟先后两次 scan 完成顺序颠倒     |
| stale 缓存数据陈旧（DB schema 改）   | hydrateShell 拿 schema_version 比对，不一致清空 |

## 测试

| 用例                                  | 验证                                   |
|---------------------------------------|----------------------------------------|
| 启动后 500ms 内显示缓存数据            | 测 markAppPerformance("shell_ready") 时间 |
| 扫描进行中点取消                       | scan_state 变 stale，UI 保留缓存       |
| 扫描失败                               | StatusTile 显式 error，点开有详情      |
| 切 target 后 200ms 内右栏显示 stale 缓存 | 不闪 0 of 0                            |
| 慢扫描期间 phase 文案推进              | UI 看到 probing→discovering→reading    |

## 估时

1-2 天工作日。
