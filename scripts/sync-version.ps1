$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$packageJsonPath = Join-Path $repoRoot "package.json"
$cargoTomlPath = Join-Path $repoRoot "src-tauri/Cargo.toml"
$cargoLockPath = Join-Path $repoRoot "src-tauri/Cargo.lock"
$tauriConfigPath = Join-Path $repoRoot "src-tauri/tauri.conf.json"

$packageJson = Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json
$version = [string]$packageJson.version

if ([string]::IsNullOrWhiteSpace($version)) {
    throw "package.json is missing a valid version."
}

function Replace-FirstOrFail {
    param(
        [string]$Content,
        [string]$Pattern,
        [string]$Replacement,
        [string]$ErrorMessage
    )

    $regex = New-Object System.Text.RegularExpressions.Regex($Pattern)

    if (-not $regex.IsMatch($Content)) {
        throw $ErrorMessage
    }

    return $regex.Replace($Content, $Replacement, 1)
}

$tauriConfigContent = Get-Content -LiteralPath $tauriConfigPath -Raw
$tauriConfigUpdated = Replace-FirstOrFail `
    -Content $tauriConfigContent `
    -Pattern '("version"\s*:\s*")[^"]+(")' `
    -Replacement ('${1}' + $version + '${2}') `
    -ErrorMessage "tauri.conf.json version was not found."
Set-Content -LiteralPath $tauriConfigPath -Value $tauriConfigUpdated -NoNewline

$cargoTomlContent = Get-Content -LiteralPath $cargoTomlPath -Raw
$cargoTomlUpdated = Replace-FirstOrFail `
    -Content $cargoTomlContent `
    -Pattern '(?ms)(\[package\]\s*name\s*=\s*"skills-manage"\s*version\s*=\s*")[^"]+(")' `
    -Replacement ('${1}' + $version + '${2}') `
    -ErrorMessage "Cargo.toml package version was not found."
Set-Content -LiteralPath $cargoTomlPath -Value $cargoTomlUpdated -NoNewline

$cargoLockContent = Get-Content -LiteralPath $cargoLockPath -Raw
$cargoLockUpdated = Replace-FirstOrFail `
    -Content $cargoLockContent `
    -Pattern '(?ms)(\[\[package\]\]\s*name\s*=\s*"skills-manage"\s*version\s*=\s*")[^"]+(")' `
    -Replacement ('${1}' + $version + '${2}') `
    -ErrorMessage "Cargo.lock package version was not found."
Set-Content -LiteralPath $cargoLockPath -Value $cargoLockUpdated -NoNewline

Write-Host ("[version] synced to " + $version)
