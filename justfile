set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# ========================================================================
# 步骤1：同步应用版本
# ========================================================================
# 目标：
# 1) 以 package.json 为唯一版本源
# 2) 同步 Tauri / Cargo 元数据中的应用版本
sync-version:
    node scripts/sync-version.mjs

# ========================================================================
# 步骤2：检查工作流
# ========================================================================
# 目标：
# 1) 先同步版本元数据
# 2) 并行运行 Web 链（typecheck -> lint -> sizecheck -> test）
# 3) 并行运行 Rust 链（clippy -> test）
ci: sync-version
    node scripts/run-ci.mjs

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
