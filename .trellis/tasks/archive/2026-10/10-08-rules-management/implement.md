# Rules 实施与验收计划

状态：产品实现、独立复核与 `just ci` 已通过。测试不改真实用户规则。逐层证据和未运行项见 [validation/results.md](validation/results.md)。任务保留 `in_progress`。

## 组织与前置条件

同一fullstack任务交付共享界面、文件库和两工具链接。两个工具依赖同一正文/service，缺少共用界面不能独立完成验收，因此不拆父子任务。

预计超过8个产品/测试文件，包含需求内的1个Rules service。trellis-implement负责业务代码，trellis-check独立复核，主会话负责协调与最终证据。原Central、Skills CLI、目录链接及junction行为保持原样。

- [x] 用户在后续消息批准最新prd.md、design.md、ui-design.md及本计划，原文「开始实施」。
- [x] 重新读取工作区，只有该任务文档未跟踪，不自动切分支、提交或push。
- [x] 用户补充授权「使用最新版本的pnpm」「删除仓库的声明版本，优先使用最新版」：移除 pnpm 精确版本声明，同步 doctor、CI 和现行文档。Node 26 与 Rust 1.98.0 保留；不改系统 PATH 或其他依赖。
- [x] manifest 已验证，已执行 `python .trellis/scripts/task.py start .trellis/tasks/10-08-rules-management`。
- [x] 委派提示以Active task开头；原生上下文注入缺失时子代理主动读取任务和manifest。

## 1. 后端

- [x] paths.rs新增Rules专用根、固定工具解析及受校验子路径，保留旧常量；测试自定义根/profile的首版拒绝策略。
- [x] 新services/rules实现文件模型、共享头部、bounded读取、revision、状态扫描、错误枚举、备份及receipts；使用blocking-FS和现有Local guard。
- [x] Rules模块使用Windows文件symlink_file，复用相对路径计算；不改现有symlink_dir/目录junction，不引入copy降级。
- [x] 实现导入预览/导入、创建/保存、逐工具启停、删除与恢复；每个破坏性阶段均可从备份恢复，不覆盖外部变化。
- [x] commands/rules.rs、commands/mod.rs、services/mod.rs、ipc_registry.rs注册命令和操作策略；接入现有IPC边界、reviewed错误码、脱敏与i18n。
- [x] 运行pnpm ipc:codegen和pnpm docs:gen，生成命令类型及架构输出，不手改生成文件。

## 2. 前端

- [x] 新src/stores/rulesStore.ts及领域类型owner，避免@/types barrel增长；处理请求归属、dirty、失败草稿及切页/target后的草稿恢复，不迁移BrowserRouter。
- [x] 新src/pages/RulesView.tsx及src/components/rules中的列表、编辑区、两工具行和确认流程。
- [x] App.tsx增加lazy route，Sidebar.tsx在Central后插入Rules。
- [x] 复用字体偏好、Tailwind4、Textarea、Button、Markdown依赖和样式，无新增UI依赖，无SKILL.md标记。
- [x] en.json/zh.json增加所有新可见文本、状态和错误。既有IPC fixtures增加隔离Rules数据与mutation行为。
- [x] README.md/README_CN.md同步功能、路径、导入/接管差异、备份和首版限制；更新相关路径、文件链接与renderer契约spec。

## 3. 定向检查

所有mutation测试在临时目录中运行，不能改真实~/.claude/rules、~/.omp/agent/rules或初始化真实~/.skillport。

- [ ] Rust：正文/备份字节，重复导入，同名异文，非Local读前拒绝，覆盖路径/非法名称，Windows真实文件链接、中文/空格、相对目标，权限/占用失败，外部链接保留，断链所有权。**PARTIAL：除真实 symlink 权限不足状态外均有通过证据；未修改系统权限。**
- [x] Rust故障注入：备份、receipt、原子写入、链接提交、部分停用/删除各阶段失败；断言原文件或备份可读，恢复不覆盖外部编辑。保存revision冲突保留旧内容。
- [x] Store：草稿保护、旧请求归属、启停pending与失败反馈、异文拒绝接管、恢复受阻。
- [x] Page：导航、搜索、编辑/预览、导入、两工具分别启停、dirty规则切换确认、切页/target后的草稿保留、非Local禁用、中英文和恢复状态。

计划命令，在新增测试文件后执行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked services::rules
pnpm exec vitest run src/test/stores/rulesStore.test.ts src/test/pages/RulesView.test.tsx src/test/components/layout/Sidebar.test.tsx
pnpm typecheck
pnpm lint
pnpm ipc:codegen:check
pnpm docs:gen:check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
```

## 4. 运行证据与完整门槛

- [x] Windows隔离home：验证symlink_metadata、文件链接类型、read_link、入口回读；编辑后两个入口均更新。开发者模式配置不能替代验收。
- [x] 使用隔离目录和无付费调用的发现探针核实OMP18.8.6发现symlink并进入always-apply。若只能通过模型会话验证，未经真实调用授权保留UNVERIFIED；失败返回规划。
- [x] Claude隔离配置/会话检查共享规则发现。文件回读与CLI实际加载分开记录，不写真实用户home。
- [x] UI在1200×800、900×800、1200×600、900×600及布局边界两侧检查；中英文、导航展开/收起、最大字体scale、长文件名、空态、冲突、未保存、权限失败留截图。
- [ ] 原生Windows Tauri检查/rules的编辑、链接状态和确认流程。**PREPARED_NOT_RUN：原生控制不可用，人工路径见 results.md；浏览器 fixture 不替代原生。**
- [x] 最终运行just ci。定向检查不替代完整门槛，只有新失败/新修改才扩大或重复测试。
- [x] 用户授权的 pnpm 策略变更涉及打包工作流，执行 `pnpm tauri build --bundles nsis` 并核实新 NSIS。**PASS，exit 0；产物大小、版本与 SHA-256 见 results.md。未执行安装。**

## 5. 独立复核与回退

- [x] trellis-check检查全部diff、AC到测试映射、实际截图和原始失败/修复后retest；修复任务引入的问题。
- [x] 确认用户规则正文未复制到仓库、无秘密暴露、旧路径/技能UID/目录链接行为未变。
- [x] 验回退只解除已验证本功能链接并恢复备份原字节；外部改动导致停止，保留中央内容和receipt。
- [x] 报告各证据层状态。提交、push、PR、归档和发布等待对应明确授权。
