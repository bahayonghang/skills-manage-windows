# 优化 SSH/WSL 技能更新可靠性与性能

## Goal

让 Update Center 在 Local、SSH、WSL 三种 active target 下都能可靠完成 Central skill 检查与更新，并把 SSH/WSL 相对原生 Windows 的额外耗时降到可解释、可测量、可回归的范围。

用户价值：远程目标不再因为 shell 参数传递错误而更新失败；批量更新不会因重复连接、重复进程启动或串行远端往返而长时间等待；性能回归能在开发期被基准与测试发现。

## Confirmed Facts

- WSL 更新当前会在原子替换脚本的首个 `mkdir` 失败。`target_dir` 的父目录计算正确，失败发生在 `wsl.exe -> sh -lc -> sh -c` 的嵌套命令边界，脚本位置参数 `$1..$4` 丢失。
- 当前 Central update 已使用 tar.gz 单次上传、远端 staging + atomic swap；SSH hash 已从逐文件读取优化为分块远端 manifest hash。不能把旧的逐文件优化计划原样重做。
- Local、SSH、WSL 共用 `ConnectedRemoteTarget` facade，但 SSH 与 WSL 的进程启动和命令传递成本不同。
- `ConnectedSshTarget` 的每次远端操作都会启动一个新的 `ssh` 客户端进程；`ConnectedWslTarget` 的每次远端操作都会启动一个新的 `wsl.exe` 进程。当前“连接对象复用”不等于持久会话或进程复用。
- 手动刷新必须继续绕过 GitHub snapshot cache；内容 hash 仍是更新判断依据，version 仅是元数据。
- 2026-05 已做过一次 SSH/Local 更新性能优化：批量 assignment、repo snapshot 并发与缓存、分块远端 hash、archive upload、减少前端重复刷新。此次任务必须基于当前代码重新测量剩余瓶颈。

## Requirements

- 修复 WSL Central skill 原子更新脚本的参数传递，保持 archive stdin、路径安全引用、staging/backup 和失败恢复语义。
- 建立同一批固定 fixture 在 Local、WSL、SSH 下的分段性能基线，至少覆盖：连接/进程启动、远端 hash、archive 构建与传输、原子替换、copy install refresh、数据库/前端收尾。
- 记录每个用户动作产生的 GitHub snapshot 请求数、SSH/wsl.exe 子进程数、远端 shell round-trip 数和传输字节量；优化必须针对测得的主导项。
- 批量操作必须在一次 action 内复用 target 上下文；禁止因抽象复用而把远端复合动作拆成更多往返。
- SSH 优化必须兼容现有 key/password 认证、askpass、超时、host key policy 和错误文案；不得把凭据写入日志、数据库或 benchmark 产物。
- WSL 优化必须继续使用指定 distribution 和默认用户 HOME，不依赖 WSL 内开启 sshd。
- 保持 Update Center 的 scope、inventory、manual refresh cache-bypass、force update/mirror、remote-missing 和 copy refresh 语义。
- 性能测量与回归检查必须可由 agent 在 Windows 上重复执行；没有可用 SSH target 时，允许将 SSH 实机数字标为待采集，但必须保留可执行 harness 和调用次数断言。

## Acceptance Criteria

- [ ] WSL 单 skill 与批量 Central update 均可完成，且回归测试能在 `$1..$4` 丢失时失败。
- [ ] 提交一份可重复基线，分别报告 Local、WSL、SSH 的样本规模、冷/热状态、总耗时、阶段耗时、中位数或稳定区间、子进程/round-trip 数。
- [ ] 固定 10-skill warm fixture 下，优化后的 WSL/SSH apply p50 至少比实施前基线快 60%，且单 skill 不回退。
- [ ] 不含 copy refresh 时，远端 apply 进程数从 `1 + N` 降到不超过 `ceil(N / 16)`；copy refresh 从 `C` 降到不超过 `ceil(C / 32)`。
- [ ] 固定 10-skill warm fixture 下，WSL 相对 Local 的附加耗时不超过 1.5 秒；LAN SSH 在继续使用 `ssh.exe` 的阶段不超过 4 秒。未达到时必须回到 planning，不得静默放宽门槛。
- [ ] 批量更新的 GitHub snapshot 获取按 repository 去重，不随同 repo 的 skill 数线性重复。
- [ ] 单次批量 action 不为每个 skill 重建 SSH 认证上下文或重复执行可合并的远端探测。
- [ ] 原子更新在解包或 swap 失败时不破坏原 Central skill，且不会遗留本任务生成的 staging/backup 目录。
- [ ] 相关 Rust 单元/集成测试、前端定向测试、`just ci` 全部通过；涉及 WSL 的实机 harness 在本机 `Ubuntu-24.04` 通过。

## Out of Scope

- 不替换 GitHub API 或改变内容 hash / version 的产品语义。
- 不为 WSL 引入 sshd 依赖。
- 不把本机到远程项目迁移、Discover 或所有远程域的性能重构纳入首轮实现；只修复因同一 transport 原语直接受影响的回归。
- 不在本任务中升级 Trellis。

## Decisions

- 用户于 2026-07-11 批准推荐方案：先完成低/中风险的 WSL `--exec`、lazy open 和批量 apply；`russh` 持久 SSH session 以子任务 `07-11-ssh-persistent-session` 纳入任务树，仅在 Stage 2 后 SSH 未达到性能门槛时启动。
