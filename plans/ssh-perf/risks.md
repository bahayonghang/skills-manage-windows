# 风险登记

> 按"严重度 × 概率"排序。每条至少有一个明确缓解动作。

## R1：russh 与远端 OpenSSH 兼容性（高 × 中）

- **症状**：连接成功但认证失败，或某些算法协商失败
- **触发场景**：远端是老版 OpenSSH（< 7.0）或裁剪版（Dropbear、商用网关）
- **缓解**：
  - 阶段 3 选 algorithm preferences 与 OpenSSH 默认对齐
  - 在 dckj 测试服 + 至少 3 个不同发行版（Ubuntu 18/20/22、CentOS 7/8、Alpine）跑集成测试
  - 错误消息要清晰区分：协议不兼容 vs 认证失败 vs 网络问题
- **应急**：阶段 3 dev 分支保留 ssh.exe 路径作对照（合并主线前删）

## R2：私钥兼容性（高 × 中）

- **症状**：load_secret_key 报错，用户无法登录
- **触发场景**：
  - 用户用 PuTTY .ppk 格式（不被 russh-keys 支持）
  - 用户私钥有 passphrase 但应用没问
  - GPG agent 转 SSH 的私钥（无法直接读文件）
- **缓解**：
  - 阶段 3 落地前列出"支持的私钥格式"清单写到设置页提示
  - 检测到 .ppk 给明确提示"请用 puttygen 转 OpenSSH 格式"
  - 检测到 passphrase 加密的 key，先查 cred store 拿 passphrase；没有就报错让用户在设置页加
- **不在范围**：GPG agent / SSH agent 转发支持

## R3：known_hosts 不匹配静默失败（高 × 低）

- **症状**：远端换机后连接拒绝，用户不知道为什么
- **缓解**：
  - 拒绝时返回结构化错误 `KnownHostsMismatch { host, fingerprint_expected, fingerprint_got }`
  - UI 弹专门的对话框显示新旧 fingerprint，让用户选 trust/cancel
  - trust 后写新 fingerprint，原 fingerprint 备份到 known_hosts.bak

## R4：scan_state 自愈阈值不当（中 × 中）

- **症状**：用户机器慢，正常 scan 跨过 10min 阈值被强重置；或 scan hang 但未达阈值仍卡
- **缓解**：
  - 阶段 1 完成阶段 2 之前阈值定 10min（保守）
  - 阶段 2 完成后扫描 < 10s，阈值缩到 3min
  - 阶段 4 加 cancel 按钮后用户可手动结束，无需依赖阈值

## R5：批量脚本 stdout 体积过大（中 × 中）

- **症状**：30 个 agent × 50 个技能 × 5KB SKILL.md ≈ 7MB stdout，单次 channel 拉取慢
- **缓解**：
  - 阶段 2 设单次脚本输出上限（如 4MB），超过分批
  - 阶段 4 进度事件按 root 粒度 emit
  - 实测 dckj 上 7MB stdout 拉取时间，必要时切 SFTP（阶段 5.4）

## R6：远端 shell 不是 bash（低 × 高）

- **症状**：脚本 `#!/usr/bin/env bash` 在某些远端找不到 bash
- **缓解**：
  - 阶段 2 探测时同时探测 `command -v bash`
  - 没有 bash 时报清晰错误（不试图退回 sh，避免协议不兼容）
  - 文档明确"远端需要 bash 4.x+"

## R7：长连接 NAT 超时（中 × 高）

- **症状**：开着应用等 30 分钟，再操作发现 SSH 已断
- **缓解**：
  - russh 内置 keepalive，配置 60s 一次
  - 检测到 broken pipe 时自动 invalidate pool，下次 run_command 时新建 session
  - 用户层不感知，操作正常进行（多 1 次握手）

## R8：cancel 后状态污染（中 × 中）

- **症状**：用户取消 scan 后状态条仍显示 refreshing
- **缓解**：
  - 单一 path：cancel → backend 立刻关 channel → 返回 Err("cancelled")
  - frontend 把 "cancelled" 单独处理为 stale 状态而非 error
  - 单测覆盖：取消前/中/后 三种时序

## R9：scanGeneration 乱序（低 × 低）

- **症状**：用户快速切两次 target，先发的 scan 后回，覆盖了后发 scan 的结果
- **缓解**：
  - 每次 scan 启动 +1 generation，emit 时带 generation
  - 前端比对：generation < currentGeneration 直接丢弃
  - 单测：模拟两次 scan 完成顺序颠倒

## R10：SQLite 并发查询连接耗尽（低 × 低）

- **症状**：阶段 5.1 把 5 个 SQL 改并发后，pool size 不够阻塞
- **缓解**：
  - 改前 check `db/pool.rs` max_connections，确认 ≥ 8（5 业务 + 3 buffer）
  - 用 `tokio::try_join!` 而非 `tokio::join!`（首个失败立刻短路）

## R11：SSH 多 channel 把远端打挂（中 × 低）

- **症状**：阶段 3 多 channel 并发，远端 sshd 配置 MaxSessions=10 被打满
- **缓解**：
  - SshClientPool 给每个 target 加 Semaphore，同时开 channel 上限 = 4
  - 远端 sshd 配置 MaxSessions 默认 10，留 buffer

## R12：私钥文件权限校验（低 × 中）

- **症状**：Windows 下用户私钥 NTFS 默认权限较松，OpenSSH 拒绝；russh 是否拒绝未知
- **缓解**：
  - 阶段 3 验证 russh-keys 在松权限下能读
  - 文档建议用户手动收紧到 600 等价 ACL（但不强制）

## R13：删除 ssh.exe 路径破坏现有 install/uninstall 流程（高 × 中）

- **症状**：阶段 3 后 linker.rs / installation/remote.rs 调老接口的地方未全部迁移
- **缓解**：
  - 阶段 3 第一步：保持 `ConnectedSshTarget` 公开 API 不变，仅换底层实现
  - 调用方代码 0 修改
  - 全文 grep `ssh_program` / `base_command` / `askpass` 确认零残留
  - 集成测试覆盖 install/uninstall

## R14：用户期望阶段 1 立即解决卡死，但阶段 1 内部仍可能 hang（中 × 中）

- **症状**：阶段 1 加超时但 ssh.exe 子进程在认证阶段卡 30s，超时未触发
- **缓解**：
  - 阶段 1 的 `tokio::time::timeout(90s)` 是 Rust 层强制超时，无视子进程状态
  - 超时后 kill 子进程（阶段 3 后这个问题消失）

## R15：操作日志在切 target 后混淆（低 × 高）

- **症状**：scan 前切 target，日志里 target 标签错位
- **缓解**：
  - 已有 `target_context_from_active_target` 在 scan 入口取 target id
  - scan 进行中切 target 不允许（前端禁用 + 后端再校验）
