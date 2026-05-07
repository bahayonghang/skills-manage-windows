# 决策记录（ADR）

## ADR-001：SSH 实现选型 — russh

**日期**：2026-05-06
**状态**：已采纳
**决策者**：用户 + Claude

### 上下文

现状用系统 `ssh.exe` 子进程实现，每次 `run_command` 重新握手。测算单次扫描需 100-200 次握手，端到端 60-200s，远超用户体感容忍。

候选方案：
- A. OpenSSH ControlMaster（共享 socket）
- B. 换 Rust SSH 库（russh / ssh2）

### 决策

直接走 B，选 russh。

### 理由

- ControlMaster 在 Windows 原生 OpenSSH 实现不一，命名管道方案与 Unix socket 行为不一致，长期会埋坑。
- ControlMaster 仍要 spawn `ssh.exe`，每次启动开销 50-100ms 在 Windows 上仍可见。
- ControlMaster 的连接生命周期靠子进程 PID 管理，cancel 要 `kill`，难以做精细控制。
- russh 是纯 Rust 实现，跨平台一致，类型化错误，单测可注入 mock transport。
- russh 持有 client handle 后开 channel 是 O(ms) 量级，远端只握手一次。
- 长期看应用需要 SFTP、reverse tunnel、port forward 等扩展能力，russh 都原生支持。

### 不选 ssh2 的原因

- ssh2 是 libssh2 的 Rust 绑定，需 C 依赖，跨平台打包复杂（尤其 Windows 静态链接 OpenSSL）。
- ssh2 的 async 适配靠 tokio 包装，体验差。
- russh 是 async-first，与现有 tokio 栈天然契合。

### 后果

- 阶段 3 工作量大（3-5 天）
- 删除 askpass.rs 与 ssh.exe 子进程路径，减少跨平台分支
- 引入 known_hosts 自管理逻辑（与系统 ssh 隔离）
- 私钥兼容性需要在主流格式（OpenSSH 新格式、PKCS#8、RSA）覆盖

---

## ADR-002：扫描批量化协议 — bash 脚本 + 自定义分隔符

**日期**：2026-05-06
**状态**：已采纳

### 决策

阶段 2 用 bash 脚本一次性输出全部探测/内容数据，在 stdout 中用 ASCII 控制字符（`\x01..\x05` 范围）做分隔。

### 备选与拒绝

- `tar -c -- $files | base64`：编码/解码额外 30% 体积，且 SKILL.md 有时含二进制图片附件不算典型场景，但 tar 会一刀切打包整个目录树
- `git archive`：依赖 git，远端不一定装
- jsonl：每行 JSON，但 SKILL.md 内容含引号/反斜杠需转义，CPU 比简单分隔贵
- protobuf over stdout：依赖远端工具

### 后果

- 远端必须有 bash（POSIX 子集）
- 阶段 3 落地后这个协议仍跑在 russh channel 内，不变
- 后续可平滑迁移到 SFTP（阶段 5.4）

---

## ADR-003：cancel 机制 — CancellationToken 全链路

**日期**：2026-05-06
**状态**：已采纳

### 决策

`tokio_util::sync::CancellationToken` 从 IPC 入口一路传到 `ConnectedSshTarget::run_command(cmd, token)`，token cancel 立即关 channel。

### 不选信号/进程组的原因

阶段 3 之后没有子进程，只有 russh channel。CancellationToken 与 channel 生命周期天然对齐。

---

## ADR-004：known_hosts 与系统 ssh 隔离

**日期**：2026-05-06
**状态**：已采纳

### 决策

应用自管理 `known_hosts`，路径 `<app_data>/known_hosts`，与 `~/.ssh/known_hosts` 完全隔离。

### 理由

- 用户应用内增删 target 不应该污染系统 ssh
- 应用沙盒（macOS）通常不让访问 `~/.ssh`
- 可控的"信任并替换"UI 行为
- 首次连接策略 `accept-new`，写入后严格匹配

### 后果

- 用户在系统 ssh 已信任的主机，应用内仍要首次确认
- 弹"是否信任"前要把 fingerprint 显示给用户

---

## ADR-005：阶段 1 朴素清空 vs 阶段 4 stale-while-revalidate

**日期**：2026-05-06
**状态**：已采纳

### 决策

阶段 1.4（切 target 联动 rescan）先用朴素清空，阶段 4.2 再升级成 stale-while-revalidate。

### 理由

阶段 1 不引入 `scanGeneration` 比对逻辑，避免多状态字段在阶段 4 反复改。两阶段分开发布更安全。

---

## ADR-006：不保留 ssh.exe 路径作 fallback

**日期**：2026-05-06
**状态**：已采纳

### 决策

阶段 3 完成后，`ssh.exe` 子进程路径整体下线。**不保留** "russh 失败时退回 ssh.exe" 的兜底。

### 理由

- 双实现增加 N 倍测试与维护成本
- "退回"语义不明：是认证失败退？连接失败退？算法不兼容退？
- russh 覆盖主流 OpenSSH 版本（5.x+ 都行），真不行就报清晰错误让用户上报
- 双实现存在时，bug 定位耗时几何级上升

### 后果

- 必须确保 russh 测试覆盖足够（含老 OpenSSH server 版本兼容性）
- 出问题时只需 debug 一条路径

### 例外

测试期（阶段 3 开发分支内）可以保留 ssh.exe 路径以做对照基准。**合并主线前必须删除**。

---

## ADR-007：默认启用的 agent 收窄至 5 个

**日期**：2026-05-06
**状态**：拟采纳，等用户最终确认

### 决策

默认启用：claude-code、codex、openclaw、central（4 个）+ 用户可加。其他 23 个默认 disabled，用户在设置页可启用。

### 理由

- 减少远端探测工作量（哪怕阶段 2/3 完成，仍是节省）
- 大部分用户只用 1-2 个 agent
- 设置页已有 PlatformVisibility 配置面板

### 风险

- 升级用户希望保留全部启用 → 不动现有用户配置，仅影响新装
