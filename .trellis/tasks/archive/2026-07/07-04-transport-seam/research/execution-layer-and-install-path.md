# 盘点：targets/ 执行层 + install 试点路径深读（2026-07-05）

> 由探查 agent 产出，行号基于 dev 分支当日快照。路径相对 `src-tauri/src/`。

## 1. targets/ 模块

### 1a. 构造与执行的分离度

SSH/WSL 是 `targets/exec.rs` 内两个近乎复制的 impl 块：`ConnectedSshTarget`（`exec.rs:139-442`，struct 在 `askpass.rs:2-6`：target/password/askpass_helper）、`ConnectedWslTarget`（`exec.rs:444-682`，struct `exec.rs:113-116`）。

- **命令构造**各自收敛在一个纯构造器：`ConnectedSshTarget::base_command()`（`exec.rs:140-198`，全部 `-o` 选项、key/password-askpass 分支、`user@host`）；`ConnectedWslTarget::base_command()`（`exec.rs:445-453`，`wsl -d <distro> --`）。
- **命令执行**散在每传输 ~7 个方法里，各自 `base_command()` 后立刻 spawn；远程 shell 载荷（`test -e`/`mkdir -p`/`cat >`/`cp -R`/`rm -rf`/`readlink`）在方法内 `format!` + `shell_quote`（`exec.rs:705-707`）内联拼装：`write_file`(:385/:624)、`copy_dir`(:397/:637)、`remove_tree`(:406/:646)、`inspect_path`(:345/:582)、`list_dir`(:418/:658)。
- 两个 impl 块 ~95% 复制，差异仅 `base_command()` 与 shell 前奏（WSL `sh -lc` vs SSH 裸参数；脚本 stdin：WSL `sh -s --`，SSH `remote_script_command`，`exec.rs:684-691`）。`targets/remote.rs` 的 `ConnectedRemoteTarget` 已手写枚举分发统一两者。
- 注意：方法是 `async fn` 但内部直接调 `std::process::Command::output()`（阻塞，无 spawn_blocking）——runner 契约按同步设计即保持现状。

### 1b. 进程 spawn 点（choke point 问题）

**无单一 choke point**——14 个 spawn 点，但全部经过两个构造器，且收敛为**两种执行形态**：(A) `stdin(null).output()` 捕获；(B) `spawn()`→写 stdin→`wait_with_output()`。`exists`/`inspect_path` 的退出码分支只是对捕获 `Output` 的后处理。

| file:line                    | 方法                               | 形态           |
| ---------------------------- | ---------------------------------- | -------------- |
| exec.rs:141 / :446           | SSH / WSL `base_command`           | 构造器         |
| exec.rs:271 / :502           | `run_command` `.output()`          | A              |
| exec.rs:287+:297 / :520+:530 | `run_command(_process)_with_stdin` | B              |
| exec.rs:311 / :546           | `run_command_bytes`                | A              |
| exec.rs:333 / :570           | `exists`                           | A+退出码       |
| exec.rs:354 / :593           | `inspect_path`                     | A+退出码       |
| wsl_discovery.rs:12+:32      | `list_wsl_distributions`           | 发现用，试点外 |

**runner 插入点**：`base_command()` 与两种形态之间——`fn run(&self, cmd, stdin: Option<&[u8]>) -> io::Result<Output>` 覆盖全部现场；`base_command()` 保持纯构造器不动。

### 1c. 对外 API 面

`mod.rs:35-42` re-export；`targets::` 命中 36 文件。核心消费者全部走 **`ConnectedRemoteTarget`**（`remote.rs:23-162`：run_script / run_command / run_command_with_stdin_bytes / exists / inspect_path / mkdir_p / write_file / read_file / copy_dir / remove_tree / remove_file / list_dir + remote_home / remote_os / symlink_allowed）——它已是事实上的传输接口，只是不可注入（唯一构造入口 `connect_remote_target`，`remote.rs:9-21`，真开 SSH/WSL）。消费者：`services/installation/remote.rs`、`installation/project.rs`、`services/local_remote_sync.rs`、`services/scanner/ssh_batch.rs`、`services/central_updates/*`、`commands/linker.rs:26`（connect_remote_target + ActiveTarget）、`commands/targets.rs`（TargetRegistry 全套）。

### 1d. targets/tests.rs 现状（38 条断言确认）

39 个 test fn（平台门控后单平台 ~38），**零进程执行**。三类：`command_arg_strings(...base_command())` 参数序断言（`tests.rs:334-459`）、纯解析器/helpers（`:213-271`）、凭据/askpass 用注入的 `MemoryCredentialBackend` fake（`tests.rs:7-66`——仓内唯一 DI 现场，注入的是凭据库不是 runner）。**今天不可测**：`base_command()` 之后的一切——run/exists/inspect/copy/remove 行为、退出码分支、stdin 管道、内联 shell 字符串。

## 2. Install 试点路径

### 2a. 壳层分发

`commands/linker.rs` 纯壳。`install_skill_to_agent`（`:47-113`）：`active_target`(:53) 分支——Local→按 method 三选一(:59-67)；远程→method 强转 symlink/copy 后 `_remote_impl`(:68-82)；其余是 OperationLogEvent(:84-112)。`batch_install_to_agents`(:253)、`batch_install_central_skills`(:421) 重复该分叉；远程批量预开一次 `connect_remote_target`(:297/:476) 复用 `install_skill_to_agent_ssh_with_connection`。

### 2b. 三份实现的重复度

**Local**（`native.rs:58-145`）：guard central(:74)→agent+central 查询(:79-86)→canonical_dir(:88)→`ensure_centralized`(:91)→同根 native 捷径 `agents_share_skills_dir`(:93)→create_dir_all(:105)→`detect_existing_agent_install` skip(:111)→`ensure_replaceable_target`(:119)→**相对** symlink(:122-129)→DB upsert(:132-140)。copy 变体（`:198-268`）除第 7 步 `copy_dir_all_blocking`(:252) 外逐行同构。auto（`:159-181`）仅 Windows `SymlinkCreate` 时回退 copy。

**Remote**（`remote.rs:224-331`）：`_remote_impl`/`_ssh_impl` 都是薄连接壳，共享 `install_skill_to_agent_ssh_with_connection`（`remote.rs:252-331`，名字带 ssh 实则 SSH+WSL 通吃）：guard(:259)→查询(:263-268)→`remote_join` canonical(:269)→同根捷径 `ensure_remote_centralized`(:276-288)→method 强转+`symlink_allowed` 门(:291-298)→installations 查 `managed_copy` 旗标(:299-304)→**`run_remote_central_install_script`**(:306-315)→标记中央化+DB(:316-330)。

**重复 vs 传输特有**：编排骨架（guard→查询→canonical→centralize→同根捷径→清槽→落链→记 DB）重复三遍（local symlink / local copy / remote）。真正传输特有很小：local 用离散 FS 调用；remote 把清槽+mkdir+symlink/copy 收进**一个原子 POSIX 脚本**（`REMOTE_CENTRAL_INSTALL_SCRIPT`，`remote.rs:19-67`）单次往返，existing-entry 分类在 shell 内做（exit 42/43）。local `ensure_replaceable_target`（`centralize.rs:77-99`）与 remote shell 的 `if [ -L ]/elif [ -e ]` 是**同一决策的两种语言**。skip 检测（`native.rs:111`）仅 local 有。

### 2c. installation/remote.rs 对 targets/ 的用法

从不碰 `Command`，全走 `ConnectedRemoteTarget`：`.exists()`(:99-119)、`.copy_dir()`(:122)、`.run_script()`(:162-174)、`.remove_tree()`(:366-369)。**所有传输错误被拍平**：`.map_err(|e| InstallationError::Remote(e.to_string()))`（6 处）——类型化 `TargetsError` 在此边界字符串化。test-only 的 `classify_remote_existing_install_target`（`remote.rs:188-222`）用 Rust 复刻 shell 的 exit-42/43 决策纯为可测——团队早想让 shell 决策进程内可测的证据。

### 2d. Install 相关错误变体

`InstallationError`（`error.rs:9-163`，40 变体）。install 相关：CentralAgentTarget(:33)、AgentNotFound(:39)、CentralAgentMissing(:42)、SkillNotFound(:45)、SkillSourceMissing(:48)、CanonicalSkillMissing(:51)、InvalidSkillFilePath(:54)、TargetOccupied(:64)、SymlinkCreate(:21，驱动 Windows 回退)、SymlinkUnsupported(:30)。远程特有：RemoteSymlinkDisabled(:102)、RemoteTargetOccupied(:116)、兜底 **`Remote(String)`**(:121)。不对称：remote 全部传输失败进一个 stringly 变体，local 失败类型化良好。

### 2e. installation 测试现状

`services/installation/tests.rs` 64 条（声称 66 的另 ~2 在内联模块）。`setup_db` 用 **in-memory pool**（`test_support::mem_pool`）+ TempDir 重定向 agent 目录；驱动 **local** impl 打真实临时文件系统断言 symlink/copy/DB 行——无 FS mock。**remote** impl 无执行路径覆盖，仅纯分类器单测（`tests.rs:27-32`）——正因无可注入 runner。

## 3. local_remote_sync

755 行，同样走 `ConnectedRemoteTarget` 但与 installation/remote.rs **零共享**：自己的快照/哈希/tar.gz 脚本（`:44-121`）、自己的错误枚举。与 installation 唯一交集是 targets/ 原语。三个远程消费者（installation、local_remote_sync、scanner）各自伸手 `ConnectedRemoteTarget` + 手拼 shell + 各域拍平错误——**runner 注入放在 `ConnectedRemoteTarget` 之下、三域同时受益**的最强论据。

## 综合建议

最干净注入点：`CommandRunner` trait 被 `ConnectedSshTarget`/`ConnectedWslTarget` 消费，`exec.rs` 14 个 spawn 点改 `self.runner.run(cmd, stdin)`；`base_command()` 维持纯构造器（已有参数序测试）。`ConnectedRemoteTarget` 已是三服务共依的操作级接口——让它可被 fake 注入即同时解锁 install/sync/scan 的执行路径测试。

**关键文件**：`targets/exec.rs`（构造器 :140/:445 + spawn 点表）、`targets/remote.rs:23-162`、`targets/askpass.rs:20-40`（connect_ssh_target）、`services/installation/native.rs:68-268`、`services/installation/remote.rs:19-331`、`services/installation/centralize.rs:77-99`、`services/local_remote_sync.rs:638-748`。
