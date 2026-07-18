# 统一技能导入入口与安全 ZIP 导入

## Goal

把 Central 当前单一的“GitHub 导入”按钮收敛为稳定的“添加技能”入口，并新增本地 ZIP 技能包的安全预览与原子导入。GitHub 继续复用现有 wizard；ZIP 只把经过确认的技能写入 Central，不直接绕过中央唯一真源安装到平台。

## Background

- 当前 Central 头部直接打开 `GitHubRepoImportWizard`（`src/components/central/CentralSkillsShell.tsx:354`）。
- GitHub 状态已由 Zustand slice 管理，Preview 与 Import 分别通过 typed IPC command（`src/stores/marketplaceStore.githubImportSlice.ts:41`、`:115`）。
- Tauri dialog 插件已经存在于前后端依赖和 capability 中（`package.json:36`、`src-tauri/Cargo.toml:21`、`src-tauri/capabilities/default.json:9`）。
- 当前 Rust 依赖没有 ZIP reader；实施需新增生产依赖，必须在任务启动前确认版本与许可。
- `src-tauri/Cargo.toml:40` 已直接依赖 `sha2 = "0.10"`，可复用 SHA-256 为 preview/import 建立内容一致性，不需要再引入哈希依赖。
- SkillKit 的 ZIP 实现直接 `extractAllTo` 后寻找 `SKILL.md`（`ref/skillkit/apps/desktop/electron/installer.ts:894`），缺少 SkillPort 所需的预算、路径冲突与原子持久化边界，只能借鉴交互入口，不能复制实现。

## Requirements

### R1. 统一入口

- Central 主操作显示“添加技能”，通过紧凑 menu/source picker 提供“GitHub 仓库”和“本地 ZIP”两种 intent。
- GitHub intent 直接打开现有 `GitHubRepoImportWizard`，不得再包一层 modal、复制 wizard 状态或改变 Preview → Confirm → Result 行为。
- 来源 router 必须是可复用的前端状态边界，为后续 deep-link prefill 提供单一入口。

### R2. ZIP 选择与预览

- 使用现有 Tauri dialog 选择 `.zip`；取消选择不改变当前状态。
- MVP 支持单个 skill 包：根 `SKILL.md`，或唯一一层包装目录下的 `SKILL.md`。发现多个候选、没有候选或结构模糊时明确报错，不猜测要导入哪个。
- Preview 至少显示 skill id/name/description、有效根目录、完整文件树、文件数、总展开字节和 Central 冲突。
- Preview DTO 同时返回后端计算的 archive fingerprint（SHA-256 + 压缩字节数），供 import 证明用户确认的仍是同一份 archive；fingerprint 不替代 import 的完整安全重验。
- Preview 只读，不创建 Central 目录、不写数据库、不记录成功日志。

### R3. 安全边界

- 在解压/写盘前拒绝绝对路径、`..`、反斜线逃逸、空路径、重复路径、大小写折叠冲突、文件/目录前缀冲突、符号链接、加密或不支持的条目类型。
- 复用 `ResourceBudget::default_skill()` 的文件数、展开字节和单文件上限；增加压缩包字节与压缩比检查，防止 zip bomb。
- ZIP 条目必须确定性排序；所有路径以剥离后的 skill 根为基准，根相对 `SKILL.md` 缺失时 fail closed。

### R4. 冲突与原子导入

- 冲突处理语义与 GitHub import 对齐：overwrite / rename / skip；rename 必须经过现有 skill id 规范化规则。
- Import 必须接收 preview 返回的 expected fingerprint，在压缩包预算内只读取一次 archive bytes，先比较 SHA-256 + byte length，再用同一份 bytes 重新执行 inventory/candidate/预算校验；不匹配返回 typed `archive_changed_since_preview`，不得静默导入替换后的内容。
- 确认后先在 staging 完成安全解压与完整校验，再获得 Central mutation guard，原子替换目标目录并在同一业务结果中更新数据库。
- 任意失败恢复旧目录、清理 staging/backup，不留下部分技能或成功结果。
- 记录持久化 Operation Log，包含来源类型、最终 skill id、resolution 和成功/失败摘要，不记录本地完整路径中的敏感用户目录。

### R5. 目标与可用性

- 本任务只把 ZIP 导入当前本机 Central。活动目标为 SSH/WSL 时，ZIP intent 保持可见但 disabled，并用 i18n 说明“本地 ZIP 暂不支持远程目标”。
- GitHub 在 Local/SSH/WSL 下继续沿用现有行为。
- 导入完成后刷新 Central；不在本任务中追加平台安装步骤，用户继续通过现有 InstallDialog 安装，从而保留 `ensure_centralized`。
- archive 来源默认保持无 repository assignment / unknown source；现有更新检查应将其视为 unsupported，而不是 GitHub 仓库异常或远端缺失。

### R6. UI 与 i18n

- 入口和 ZIP wizard 使用现有 Button/Menu/Dialog/FileTree 组件、lucide icon、focus-ring 和调度台 token；不新增 SkillCard 实现或嵌套卡片。
- loading、空、冲突、错误、取消、成功状态均可键盘操作；所有用户文本同步中英文。

## Acceptance Criteria

- [ ] Central 只有一个“添加技能”主入口；选择 GitHub 后现有 wizard 的 Preview/Confirm/Result 和测试语义保持不变。
- [ ] 本地根 skill ZIP 与单包装目录 ZIP 均能预览完整文件树，并在确认前无文件系统/数据库写入。
- [ ] Preview 返回 SHA-256 + byte length；未变化 archive 可导入，preview 后内容或长度变化会在任何 staging/Central/DB 写入前以 `archive_changed_since_preview` 失败。
- [ ] 多 skill、无 `SKILL.md`、路径穿越、绝对路径、重复/大小写冲突、symlink、加密条目、超预算和高压缩比 fixture 均 fail closed。
- [ ] overwrite/rename/skip 三种冲突结果正确；写入失败会恢复旧目录并清理 staging/backup。
- [ ] 成功导入刷新 Central，记录经过脱敏的 Operation Log；平台安装仍走现有链路。
- [ ] archive 来源 skill 没有 GitHub repository assignment，更新检查稳定显示 unsupported/unknown source，不产生 remote-missing 或仓库加载错误。
- [ ] SSH/WSL 下 ZIP 有明确 disabled 说明，GitHub remote import 无回归。
- [ ] 前端测试覆盖 source picker、取消、preview、冲突、错误、成功和 remote disabled；Rust 测试覆盖 ZIP 解析安全矩阵与原子回滚。
- [ ] `pnpm typecheck`、`pnpm lint`、相关 Vitest、相关 Rust tests、`cargo clippy -- -D warnings`、`git diff --check` 和 `just ci` 通过。

## Out of Scope

- 多 skill ZIP 批量选择、逐文件排除、ZIP 内文件内容预览。
- 直接把 ZIP 安装到 agent 或项目目录。
- 把本机 ZIP 上传到 SSH/WSL 活动目标。
- `.tar.gz`、`.7z`、在线分享链接、账号或云存储。
- 修改 GitHub preview DTO、selection 或持久化契约。
