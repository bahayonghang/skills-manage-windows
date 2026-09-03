<#
.SYNOPSIS
  Bounded Windows installer smoke for SkillPort NSIS and MSI cases.

.DESCRIPTION
  Owns one installer case: install, unique installed skillport.exe checks,
  launch, stop, uninstall, and residue cleanup. Every waited process has a
  numeric deadline, an exit code, and process-tree termination on timeout.
  Cleanup always runs in finally; cleanup failure is its own failure.

  Logs only redacted stage records:
  {stage, outcome, exitCode?, timedOut, cleanupOutcome}
  plus optional artifact digest and install root. Never dumps the environment.

.PARAMETER Fixture
  timeout - start a harmless hang (Start-Sleep) and prove timeout, tree kill,
  and cleanup without installing SkillPort. Exits non-zero when the deadline
  fires as intended.

.PARAMETER InstallerKind
  nsis or msi. Distinct native install/uninstall arguments; do not share
  mutable install state across cases.

.PARAMETER ArtifactPath
  Final signed installer path for this case.

.PARAMETER ExpectedVersion
  Release version from release-context (for example 1.0.2).

.PARAMETER InstallRoot
  Unique case install directory, typically under $RUNNER_TEMP.

.PARAMETER SigningStatePath
  windows-signing.json produced by windows-release-signing. Authenticode
  policy matches validateSigningState: valid requires Valid + signer +
  timestamp; not-configured requires NotSigned and must not be reported as
  signed. Does not claim the inner exe was signed before bundle.

.PARAMETER TimeoutMs
  Per-process deadline in milliseconds. Default 120000.

.EXAMPLE
  pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -Fixture timeout

.EXAMPLE
  pwsh -NoProfile -File scripts/release/windows-installer-smoke.ps1 -InstallerKind nsis -ArtifactPath <final-nsis-path> -ExpectedVersion <release-version> -InstallRoot <unique-temp-root> -SigningStatePath windows-signing.json
#>
[CmdletBinding()]
param(
  [ValidateSet("timeout")]
  [string]$Fixture,

  [ValidateSet("nsis", "msi")]
  [string]$InstallerKind,

  [string]$ArtifactPath,

  [string]$ExpectedVersion,

  [string]$InstallRoot,

  [string]$SigningStatePath,

  [int]$TimeoutMs = 120000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$script:ProcessTreeReapMs = 5000
$script:LaunchSettleMs = 5000
$script:FixtureHangTimeoutMs = 3000
$script:MsiSuccessExitCodes = @(0, 3010)

function Write-StageRecord {
  param(
    [Parameter(Mandatory)][string]$Stage,
    [Parameter(Mandatory)][string]$Outcome,
    [bool]$TimedOut,
    [Parameter(Mandatory)][string]$CleanupOutcome,
    $ExitCode,
    [string]$Digest,
    [string]$InstallRootPath
  )
  $record = [ordered]@{
    stage = $Stage
    outcome = $Outcome
    timedOut = [bool]$TimedOut
    cleanupOutcome = $CleanupOutcome
  }
  if ($PSBoundParameters.ContainsKey("ExitCode") -and $null -ne $ExitCode) {
    $record.exitCode = [int]$ExitCode
  }
  if (-not [string]::IsNullOrWhiteSpace($Digest)) {
    $record.digest = $Digest
  }
  if (-not [string]::IsNullOrWhiteSpace($InstallRootPath)) {
    $record.installRoot = $InstallRootPath
  }
  Write-Output ($record | ConvertTo-Json -Compress)
}

function Get-DescendantProcessIds {
  param([Parameter(Mandatory)][int]$ProcessId)
  $ids = [System.Collections.Generic.List[int]]::new()
  $ids.Add($ProcessId)
  $children = @(Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $ProcessId" -ErrorAction SilentlyContinue)
  foreach ($child in $children) {
    foreach ($descendant in @(Get-DescendantProcessIds -ProcessId ([int]$child.ProcessId))) {
      $ids.Add($descendant)
    }
  }
  return @($ids | Select-Object -Unique)
}

function Stop-ProcessTree {
  param(
    [Parameter(Mandatory)][int]$ProcessId,
    [int]$ReapMs = $script:ProcessTreeReapMs
  )
  $ids = @(Get-DescendantProcessIds -ProcessId $ProcessId)
  [array]::Reverse($ids)
  foreach ($id in $ids) {
    Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
  }
  $deadline = [datetime]::UtcNow.AddMilliseconds($ReapMs)
  do {
    $alive = @($ids | Where-Object { $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue) })
    if ($alive.Count -eq 0) {
      return $true
    }
    Start-Sleep -Milliseconds 100
  } while ([datetime]::UtcNow -lt $deadline)
  $alive = @($ids | Where-Object { $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue) })
  return ($alive.Count -eq 0)
}

function Invoke-BoundedProcess {
  param(
    [Parameter(Mandatory)][string]$FilePath,
    [string[]]$ArgumentList = @(),
    [Parameter(Mandatory)][int]$DeadlineMs,
    [Parameter(Mandatory)][string]$Stage,
    [int[]]$SuccessExitCodes = @(0)
  )
  $startParams = @{
    FilePath = $FilePath
    PassThru = $true
    WindowStyle = "Hidden"
  }
  if ($ArgumentList.Count -gt 0) {
    $startParams.ArgumentList = $ArgumentList
  }
  $process = Start-Process @startParams
  if ($null -eq $process) {
    return @{
      stage = $Stage
      outcome = "failed"
      timedOut = $false
      cleanupOutcome = "not-required"
    }
  }
  $exited = $process.WaitForExit($DeadlineMs)
  if (-not $exited) {
    $cleaned = Stop-ProcessTree -ProcessId $process.Id
    return @{
      stage = $Stage
      outcome = "timeout"
      timedOut = $true
      cleanupOutcome = $(if ($cleaned) { "ok" } else { "failed" })
    }
  }
  $code = [int]$process.ExitCode
  return @{
    stage = $Stage
    outcome = $(if ($SuccessExitCodes -contains $code) { "ok" } else { "failed" })
    exitCode = $code
    timedOut = $false
    cleanupOutcome = "not-required"
  }
}

function Invoke-MsiQuery {
  param(
    $Database,
    [Parameter(Mandatory)][string]$Query
  )
  $view = $Database.GetType().InvokeMember("OpenView", "InvokeMethod", $null, $Database, @($Query))
  [void]$view.GetType().InvokeMember("Execute", "InvokeMethod", $null, $view, $null)
  $rows = [System.Collections.Generic.List[string[]]]::new()
  while ($true) {
    $record = $view.GetType().InvokeMember("Fetch", "InvokeMethod", $null, $view, $null)
    if ($null -eq $record) {
      break
    }
    $columnCount = [int]$record.GetType().InvokeMember("FieldCount", "GetProperty", $null, $record, $null)
    $row = [string[]]::new($columnCount)
    for ($index = 1; $index -le $columnCount; $index += 1) {
      $row[$index - 1] = [string]$record.GetType().InvokeMember("StringData", "GetProperty", $null, $record, $index)
    }
    $rows.Add($row)
  }
  return $rows.ToArray()
}

function Get-MsiMetadata {
  param([Parameter(Mandatory)][string]$Path)
  $resolved = (Resolve-Path -LiteralPath $Path).Path
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $null
  try {
    $database = $installer.GetType().InvokeMember("OpenDatabase", "InvokeMethod", $null, $installer, @($resolved, 0))
    $productRows = @(Invoke-MsiQuery -Database $database -Query "SELECT Value FROM Property WHERE Property = 'ProductCode'")
    if ($productRows.Count -ne 1 -or [string]::IsNullOrWhiteSpace($productRows[0][0])) {
      throw "MSI ProductCode is missing or ambiguous."
    }
    $installDirRows = @(Invoke-MsiQuery -Database $database -Query "SELECT Directory FROM Directory WHERE Directory = 'INSTALLDIR'")
    $applicationFolderRows = @(Invoke-MsiQuery -Database $database -Query "SELECT Directory FROM Directory WHERE Directory = 'APPLICATIONFOLDER'")
    $directoryProperties = @()
    if ($installDirRows.Count -eq 1) {
      $directoryProperties += "INSTALLDIR"
    }
    if ($applicationFolderRows.Count -eq 1) {
      $directoryProperties += "APPLICATIONFOLDER"
    }
    if ($directoryProperties.Count -ne 1) {
      throw "MSI install-directory property is missing or ambiguous."
    }
    return [pscustomobject]@{
      ProductCode = $productRows[0][0]
      InstallDirProperty = $directoryProperties[0]
    }
  }
  finally {
    if ($null -ne $database) {
      [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($database)
    }
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($installer)
  }
}

function Resolve-InstalledSkillPort {
  param([Parameter(Mandatory)][string]$CaseInstallRoot)
  if (-not (Test-Path -LiteralPath $CaseInstallRoot)) {
    throw "Install root does not exist."
  }
  $matches = @(Get-ChildItem -LiteralPath $CaseInstallRoot -Recurse -File -Filter "skillport.exe" -ErrorAction SilentlyContinue)
  if ($matches.Count -ne 1) {
    throw "Expected exactly one skillport.exe under the case install root, found $($matches.Count)."
  }
  return $matches[0]
}

function Test-FileVersionMatches {
  param(
    [Parameter(Mandatory)]$VersionInfo,
    [Parameter(Mandatory)][string]$Expected
  )
  $candidates = @($VersionInfo.ProductVersion, $VersionInfo.FileVersion) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  foreach ($candidate in $candidates) {
    $value = [string]$candidate
    if ($value -eq $Expected -or $value.StartsWith("$Expected.") -or $value.StartsWith("$Expected-")) {
      return $true
    }
  }
  return $false
}

function Read-SigningState {
  param([Parameter(Mandatory)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    throw "Windows signing state not found."
  }
  $state = Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json
  if ($null -eq $state -or $state.PSObject.Properties["authenticode"] -eq $null) {
    throw "Windows signing state must be a JSON object."
  }
  $authenticode = [string]$state.authenticode
  if ($authenticode -notin @("valid", "not-configured", "invalid")) {
    throw "Unknown Authenticode state: $authenticode."
  }
  if ($authenticode -eq "invalid") {
    throw "Authenticode validation failed for installed executable."
  }
  return $state
}

function Assert-InstalledExecutable {
  param(
    [Parameter(Mandatory)]$Executable,
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)]$SigningState
  )
  $signature = Get-AuthenticodeSignature -LiteralPath $Executable.FullName
  $status = $signature.Status.ToString()
  $authenticode = [string]$SigningState.authenticode
  if ($authenticode -eq "valid") {
    if ($status -ne "Valid") {
      throw "Installed skillport.exe Authenticode status must be Valid."
    }
    if ($null -eq $signature.SignerCertificate -or [string]::IsNullOrWhiteSpace($signature.SignerCertificate.Subject)) {
      throw "Installed skillport.exe is missing an Authenticode signer."
    }
    if ($null -eq $signature.TimeStamperCertificate) {
      throw "Installed skillport.exe is missing an Authenticode timestamp."
    }
    $expectedSigner = $null
    if ($null -ne $SigningState.files -and $null -ne $SigningState.files."skillport.exe") {
      $expectedSigner = [string]$SigningState.files."skillport.exe".signer
    }
    if (-not [string]::IsNullOrWhiteSpace($expectedSigner) -and $signature.SignerCertificate.Subject -ne $expectedSigner) {
      throw "Installed skillport.exe signer does not match signing evidence."
    }
  }
  elseif ($authenticode -eq "not-configured") {
    if ($status -ne "NotSigned") {
      throw "Installed skillport.exe must be NotSigned when Authenticode is not-configured."
    }
  }
  else {
    throw "Authenticode validation failed for installed executable."
  }
  Write-StageRecord -Stage "signature" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required"

  $versionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Executable.FullName)
  if (-not (Test-FileVersionMatches -VersionInfo $versionInfo -Expected $Version)) {
    throw "Installed skillport.exe version does not match the release version."
  }
  Write-StageRecord -Stage "version" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required"
}

function Stop-LaunchedApplication {
  param($Process)
  if ($null -eq $Process) {
    Write-StageRecord -Stage "stop" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required"
    return $true
  }
  if ($Process.HasExited) {
    Write-StageRecord -Stage "stop" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required" -ExitCode $Process.ExitCode
    return $true
  }
  $cleaned = Stop-ProcessTree -ProcessId $Process.Id
  Write-StageRecord -Stage "stop" -Outcome $(if ($cleaned) { "ok" } else { "failed" }) -TimedOut $false -CleanupOutcome $(if ($cleaned) { "ok" } else { "failed" })
  return $cleaned
}

function Invoke-TimeoutFixture {
  $shell = (Get-Command pwsh -ErrorAction Stop).Source
  $result = Invoke-BoundedProcess -FilePath $shell -ArgumentList @("-NoProfile", "-Command", "Start-Sleep -Seconds 60") -DeadlineMs $script:FixtureHangTimeoutMs -Stage "fixture-timeout"
  $exitArgs = @{}
  if ($result.ContainsKey("exitCode")) {
    $exitArgs.ExitCode = $result.exitCode
  }
  Write-StageRecord -Stage $result.stage -Outcome $result.outcome -TimedOut $result.timedOut -CleanupOutcome $result.cleanupOutcome @exitArgs
  if (-not $result.timedOut) {
    throw "Timeout fixture did not observe a deadline."
  }
  if ($result.cleanupOutcome -ne "ok") {
    throw "Timeout fixture failed to kill the process tree."
  }
  exit 1
}

function Invoke-InstallerCase {
  if ([string]::IsNullOrWhiteSpace($ArtifactPath) -or [string]::IsNullOrWhiteSpace($ExpectedVersion) -or [string]::IsNullOrWhiteSpace($InstallRoot) -or [string]::IsNullOrWhiteSpace($SigningStatePath)) {
    throw "InstallerKind requires ArtifactPath, ExpectedVersion, InstallRoot, and SigningStatePath."
  }
  if (-not (Test-Path -LiteralPath $ArtifactPath)) {
    throw "Installer artifact was not found."
  }

  $signingState = Read-SigningState -Path $SigningStatePath
  $digest = (Get-FileHash -LiteralPath $ArtifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
  $appProcess = $null
  $productCode = $null
  $primaryError = $null
  $cleanupFailed = $false

  Write-StageRecord -Stage "resolve" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required" -Digest $digest -InstallRootPath $InstallRoot

  try {
    New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    $installResult = $null
    if ($InstallerKind -eq "nsis") {
      $installResult = Invoke-BoundedProcess -FilePath $ArtifactPath -ArgumentList @("/S", "/D=$InstallRoot") -DeadlineMs $TimeoutMs -Stage "install"
    }
    else {
      $msi = Get-MsiMetadata -Path $ArtifactPath
      $productCode = $msi.ProductCode
      $installResult = Invoke-BoundedProcess -FilePath "msiexec.exe" -ArgumentList @("/i", $ArtifactPath, "/qn", "/norestart", "$($msi.InstallDirProperty)=$InstallRoot") -DeadlineMs $TimeoutMs -Stage "install" -SuccessExitCodes $script:MsiSuccessExitCodes
    }
    $installExit = @{}
    if ($installResult.ContainsKey("exitCode")) {
      $installExit.ExitCode = $installResult.exitCode
    }
    Write-StageRecord -Stage $installResult.stage -Outcome $installResult.outcome -TimedOut $installResult.timedOut -CleanupOutcome $installResult.cleanupOutcome -InstallRootPath $InstallRoot @installExit
    if ($installResult.timedOut -or $installResult.outcome -ne "ok") {
      throw "Installer did not complete successfully."
    }

    $executable = Resolve-InstalledSkillPort -CaseInstallRoot $InstallRoot
    Assert-InstalledExecutable -Executable $executable -Version $ExpectedVersion -SigningState $signingState

    $appProcess = Start-Process -FilePath $executable.FullName -WorkingDirectory $executable.DirectoryName -PassThru -WindowStyle Hidden
    if ($null -eq $appProcess) {
      throw "Failed to launch installed skillport.exe."
    }
    $launchDeadline = [datetime]::UtcNow.AddMilliseconds($script:LaunchSettleMs)
    while ([datetime]::UtcNow -lt $launchDeadline) {
      if ($appProcess.HasExited) {
        throw "Installed SkillPort exited before the smoke check."
      }
      Start-Sleep -Milliseconds 200
    }
    Write-StageRecord -Stage "launch" -Outcome "ok" -TimedOut $false -CleanupOutcome "not-required"
  }
  catch {
    $primaryError = $_
    Write-StageRecord -Stage "case" -Outcome "failed" -TimedOut $false -CleanupOutcome "pending"
  }
  finally {
    try {
      if (-not (Stop-LaunchedApplication -Process $appProcess)) {
        $cleanupFailed = $true
      }

      if ($InstallerKind -eq "nsis") {
        $uninstaller = $null
        if (Test-Path -LiteralPath $InstallRoot) {
          $uninstaller = Get-ChildItem -LiteralPath $InstallRoot -Recurse -File -Filter "uninstall.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        }
        if ($null -ne $uninstaller) {
          $uninstallResult = Invoke-BoundedProcess -FilePath $uninstaller.FullName -ArgumentList @("/S") -DeadlineMs $TimeoutMs -Stage "uninstall"
          $uninstallExit = @{}
          if ($uninstallResult.ContainsKey("exitCode")) {
            $uninstallExit.ExitCode = $uninstallResult.exitCode
          }
          Write-StageRecord -Stage $uninstallResult.stage -Outcome $uninstallResult.outcome -TimedOut $uninstallResult.timedOut -CleanupOutcome $uninstallResult.cleanupOutcome @uninstallExit
          if ($uninstallResult.timedOut -or $uninstallResult.outcome -ne "ok") {
            $cleanupFailed = $true
          }
        }
      }
      elseif (-not [string]::IsNullOrWhiteSpace($productCode)) {
        $uninstallResult = Invoke-BoundedProcess -FilePath "msiexec.exe" -ArgumentList @("/x", $productCode, "/qn", "/norestart") -DeadlineMs $TimeoutMs -Stage "uninstall" -SuccessExitCodes $script:MsiSuccessExitCodes
        $uninstallExit = @{}
        if ($uninstallResult.ContainsKey("exitCode")) {
          $uninstallExit.ExitCode = $uninstallResult.exitCode
        }
        Write-StageRecord -Stage $uninstallResult.stage -Outcome $uninstallResult.outcome -TimedOut $uninstallResult.timedOut -CleanupOutcome $uninstallResult.cleanupOutcome @uninstallExit
        if ($uninstallResult.timedOut -or $uninstallResult.outcome -ne "ok") {
          $cleanupFailed = $true
        }
      }

      $residue = @()
      if (Test-Path -LiteralPath $InstallRoot) {
        $residue = @(Get-ChildItem -LiteralPath $InstallRoot -Recurse -File -Filter "skillport.exe" -ErrorAction SilentlyContinue)
      }
      if ($residue.Count -gt 0) {
        $cleanupFailed = $true
        Write-StageRecord -Stage "cleanup" -Outcome "failed" -TimedOut $false -CleanupOutcome "failed" -InstallRootPath $InstallRoot
      }
      else {
        $cleanupOutcome = $(if ($cleanupFailed) { "failed" } else { "ok" })
        Write-StageRecord -Stage "cleanup" -Outcome $cleanupOutcome -TimedOut $false -CleanupOutcome $cleanupOutcome -InstallRootPath $InstallRoot
      }
    }
    catch {
      $cleanupFailed = $true
      Write-StageRecord -Stage "cleanup" -Outcome "failed" -TimedOut $false -CleanupOutcome "failed" -InstallRootPath $InstallRoot
    }
  }

  if ($cleanupFailed) {
    throw "Installer case cleanup failed."
  }
  if ($null -ne $primaryError) {
    throw $primaryError
  }
}

if ($Fixture -eq "timeout") {
  Invoke-TimeoutFixture
}
elseif (-not [string]::IsNullOrWhiteSpace($InstallerKind)) {
  Invoke-InstallerCase
}
else {
  throw "Specify -Fixture timeout or -InstallerKind nsis|msi."
}
