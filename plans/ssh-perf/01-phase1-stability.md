# 阶段 1 — 稳定性兜底

## 目标

让用户即使在慢/坏 SSH 下也不会卡死、能恢复、不"一直转"。**这是不动 SSH 引擎前提下的最大可用性提升。**

## 任务清单

### 1.1 SSH Command 加超时参数

文件：`src-tauri/src/targets/exec.rs`，`base_command()` 函数

要加的参数：

```text
-o ConnectTimeout=10
-o ServerAliveInterval=15
-o ServerAliveCountMax=3
-o BatchMode=yes  # （仅 key 模式，password 模式已经用 BatchMode=no）
```

注：`StrictHostKeyChecking=accept-new` 已有，不动。

```text
┌──────────────┬──────────────────────────────────────────┐
│ 入口         │ src-tauri/src/targets/exec.rs            │
│ 函数         │ ConnectedSshTarget::base_command         │
│ 实现         │ 在 -p port 之后插入 -o 三连              │
│ 测试点       │ 给一个 192.0.2.1 (TEST-NET-1，黑洞地址)  │
│              │ 验证 10s 内返回 timeout 而不是无限等     │
└──────────────┴──────────────────────────────────────────┘
```

### 1.2 scan_all_skills 全局 timeout

文件：`src-tauri/src/commands/scanner.rs`

包一层 `tokio::time::timeout`：

```text
let scan_result = match active_target {
    ActiveTarget::Local => scan_all_skills_impl(&pool).await,
    ActiveTarget::Ssh(target) => {
        tokio::time::timeout(
            Duration::from_secs(90),
            scan_ssh_skills_impl(&pool, &target),
        )
        .await
        .map_err(|_| "Scan timed out after 90s".to_string())
        .and_then(|r| r)
    }
};
```

超时分支要：
- 写 `scan_state="error"`
- `record_operation_log_best_effort` 记录失败原因 = "timeout"
- 返回 Err 让前端显示具体错误

90s 阈值依据：现状无超时随便就 5+ 分钟；阶段 2 完成后扫描 < 10s；保守取 90s 作为完全超时上限，阶段 4 加 cancel 后用户可手动结束。

### 1.3 启动自愈 scan_state

文件：`src-tauri/src/commands/bootstrap.rs` 的 `get_bootstrap_snapshot_impl`

启动时检查：

```text
读 scan_state、scan_last_completed_at（rfc3339）
若 scan_state == "refreshing" 且 distance(now, scan_last_completed_at) > 10min
  → 重置 scan_state = "idle"
  → 记 operation log: scan_state recovered from stale refreshing
```

阈值 10min：阶段 2 完成后扫描 < 10s，正常 refreshing 不可能跨 10min。

### 1.4 切 target 后联动 rescan（前端）

文件：`src/stores/targetStore.ts` `switchTarget` 方法

```text
async switchTarget(targetId) {
  const activeTarget = await invoke("set_active_target", { targetId });
  set({ targets: markActive(...), activeTarget });
  await get().loadTargets();

  // 新增：触发 platform 数据全量刷新
  const platform = usePlatformStore.getState();
  platform.resetForTargetChange();   // 新增 action
  await platform.initialize();        // 重新 hydrate + 后台扫
}
```

`resetForTargetChange` 要做的事：

```text
- agents: []
- skillsByAgent: {}
- lastScanAt: null
- scanState: "idle"
- isLoading: true   ← 让 UI 进入 loading 而非仍显示旧 target 数据
- scanGeneration += 1
```

注意：阶段 4 会改成"先显示旧数据，扫完再 swap"，本阶段先做朴素清空。

### 1.5 askpass 临时文件清理

文件：`src-tauri/src/targets/askpass.rs`

现状：每次 `connect_ssh_target` 调 `create_askpass_helper` 写临时脚本，依赖进程退出时 OS 清理。Windows 下经常残留。

要做：

- `AskpassHelper` 实现 `Drop` trait，主动 `fs::remove_file`
- `connect_ssh_target` 每次进入时先 sweep `<tmpdir>/skillport-askpass-*` 中超过 1 小时的旧文件
- 给 helper 文件路径加 PID 前缀，避免多实例互删

```text
┌──────────────┬────────────────────────────────────────┐
│ 入口         │ targets/askpass.rs                     │
│ 新增         │ impl Drop for AskpassHelper            │
│ 调用点       │ create_askpass_helper 进入时调 sweep   │
│ 残留判定     │ mtime 超 1 小时 + 文件名前缀匹配        │
└──────────────┴────────────────────────────────────────┘
```

## 测试

| 用例                                       | 验证                                  |
|--------------------------------------------|---------------------------------------|
| 黑洞 host 192.0.2.1                        | scan ≤ 90s 失败而非永久卡             |
| 网络断开                                    | 同上                                  |
| 模拟 panic 写 `scan_state="refreshing"`     | 重启 ≤ 1s 自动重置为 idle             |
| 切 target Local→SSH→Local                   | 每次切完 200ms 内右栏新数据           |
| 连续 connect 100 次                         | tmpdir 下无 askpass-* 残留            |
| 现有 cargo test                            | 不破坏                                |

## 文件改动清单

```text
src-tauri/src/targets/exec.rs           +3 行 -o 参数
src-tauri/src/targets/askpass.rs        ~50 行 Drop + sweep
src-tauri/src/commands/scanner.rs       ~10 行 timeout 包装
src-tauri/src/commands/bootstrap.rs     ~15 行 自愈逻辑
src/stores/platformStore.ts             ~10 行 resetForTargetChange action
src/stores/targetStore.ts               ~5 行 switchTarget 联动
src/test/targetStore.test.ts            +1 用例
src-tauri/src/services/scanner/tests.rs +2 用例（timeout、stale state）
```

## 风险

| 风险                                | 缓解                                  |
|-------------------------------------|---------------------------------------|
| ConnectTimeout=10 对慢网络可能太短  | 设可配置（settings 表新增 ssh.connect_timeout 默认 10） |
| BatchMode=yes 阻断交互式认证        | 仅 key 模式加；password 模式不加      |
| 切 target 立刻清空数据用户体验差    | 阶段 4 改成 stale-while-revalidate    |

## 验收

- 黑洞测试：实测 scan 在 ≤90s 内显式失败
- 切 target 测试：切完 200ms 内右栏数字更新
- 残留测试：连续 100 次 connect 后 tmpdir 干净
- 回归：`cargo test`、`pnpm test`、`pnpm typecheck`、`cargo clippy -- -D warnings`

## 估时

1-2 天工作日。
