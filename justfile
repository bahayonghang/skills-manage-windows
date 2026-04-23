set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

output_dir := "outputs"
nsis_bundle_dir := "src-tauri/target/release/bundle/nsis"
app_exe := "skills-manage.exe"

# ========================================================================
# 步骤1：检查工作流
# ========================================================================
# 目标：
# 1) 运行前端类型检查、ESLint
# 2) 运行 Rust 单元测试与 clippy 静态检查
ci:
    @$ErrorActionPreference = 'Stop'; function Run-Step { param([scriptblock]$Step) & $Step; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }; Write-Host '[ci] 开始前端检查'; Run-Step { pnpm typecheck }; Run-Step { pnpm exec eslint src --ext .ts,.tsx --report-unused-disable-directives }; Write-Host '[ci] 开始 Rust 检查'; Run-Step { cargo test --manifest-path src-tauri/Cargo.toml }; Run-Step { cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings }; Write-Host '[ci] 检查完成'

# ========================================================================
# 步骤2：构建桌面应用
# ========================================================================
# 目标：
# 1) 构建 Tauri Windows 应用
# 2) 把 NSIS 安装包复制到项目根目录 outputs/
build:
    @$ErrorActionPreference = 'Stop'; function Run-Step { param([scriptblock]$Step) & $Step; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }; $outputDir = '{{output_dir}}'; $bundleDir = '{{nsis_bundle_dir}}'; $legacyExe = Join-Path $outputDir '{{app_exe}}'; Write-Host '[build] 准备输出目录'; New-Item -ItemType Directory -Path $outputDir -Force | Out-Null; Write-Host '[build] 开始 Tauri 构建'; Run-Step { pnpm tauri build }; $installer = Get-ChildItem -Path $bundleDir -Filter '*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1; if ($null -eq $installer) { throw ('未找到 NSIS 安装包: ' + $bundleDir) }; if (Test-Path $legacyExe) { Remove-Item -LiteralPath $legacyExe -Force }; $outputInstaller = Join-Path $outputDir $installer.Name; Copy-Item -LiteralPath $installer.FullName -Destination $outputInstaller -Force; Write-Host ('[build] 已复制安装包到 ' + $outputInstaller)

# ========================================================================
# 步骤3：启动开发模式
# ========================================================================
# 目标：
# 1) 启动 Tauri 开发环境
# 2) 直接运行桌面应用
dev:
    @$ErrorActionPreference = 'Stop'; Write-Host '[dev] 启动 Tauri 开发模式'; pnpm tauri dev; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
