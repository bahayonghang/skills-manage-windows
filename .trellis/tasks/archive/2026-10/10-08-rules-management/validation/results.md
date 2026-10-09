# Rules 实施验收记录

日期：2026-10-08。工作区：`dev`。状态：产品实现和完整本机 CI 通过；原生交互与真实用户导入尚未执行。任务保留 `in_progress`，没有提交、push、PR、归档或发布。

## 交付范围

- 左侧 Rules 入口及 `/rules` 页面：搜索、新建、正文编辑、Markdown 预览、完整源文件、保存、删除和恢复确认。
- 中央库为 `~/.skillport/rules`，Claude Code 与 OMP 通过真实文件 symlink 独立启停。导入与工具接管是两个操作。
- 同内容普通文件在确认后备份再接管；异文、外部链接、外部版本修改和恢复冲突保留原对象。备份与恢复 receipt 记录在中央库内。
- 会话草稿在切页和切目标后保留，未保存的规则切换提供保存、放弃和取消。SSH/WSL 不读取本机规则。
- pnpm 精确版本声明已删除，Actions 使用 `latest`。doctor 检查可用性并报告实际版本。Node 26、Rust 1.98.0 与应用版本 1.0.2 保留。

最新版本核对输出：

```text
pnpm --version                  12.10.1
npm view pnpm dist-tags.latest   12.10.1
```

pnpm 12 的有效 overrides 移入 `pnpm-workspace.yaml`。原先被 package.json 遮蔽的 workspace overrides 没有合并；依赖、`pnpm-lock.yaml` 和 `Cargo.lock` 未改变。`verifyDepsBeforeRun: false` 使 run/exec 使用当前依赖，安装通过显式 install 执行。

## 正式门槛

所有 Cargo 检查和打包使用子进程环境：

```powershell
$env:CARGO_TARGET_DIR='C:\Users\lyh\AppData\Local\Temp\skillport-rules-cargo'
just ci
```

现有 `src-tauri/target` 链接访问失败，未更改该链接、ACL 或系统配置。

`just ci`：**PASS，exit 0**。原始日志：`tmp/rules-validation/ci.log`。

```text
[common:test]  Test Files  180 passed (180)
[common:test]       Tests  2102 passed (2102)
[rust-platform:test] test result: ok. 1579 passed; 0 failed; 9 ignored
[rust-platform:test] 其他 4 个非空测试组：3、3、4、5 passed；0 failed
[rust-platform:trellis-python] Ran 54 tests in 6.173s
[rust-platform:trellis-python] OK (skipped=4)
[ci] All checks passed.
```

门槛包含版本与生成文档检查、typecheck、lint、capability、size、entrypoint、Rust fmt、IPC codegen、Vitest、前端 build、文档站 build、Clippy、locked Rust tests、Trellis Python。

9 个 Rust ignored 项保持原状态：2 个 live WSL 项、4 个性能/离线大数据探针、1 个隔离 home 手工检查、1 个真实 home usage benchmark、1 个由 ProcessRunner 测试启动的 child fixture。4 个 Python skips 为 Windows 上未运行的 POSIX symlink 或 process-group 检查。这些项目没有记为 PASS。

定向检查和独立修复记录见 [review.md](review.md)：95 项前端/契约检查、19 项 Rules Rust 检查、1 项包含 3 种结果组合的命令审计检查通过。pnpm 修改的 49 项定向检查和 `just doctor` 8 项检查通过。

## 浏览器运行与视觉证据

使用已安装 Chrome 155.0.8059.39 的隔离 headless profile。Node 26 提供 CDP WebSocket，不安装浏览器依赖，不控制用户浏览器 profile。

```powershell
# 开发服务器在独立进程内运行
node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 24202 --strictPort
$env:RULES_CHROME_PATH='C:\Program Files\Google\Chrome\Application\chrome.exe'
node .trellis/tasks/10-08-rules-management/validation/rules-browser-probe.mjs
```

输出：`PASS`，8 个语言/窗口组合与 1 个最大 scale 组合。证据在 `tmp/rules-ui/report.json`、`tmp/rules-ui/*.png` 和 `tmp/rules-validation/browser.log`，均为本机未跟踪产物。

- 中英文分别运行 1200×800、900×800、1200×600、900×600，页面无横向溢出。主会话查看全部 8 张基础截图。
- 额外检查 719/721 和 1099/1101 两侧布局、导航收起、字体 preset 1.125 与 scale 1.5、长文件名、空态、冲突、未保存确认、恢复和权限错误状态。
- 实际 DOM 交互验证 Markdown 预览、只读源文件 YAML、编辑、dirty 切换、取消保留草稿、保存、导入预览及接管确认。预览没有 `SKILL.md` 标记。
- 首轮 scale 1.5 下工具区被裁切，原始截图保留在 `first-failure-max-scale.png`。复验时详情内部滚动，OMP 操作可滚动到窗口内；断言 `top >= 0`、`bottom <= 600`。

这些运行使用合成 IPC fixtures。浏览器证据没有验证 Tauri WebView、原生 IPC 或真实磁盘操作。

## 文件链接与工具发现

| 证据 | 状态与边界 |
| --- | --- |
| Windows Rust 隔离 home | PASS。实际 `symlink_file`、中文/空格名称、相对目标、双工具入口回读、保存后更新、原字节备份和恢复；19 项 Rules 测试 |
| OMP 18.8.6 安装包 helper/parser/session bucket | PASS。隔离 synthetic rule 的真实相对文件链接被发现；1 个 discovered、1 个 alwaysApply、0 个 rulebook、0 个 warnings。无模型请求 |
| Claude Code 2.1.292 CLI 初始化/context | DISCOVERED。隔离 home/config、真实文件链接，SDK initialize 和 get_context_usage 成功，rule 出现在 context 中，tokens 为 11；exit 0、未超时。无 user turn |
| 工具实际模型行为遵守规则 | UNVERIFIED。没有 provider 调用 |
| symlink 权限不足的真实系统状态 | NOT_RUN。没有关闭开发者模式、切换提权或修改系统权限；已有真实链接成功、占用失败、错误映射和浏览器权限提示证据 |

复现脚本：`omp-symlink-probe.mjs` 使用 Bun 和 `OMP_PACKAGE_ROOT`；`claude-symlink-probe.mjs` 使用 Node 和 `RULES_CLAUDE_PATH`。脚本创建临时目录，使用合成规则，不修改真实工具规则目录。Claude 探针将 provider 地址设为不可用的 loopback 地址，只发送控制请求。

真实 Claude/OMP 各 6 个现有文件仍为原文件；检查时文件长度与规划一致，`LastWriteTimeUtc` 均为 `2026-10-06 15:33:52`。真实 `~/.skillport/rules` 没有创建。真实初始化由界面中的导入和逐工具接管触发。

## Windows 安装包

`pnpm tauri build --bundles nsis`：**PASS，exit 0**。日志为 `tmp/rules-validation/nsis-build.log`。桌面 release 编译 4m 27s，`beforeBundleCommand` 的 CLI release 编译 3m 27s，makensis 生成 1 个 x64 bundle。

| 字段 | 结果 |
| --- | --- |
| 本机副本 | `outputs/rules-management-20261008-213839/SkillPort_1.0.2_x64-setup.exe` |
| 文件大小 | 14,099,177 bytes |
| FileVersion / ProductVersion | 1.0.2 / 1.0.2 |
| SHA-256 | `A660056BCDD61D582B52EAC4168BC6C0920ED38D6E39E502DB362FD17888EF0A` |
| 构建时间 UTC | `2026-10-09T02:37:11.8117262Z` |

原 bundle 与仓库输出副本 SHA-256 相同。元数据在 `tmp/rules-validation/nsis-artifact.json`。构建与副本检查没有执行安装、升级、卸载、签名或发布。

## 验收矩阵

| AC | 状态 | 证据或缺口 |
| --- | --- | --- |
| AC1 | PASS 浏览器/测试 | route、展开/收起侧栏、中英文 |
| AC2 | PASS 隔离；NOT_RUN 真实导入 | 正文字节、两来源备份、重复导入及冲突通过；真实 6 文件保留 |
| AC3 | PASS 浏览器/测试 | 新建、搜索、预览、保存、删除、草稿及 revision 冲突 |
| AC4 | PASS Windows 文件系统 | 双工具真实文件 symlink 指向单份文件，编辑后回读更新 |
| AC5 | PASS 隔离 | 仅解除验证归属的链接，外部文件和链接保留 |
| AC6 | PARTIAL | 故障注入、占用失败、恢复冲突通过；真实 symlink 权限不足状态未运行 |
| AC7 | PARTIAL | 两工具发现/注入阶段证据通过；模型应用行为 UNVERIFIED |
| AC8 | PASS 浏览器；UNVERIFIED 原生 | 4 尺寸×2语言、scale、边界、确认和工具可达性；Tauri WebView 未运行 |
| AC9 | PASS 测试 | 非 Local 前置拒绝、目标 generation、旧请求清理、草稿保留 |
| AC10 | PASS CI/Windows 文件系统/NSIS 构建 | 原生 IPC、安装流程、hosted CI、Linux/macOS 未验证 |

## 回退与未运行层

回退在 Rules 页面选择未完成操作并确认恢复。服务校验 receipt、fingerprint、备份字节与目标链接，只解除已确认归属的链接，恢复完整原文件。目标已被外部修改时返回 `rules.recovery_blocked`，保留中央文件、备份及 receipt。故障注入回归覆盖备份、receipt、原子写入、链接提交、部分停用/删除与外部改动。

原生人工路径：启动新构建的 SkillPort，在 Local → Rules 导入合成规则，逐工具启用、修改并保存、回读两个入口、停用与恢复；检查 900×600、最大字体、dirty 切换和确认框。该路径为 **PREPARED_NOT_RUN**，没有使用真实用户规则替代隔离数据。

安装/升级/卸载、签名/发布、真实用户导入、provider 调用、hosted CI、Linux/macOS：**NOT_RUN 或 UNVERIFIED**。没有创建这些层的通过结论。

## 保留的首次失败

- 规划期 doctor：pnpm 12.10.1 与原 pin 10.34.5 不一致。用户随后授权移除 pnpm pin，修复后 doctor 通过。
- Cargo 原 target 路径访问失败；使用进程级 `CARGO_TARGET_DIR` 复验，不修复原路径。
- 初始 Rules Rust 编译的 SHA-256 格式与 env closure 类型错误、IPC codegen 的 Specta u64 错误已修复；后续完整 CI 通过。
- 初始页面测试有 2 个可访问名称失败，修复后通过。
- pnpm 12 初始依赖验证/旧 overrides 配置失败；有效 overrides 迁移后，隔离 frozen/offline lock 检查通过，正式 CI 未改 lockfile。初始 exec 清理发生于 disposable fixture，当前 run/exec 禁用隐式依赖安装。
- 独立复核新增回归首轮 15 passed/4 failed；扩大 IPC 首轮 94 passed/1 failed。原始失败和修复详情见 `review.md`。
- 浏览器探针首次动态 import 命中了不同的 HMR module instance，导致等待超时；探针改用当前加载模块 URL。该失败属于探针；工具区裁切属于已修复产品问题。

最终源码和检查以当前实现为准，首次失败保留为历史证据。
