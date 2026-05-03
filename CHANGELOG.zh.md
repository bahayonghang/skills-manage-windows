# 更新日志

本文件记录该项目的重要变更。

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
