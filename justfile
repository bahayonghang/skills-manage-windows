set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# ========================================================================
# 步骤1：同步应用版本
# ========================================================================
# 目标：
# 1) 以 package.json 为唯一版本源
# 2) 同步 Tauri / Cargo 元数据中的应用版本
sync-version:
    node scripts/sync-version.mjs

# 只读检查版本元数据，不修改工作树
version-check:
    pnpm version:check

# 只读诊断本机工具链；不会安装、切换或修改任何环境变量
doctor:
    node scripts/doctor.mjs

# 快速反馈入口；先自动同步版本元数据，再运行 quick lane
# 提交前仍必须运行 just ci 与 just audit
check: sync-version
    node scripts/run-ci.mjs --lane quick

# ========================================================================
# 步骤2：检查工作流
# ========================================================================
# 目标：
# 1) 本地入口先自动同步版本元数据（与 sync-version 相同），避免 version:check 因漂移中断
# 2) common 与当前平台 Rust lane 并行运行
# 3) 生成物检查保持只读，漂移时失败而不修改工作树
# 4) GitHub Actions 直接调用 run-ci.mjs，不经过 just，因此 CI 侧版本检查仍然只读
# 5) common 负责前端、文档、格式与静态合同；Rust lane 负责 Clippy 与测试
ci: sync-version
    node scripts/run-ci.mjs

# ========================================================================
# 步骤2.1：审计生产依赖
# ========================================================================
# 目标：
# 1) 阻断未批准的 npm high/critical advisory
# 2) 阻断无精确例外的 Cargo vulnerability
audit:
    pnpm audit:dependencies

# ========================================================================
# 步骤3：构建桌面应用
# ========================================================================
# 目标：
# 1) 按当前平台构建 Tauri 应用
# 2) 把当前平台的安装包/包产物复制到项目根目录 outputs/
build: sync-version
    node scripts/build.mjs

# ========================================================================
# 步骤4：构建并安装桌面应用
# ========================================================================
# 目标：
# 1) macOS 上提示并转为 just build
# 2) Windows 上复用 build 生成的 NSIS 产物并以 passive 模式运行安装器
install:
    @just {{ if os() == "macos" { "_install_macos" } else if os() == "windows" { "_install_windows" } else { "_install_unsupported" } }}

_install_macos:
    @echo "[install] macOS detected; running just build instead of installing."
    @just build

_install_windows: build
    node scripts/install.mjs

_install_unsupported:
    node scripts/install.mjs

# ========================================================================
# 步骤5：启动开发模式
# ========================================================================
# 目标：
# 1) 启动 Tauri 开发环境
# 2) 直接运行桌面应用
dev:
    pnpm tauri dev

# ========================================================================
# 步骤6：本地预览文档站
# ========================================================================
# 目标：
# 1) 先按当前 Rust/数据库源码刷新 IPC 字典与 schema 表（pnpm docs:gen）
# 2) 启动 VitePress 开发服务器，srcDir 为仓库根 docs/
docs:
    pnpm docs:dev
