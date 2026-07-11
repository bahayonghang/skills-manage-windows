param(
    [string]$Distribution = "Ubuntu-24.04",
    [int]$Iterations = 25
)

$ErrorActionPreference = "Stop"

function Measure-ProcessStartup {
    param(
        [string]$Name,
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    $samples = foreach ($index in 1..$Iterations) {
        $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = $FilePath
        $startInfo.Arguments = $ArgumentList -join " "
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true

        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $process = [System.Diagnostics.Process]::Start($startInfo)
        $process.WaitForExit()
        $timer.Stop()
        if ($process.ExitCode -ne 0) {
            throw "$Name failed with exit code $($process.ExitCode)"
        }
        $timer.Elapsed.TotalMilliseconds
    }

    $ordered = @($samples | Sort-Object)
    $p50Index = [Math]::Floor(($ordered.Count - 1) * 0.50)
    $p95Index = [Math]::Floor(($ordered.Count - 1) * 0.95)

    [pscustomobject]@{
        name = $Name
        iterations = $ordered.Count
        minMs = [Math]::Round($ordered[0], 2)
        p50Ms = [Math]::Round($ordered[$p50Index], 2)
        p95Ms = [Math]::Round($ordered[$p95Index], 2)
        maxMs = [Math]::Round($ordered[-1], 2)
        meanMs = [Math]::Round(($ordered | Measure-Object -Average).Average, 2)
    }
}

# Warm the selected distribution so the measurements capture per-command cost,
# not WSL VM cold boot time.
$warmupInfo = [System.Diagnostics.ProcessStartInfo]::new()
$warmupInfo.FileName = "$env:WINDIR\System32\wsl.exe"
$warmupInfo.Arguments = "-d $Distribution -- true"
$warmupInfo.UseShellExecute = $false
$warmupInfo.CreateNoWindow = $true
$warmup = [System.Diagnostics.Process]::Start($warmupInfo)
$warmup.WaitForExit()
if ($warmup.ExitCode -ne 0) {
    throw "WSL warmup failed with exit code $($warmup.ExitCode)"
}

$results = @(
    Measure-ProcessStartup `
        -Name "windows-cmd" `
        -FilePath "$env:WINDIR\System32\cmd.exe" `
        -ArgumentList @("/d", "/c", "exit", "0")
    Measure-ProcessStartup `
        -Name "wsl-direct" `
        -FilePath "$env:WINDIR\System32\wsl.exe" `
        -ArgumentList @("-d", $Distribution, "--", "true")
    Measure-ProcessStartup `
        -Name "wsl-shell" `
        -FilePath "$env:WINDIR\System32\wsl.exe" `
        -ArgumentList @("-d", $Distribution, "--", "sh", "-lc", "true")
    Measure-ProcessStartup `
        -Name "wsl-exec-direct" `
        -FilePath "$env:WINDIR\System32\wsl.exe" `
        -ArgumentList @("-d", $Distribution, "--exec", "true")
    Measure-ProcessStartup `
        -Name "wsl-exec-shell" `
        -FilePath "$env:WINDIR\System32\wsl.exe" `
        -ArgumentList @("-d", $Distribution, "--exec", "sh", "-lc", "true")
)

$results | ConvertTo-Json
