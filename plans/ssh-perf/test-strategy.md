# 测试策略

## 测试金字塔

```text
            ┌──────────────┐
            │  手工 / 真实 │  E2E（dckj 远端）
            │   远端验收    │  少量、慢、人介入
            └──────────────┘
        ┌──────────────────┐
        │   集成测试         │  docker openssh 容器
        │ Rust + Frontend   │  中等数量、中速
        └──────────────────┘
   ┌──────────────────────────┐
   │       单元测试             │  mock SSH connection
   │ Rust + TS 各自独立         │  大量、快
   └──────────────────────────┘
```

## 阶段对应测试

### 阶段 1（稳定性兜底）

**单元测试（Rust）**

- `src-tauri/src/services/scanner/tests.rs`：
  - `test_scan_all_skills_timeout`：mock SSH 永不返回，验证 scan_all_skills ≤ 95s 返回 Err
  - `test_stale_scan_state_recovery`：DB 设 `scan_state="refreshing"` + `scan_last_completed_at` 11 分钟前，调 hydrate，验证状态变 idle
- `src-tauri/src/targets/tests.rs`：
  - `test_askpass_helper_drop_cleans_file`：构造 helper，drop，验证文件不存在
  - `test_askpass_sweep_old_files`：构造 mtime 老于 1h 的文件，调 connect，验证清理

**单元测试（TS）**

- `src/test/targetStore.test.ts`：
  - `switchTarget triggers platformStore.initialize`：mock platformStore，断言 initialize 被调
- `src/test/platformStore.test.ts`：
  - `resetForTargetChange clears state`：调用后 agents/skillsByAgent 清空、scanGeneration +1

**集成测试**

- 跑 `cargo test` 通过
- 手工：黑洞 host 测 90s 超时

### 阶段 2（批量化）

**单元测试**

- `src-tauri/src/services/scanner/ssh_batch.rs`（新文件）配套：
  - `test_build_probe_script`：给 root 列表，输出脚本字符串符合预期
  - `test_parse_probe_output`：给 mock stdout，解析出正确 (root, skill_md) 列表
  - `test_parse_batch_read_output`：给含特殊字符的内容，分隔正确
  - `test_path_with_spaces`：root 含空格，shell 引用正确
  - `test_path_with_tab`：用 \0 分隔，不被 tab 干扰
- `src-tauri/src/services/scanner/tests.rs`：
  - `test_scan_ssh_skills_impl_call_count`：mock connection 计数，验证 ≤ 3 次 run_command

**集成测试（docker openssh）**

- `tests/integration/ssh_scan.rs`（新）：
  - 启动 openssh-server 容器，预置 ~/.claude/skills/{foo,bar,baz}/SKILL.md
  - 跑完整 scan_ssh_skills_impl，验证 DB 中 3 条记录
  - 测时长 ≤ 10s

### 阶段 3（russh）

**单元测试**

- `src-tauri/src/targets/tests.rs`：
  - `test_client_pool_reuse`：mock russh handle，连续 get_or_connect 同 target 2 次，验证返回同一 Arc
  - `test_client_pool_invalidate`：invalidate 后 get_or_connect 返回新连接
  - `test_known_hosts_first_time_accept`：known_hosts 为空，连接成功并写入
  - `test_known_hosts_mismatch`：known_hosts 有不同 fingerprint，返回 KnownHostsMismatch error
  - `test_run_command_cancellation`：cancel token 触发后 run_command 立即返回 Err("cancelled")

**集成测试（docker）**

- `tests/integration/russh_basic.rs`：
  - 容器内跑 100 次 run_command "echo hi"，验证 ≤ 5s（说明握手只 1 次）
  - 100 次连续 connect → list_dir → run_script 流程，无 fd 泄漏（lsof 检查）
- `tests/integration/russh_keys.rs`：
  - 测试 OpenSSH 新格式 / PKCS#8 / RSA 三种私钥格式连接成功
  - 测试 passphrase 加密 key 从 cred store 读 passphrase 成功
- `tests/integration/russh_compat.rs`：
  - 用不同 sshd 版本镜像（OpenSSH 7.4 / 8.0 / 9.x）跑基础流程

**手工测试**

- 真实 dckj：100 次连续扫描，wireshark 抽检 TCP 握手数 = 1
- 真实 dckj：扫描时主机突然重启，应用应在 90s 内显式 error 而非永久卡

### 阶段 4（UX）

**单元测试**

- `src/test/platformStore.test.ts`：
  - `scanGeneration_out_of_order`：模拟先后两次 scan，先发的后回，验证后发数据胜出
  - `cancelScan_sets_stale`：scanState 从 refreshing 变 stale，缓存数据保留
  - `progress_event_updates_phase`：emit progress event，前端 scanProgress 更新

**集成测试**

- Playwright（如已配）：
  - 启动应用 → 500ms 内截图，验证看到 Dashboard 缓存数据
  - 手动触发 scan → 截图验证状态条文案变化
  - 点取消按钮 → 状态变 stale

**手工测试**

- 启动到首屏可见时间秒表测
- 切 target 后右栏刷新延迟视觉感受

### 阶段 5（边角）

**单元测试**

- `test_bootstrap_snapshot_concurrent`：mock SQL pool，验证 5 个 query 并发发起
- `test_default_enabled_agents`：seed 后默认启用列表 = [claude-code, codex, openclaw, central]

**手工测试**

- 跑 1 万条 operation_logs 后查 recent 50 耗时

## 跨阶段回归

每个阶段合并前必须全绿：

```text
┌──────────────────────────────────────────────────┐
│ pnpm typecheck                                   │
│ pnpm lint                                        │
│ pnpm test  （目前 370 中含 3 个遗留失败需保持）   │
│ cd src-tauri && cargo test                       │
│ cd src-tauri && cargo clippy -- -D warnings      │
│ cd src-tauri && cargo fmt --check                │
└──────────────────────────────────────────────────┘
```

## docker openssh 测试基础设施

新增 `tests/docker/openssh-Dockerfile`：

```dockerfile
FROM linuxserver/openssh-server:latest
ENV PUID=1000 PGID=1000 USER_NAME=test PASSWORD_ACCESS=true USER_PASSWORD=testpass
COPY fixtures/skills /home/test/.claude/skills
```

新增 `tests/integration/common.rs`：起容器、生成临时 key、解析端口、关容器的辅助函数。

按需在 CI 中跑 (`docker-compose up -d openssh && cargo test --test '*' -- --test-threads=1`)。

## 性能基准（自动化）

新增 `src-tauri/benches/ssh_scan.rs`（criterion）：

- `bench_scan_30_agents_5_skills`：每个阶段对应一组数据
- 输出 JSON 报告，记录到 `progress.md`

阶段 1：基线
阶段 2：单次 scan SSH 调用次数（应 ≤ 3）
阶段 3：100 次 run_command 总时长（应 ≤ 5s 含 1 次握手）
阶段 4：UI 首屏时间（前端 markAppPerformance 数据）

## 测试覆盖目标

- 阶段 1-2：Rust line coverage 不下降
- 阶段 3：新代码 line coverage ≥ 80%
- 阶段 4：前端 platformStore 关键 action 100%

## CI 改动

- 加 docker openssh 集成测试 job（可选 cron）
- 加 cargo bench 阶段对比（可选）
