# 远端路径 canonical 边界（防中间 symlink 逃逸）

## Goal

让 SSH/WSL skill 文件读取与目录入口具备和本地路径守卫等价的 canonical containment 语义：符号链接可以指向 canonical skill root 内部，但任何最终或中间符号链接都不能把访问范围带出该根目录。对应审计 P1-04（M-03）。

## Background

- `src-tauri/src/services/central_skills/files.rs:383-412` 的远端读取先做词法包含检查，再只检查最终对象是否为 symlink；`cat` 仍会解析全部中间 symlink。
- `src-tauri/src/services/central_skills/files.rs:428-461` 的本地守卫会 canonicalize root 与 candidate，再校验 candidate 等于 root 或位于 root 下。
- `src-tauri/src/services/central_skills/files.rs:463-520` 的远端守卫目前只做 POSIX 词法归一、拒绝 `..`/反斜杠及字符串前缀包含。
- 可利用路径为 `root/docs -> /etc`，请求 `root/docs/passwd`；最终对象不是 symlink，但实际读取 `/etc/passwd`。
- `ConnectedRemoteTarget::run_script` 已统一经过带 timeout、输出上限和可注入 runner 的异步进程监督链路。

## Requirements

1. 保留远端词法守卫作为第一层，继续拒绝父目录穿越、反斜杠、root 外绝对路径和相似前缀陷阱。
2. 在执行 inspect/read/list 前，于同一个受监督远端脚本中解析 canonical root 与 canonical candidate，并验证 candidate 等于 root 或位于 `root/` 下。
3. 采用 A 策略：允许最终或中间 symlink，但其 canonical target 必须位于 canonical root 内；root 自身允许是 agent install symlink。
4. canonical root、candidate 或 symlink target 缺失/损坏时 fail closed；远端缺少可用 canonicalization 工具时 fail closed。
5. SSH 与 WSL 使用同一脚本、参数顺序、协议解析和错误映射；路径通过位置参数传递，不拼入 shell 源码，并保留 tab/newline 字符。
6. 已知 canonical 失败映射为 `CentralSkillsError` 语义化变体；不把工具原始 stderr 暴露到用户可见错误。
7. 返回 canonical candidate，并让既有 inspect/read/list 操作使用该路径，避免校验后再使用未经解析的 lexical candidate。

## Acceptance Criteria

- [ ] 词法矩阵通过：`..`、root 外绝对路径、反斜杠、`/skill-a` 与 `/skill-ab` 前缀陷阱均拒绝，root 内相对/绝对路径允许进入 canonical 检查。
- [ ] canonical 矩阵通过：指向 root 内的 final/intermediate symlink 允许，指向 root 外的 final/intermediate symlink 拒绝。
- [ ] root 自身为 install symlink、candidate 等于 root 的合法场景不回归。
- [ ] broken symlink、缺失 root/candidate 和 canonicalization 工具不可用时 fail closed，并得到稳定的 typed domain error。
- [ ] 含 tab/newline 的 root/candidate 作为独立位置参数传输，脚本与返回协议不依赖逐行路径解析。
- [ ] SSH 与 WSL 的 FakeRunner 用例证明脚本 stdin、root/candidate 参数顺序和 standard process policy 一致。
- [ ] 目录树仍不自动递归展示出的 symlink entry，以避免循环；显式请求一个指向 root 内目录的 symlink 时可通过 canonical 入口访问。
- [ ] `cargo test central_skills --locked`、Rust fmt/Clippy/test 以及仓库 `just ci` 全部通过。

## Out of Scope

- 不修改本地路径边界或其既有 canonicalize 行为。
- 不新建通用 `RemoteFs` 抽象，不改 targets 层 transport API。
- 不让目录树自动递归 symlink entry，不处理 symlink 图遍历或循环检测。
- 不改 Central Skills 之外的远端路径调用方。
- 不新增第三方依赖。

## Key Decision

- 已选 A：Local/SSH/WSL 均以 canonical containment 为安全边界；symlink 本身不是拒绝理由，canonical escape 才是拒绝理由。
