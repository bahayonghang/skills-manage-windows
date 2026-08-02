# 更新日志

本文件记录该项目的重要变更。

## [Unreleased]

### 新功能

- **更新中心**：统一 dialog，把可更新 skills、远端新增、远端删除、平台冗余合到一个屏幕，每条都支持逐项决策。取代原本独立的 **检查更新**、**远端缺失**、**仓库同步** 三个 dialog。文档见 `docs/zh/guide/update-center.md`。
- 新增 Tauri 命令 `refresh_skill_update_inventory`、`apply_skill_update_decisions`、`clear_skill_update_inventory`、`get_skill_update_inventory`、`scan_platform_duplicate_skills`。
- 新增 DB 表 `skill_repository_pending_additions`，新增字段 `skill_repositories.last_synced_at`，用于跨 session 持久化刷新结果。
- 后端 `SkillUpdateStatus` enum 取代原来的字符串常量。
- **Central 技能库**新增仓库/来源导航、顶部筛选、自适应卡片网格、布局与密度切换、卡片内标签编辑，以及单个或批量从平台卸载。
- 更新检查支持当前平台或指定仓库范围，提供明确的刷新模式、按仓库进度、重试反馈，并支持一键清理平台残留副本。
- 新增按目标感知的 Central 状态导出/导入，支持本机、SSH 和 WSL 目标，包含来源目标元数据、v1 JSON 兼容和跨目标导入提示。
- 新增 Reasonix、Grok 内置平台目标，以及按安装来源导航和按调用次数排序的平台视图，用于区分 SkillPort 管理、仓库关联和独立安装的技能。
- 新增状态与待办队列驾驶舱首页，展示操作活动和热门标签摘要；使用统计新增平台筛选、排行、热力图、Provider 健康度、最近调用和技能详情。
- 扩展内置标签分类，保护用户自定义标签；AI 新标签必须经过复核后才能创建，并新增可展示进度、取消状态和限速风险的 AI 任务控制舱。
- 新增本地运行时诊断控制台，将 Operation Log 与 Runtime Log 分开，并提供脱敏读取、导出和清理流程。
- 统一本地 ZIP 与 GitHub 仓库导入，支持安全 ZIP 预览/导入、插件清单与文件树预览、选中子树导入、受限的 tree manifest 快速路径与 archive 回退，以及指定分支导入。
- 新增安全 GitHub 导入深链、待处理意图队列，以及 Windows 单实例冷启动/热启动生命周期处理。
- 新增稳定 skill 标识和版本化共享 `skillport-cli` JSON 契约，保证桌面端与 CLI 互操作。

### 调整

- 桌面发布收口到 tag 触发的 `Release Desktop` workflow，并补充发布检查清单与 updater 不变量文档。
- 由于当前 updater metadata 仅覆盖 Windows，应用内更新现在会在非 Windows 桌面构建上 fail closed。
- PlatformView 的 **扫描重复** 按钮现在跳转到更新中心的 **平台冗余** Tab，不再打开独立 dialog。
- `UnifiedSkillCard` 新增 inventory-only 的 badge 展示（平台冗余、失链孤儿）。
- `DuplicatePlatformSkillsDialog`、`CentralUpdateConfirmDialog`、`RemoteMissingSkillsDialog`、`CentralRepositorySyncDialog` 已标记 `@deprecated`，下一个 minor release 之后移除。
- Central 与 Platform 页面支持可配置字体 fallback 和字号缩放，统一紧凑排版与语义状态色，并补充快捷键速查、键盘焦点和无障碍行为。
- 远程更新与同步现在会批量处理 SSH/WSL 操作，按任务隔离取消状态，提供有界进度，并监督进程树生命周期。

### 修复

- GitHub 导入现在会保留根级技能内容，不再把合法的顶层 `skill/SKILL.md` 误判为无效目录；可恢复的预览失败会继续走回退路径，已删除仓库也不会留下过期条目。
- Central 刷新与清理会严格限定在请求的目标和平台范围内，处理仓库/路径迁移，保留跳过决策，并在不混淆 inventory 与已安装基线的前提下展示失败检查。
- Central 写操作与数据库迁移改为通过版本化备份、外键清理、事务化关系修复和可恢复的文件系统/数据库操作日志进行恢复。
- 数据库或目录在启动阶段异常时，应用现在进入恢复流程而不是直接退出；损坏的目标配置也会与健康目标隔离。

### 安全

- GitHub 导入现在执行有界资源预算、受限的 raw fetch 来源校验和安全回退；远程 Central 路径会拒绝符号链接逃逸。
- 收紧 renderer 权限与凭据访问，由后端策略统一控制；Operation Log 与 Runtime Log 共用敏感值脱敏策略。

### 发布与 CI

- 桌面发布现在分别校验 Windows EXE、NSIS、MSI 的 Authenticode 与 updater 签名，核对 `latest.json` 和校验和，执行安装 smoke 检查，并仅在完整产物集通过验证后发布。
- 发布构建现在以可移植方式准备 macOS universal CLI 与桌面产物，并明确 Tauri 多 binary 入口。
- CI 现在并行执行 common 与跨平台门禁，增加 fail-closed 供应链和生成文档漂移检查，仅验证目标为 `dev` 或 `main` 的 PR，并部署唯一经过线上 smoke 的 Pages 产物。

## 0.11.0 - 2026-05-11

这是 `0.10.x → 0.11.0` 版本线的安全与中央技能库 V2 收口版本。本版本继续保持 Windows-first 打包契约不变，同时让 Central Skills 成为默认的高信息密度工作区。

### 新功能

- 中央技能库 V2 默认开启，并保留“切回经典布局”链接与 `featureFlag.central.newLayout=off` 排障开关。
- 新增 Central 结构化搜索语法，支持 `tag:`、`repo:`、`owner:`、`source:`、`has:`、`platform:`、`created:`、`updated:`，同时保留自由文本搜索。
- 新增 URL-backed Central view state，把查询、多选 facet、排序、分组、视图模式和 saved view 身份写入 URL。
- 新增本地 Saved Views，用于保存常用 Central 查询，并接入侧边栏与命令面板。
- 新增 Tag Groups 与侧边栏分配入口，大量标签可按一级分组整理，且不会移动 skill 文件。
- 新增按仓库、owner、标签、更新状态分组的 group-by 视图。
- 新增 Central V2 命令面板动作：保存当前视图、创建标签分组、切换 group-by 模式、退回经典布局。

### 安全

- GitHub PAT 不再以明文写入 SQLite settings，改走统一 SecretStore，并复用 keyring、Windows DPAPI、session fallback 三层存储能力。
- AI API Key 改走 SecretStore；AI provider、region、model、URL、标签并发与间隔等非敏感配置仍保留在普通 settings。
- 为旧版 `github_pat` 与 `ai_api_key` settings 行增加一次性安全迁移：只有 SecretStore 写入并读回成功后，才清理旧明文值。
- generic settings IPC 阻止读写受保护 secret key，避免后续通过 `get_setting`、`set_setting`、`get_settings`、`set_settings` 重新引入明文写入。

### 改进

- Settings store 剩余 action 统一改用 `@/lib/tauri` 封装，让浏览器 fixture 与 Tauri runtime 判断和其他 store 保持一致。
- 在确认计划中的 `serde_yml` 替代项同样存在 RustSec 风险后，将 SKILL.md frontmatter 解析从 `serde_yaml` 切换到 `serde_norway`。
- 收敛 operation-log warning 测试，避免并发 Rust 测试继续依赖旧的 flaky SQL 错误路径。
- Central Skills 指南和 README 已补充 V2 搜索、Saved Views、Tag Groups、group-by、命令面板和回滚说明。
- 本版本不改变 Windows 打包契约；未修改安装器、签名或 bundle 输出路径。

### 已知缺口

- Saved Views 与 Tag Groups 的拖拽排序 UI 延后；后端 IPC 与 store action 已就绪。
- Saved View 与 Tag Group 管理仍使用轻量 prompt/confirm，后续再接统一 Dialog primitive。
- 后端 FTS5 搜索继续延后到技能规模或 p95 过滤耗时达到触发条件之后。
## 0.10.0 - 2026-05-03

这是一次围绕上游 0.10.0 对齐、Linux 桌面打包补齐，以及 Discover 安装链路修正的版本升级，同时继续保持当前 SkillPort fork 的 Windows-first 发布契约。

### 新功能

- 新增 Linux 桌面打包 metadata、模板和 GitHub Actions 任务，可在 SkillPort 身份下产出 `.deb`、`.rpm`、`.AppImage`。
- Discover 安装平台时，现有安装弹窗里的 `symlink` / `copy` 选项会一路透传到 Rust 后端。
- 恢复 Windows release 产物矩阵，发布时同时准备 NSIS `.exe`、MSI 和便携 ZIP。

### 改进

- 为 Tauri bundle 配置补齐 publisher、homepage、licenseFile、description 和 AppStream 集成等跨平台元数据。
- 对 Discover 共享目录模式（如 `.agents/skills`）做去重，减少同一项目被重复映射成多个平台来源。
- 新增 0.10.0 release notes，并同步 README 下载说明以覆盖新的桌面包矩阵。

### 修复

- 删除已跟踪的 `.factory/` 工厂产物，并忽略后续 `.factory/` 输出，同时继续保留 `AGENTS.md`。
- Linux 打包全程保持 `SkillPort` / `skillport` / `com.bahayonghang.skillport` 身份，不回退到上游品牌。

## 0.9.1 - 2026-04-23

这是一次以完整路径显示一致性和 README 细节补充为主的小型维护版本。

### 修复

- 中央技能库、平台页、设置页、全局搜索与平台编辑流程统一显示完整绝对路径，不再将路径折叠成 `~`。
- Windows 平台的路径展示统一为带盘符的反斜杠风格。
- 自定义平台的自动生成目录会根据当前平台的 home 目录风格生成对应路径。

### 改进

- 在中英文 README 中补充 `Star History` 小节。
- 补充路径 helper 与相关 UI 断言测试，覆盖新的显示规则。

## 0.9.0 - 2026-04-23

这次版本把上游 0.9.0 合进当前 fork，同时保留 Windows 安装包优先的发布契约。

### 新功能

- 合并上游 0.9.0 的桌面打包链路，让 release 构建可产出 Windows NSIS、Windows MSI、Windows ZIP，以及 macOS universal DMG/ZIP/TAR.GZ。
- 为 Claude 多来源技能补齐 source-aware 平台行、详情加载和 explanation 连续性。
- 在前后端补齐 Windows 友好的路径展示能力，包括 UI 里的 home 路径压缩显示。

### 修复

- 保留当前 fork 的 `~/.agents/skills` Windows 路径规则，同时把上游 home 展开和跨平台路径工具吸收到现有 `paths.rs` 模块。
- 在 Windows 无法创建符号链接时，平台安装和导入链路可自动回退到 copy。
- 让全量重扫同时刷新 central、platform、discover 三条状态链，避免计数和行状态不同步。
- 保留本地 bootstrap hydration、平台可见性和 agent 启停能力，同时把 Claude source-specific 行标识整合进现有数据模型。

## 0.8.2 - 2026-04-23

这个补丁版本主要收敛启动缓存、轻量刷新和大列表渲染成本。

### 性能优化

- 启动时先用缓存快照渲染应用壳层，再后台刷新扫描结果，让平台数量、集合数量和 Discover 计数先出来。
- 中央技能库和平台技能库的大卡片列表改成虚拟网格，并收敛重复卡片与图标渲染，降低滚动和筛选开销。

### 修复

- 统一 Windows 家目录与中央技能目录解析，避免缓存扫描、安装和导入链路与 `~/.agents/skills` 脱节。
- 对共享扫描目录的平台合并处理，并在一个事务里更新安装缓存，减少重扫后的旧计数残留。
- 新增轻量 bootstrap 和 discover summary 接口，让侧边栏和计数刷新不再为拿数量去加载整份数据。
- 改成在平台安装弹窗打开时再懒加载中央技能库，不再每次进入平台页都预加载。

## 0.8.1 - 2026-04-23

用于修复 GitHub 发布流水线的补丁版本。

### 修复

- 将 release workflow 改为使用已发布的 `tauri-apps/tauri-action@action-v0.6.2` 标签，避免 Windows 和 macOS 发布任务在启动阶段直接失败。

## 0.8.0 - 2026-04-20

首个公开发布版本。

### 新功能

- 发布基于 Tauri 的 `skills-manage` 桌面应用，用统一界面管理内置与自定义平台上的 AI agent skills。
- 新增平台视图与中央技能库视图，支持安装、卸载、符号链接状态识别和 canonical skill 管理。
- 新增完整的技能详情体验，包含 Markdown 预览、原位抽屉导航、安装操作与集合相关工作流。
- 新增技能集合管理、自定义平台设置、扫描目录配置、首次使用引导、Toast 反馈与响应式侧边栏。
- 新增中英文界面、Catppuccin 多风格主题系统、强调色切换以及全局命令面板。
- 新增项目级 Discover 扫描，支持递归发现、结果缓存、停止扫描、导入中央技能库以及更好的上下文保留。
- 新增 marketplace 浏览、预览抽屉、自动集中安装，以及 AI 技能解释能力。
- 新增 GitHub 仓库导入流程，支持预览、镜像回退重试、可选鉴权请求、选择状态保持以及导入后安装到平台。

### 性能优化

- 通过延迟查询、懒加载索引、轻量搜索结果卡片和长列表虚拟化，改善全局搜索、中央技能搜索和项目技能浏览性能。

### 修复

- 强化 AI explanation 流程，拒绝空白缓存内容，并在缓存损坏为空时自动重新生成。
- 改进 frontmatter 处理逻辑，稳定提取 `name`、`description`、`version` 等结构化字段，避免原始 YAML 混入 Markdown 预览。
- 在技能详情中展示已加入的集合，并在“加入技能集”时默认选中已存在集合。
- 优化详情抽屉、marketplace 预览和 GitHub 导入界面布局，减少跳转带来的上下文丢失。
