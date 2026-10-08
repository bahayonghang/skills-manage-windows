param(
    [ValidateRange(1, 7)][int]$Pairs = 7,
    [string]$Before = '.git/skill-small-files-tools/benchmark-before-targeted.exe',
    [string]$After = '.git/skill-small-files-tools/benchmark-after.exe'
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../..')).Path
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$beforePath = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot $Before)).Path
$afterPath = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot $After)).Path
$harnessPath = Join-Path $repositoryRoot 'src-tauri/src/services/central_updates/core/performance_benchmark.rs'
$harnessHash = (Get-FileHash -LiteralPath $harnessPath -Algorithm SHA256).Hash
$expectedHarnessHash = '203949B25783EA044256268BF4D48CBD6967386A774D78A8515477A64970B923'
if ($harnessHash -ne $expectedHarnessHash) { throw 'Frozen harness identity changed' }
$binaryPaths = @{ before = $beforePath; after = $afterPath }
$binaryHashes = @{
    before = (Get-FileHash -LiteralPath $beforePath -Algorithm SHA256).Hash
    after = (Get-FileHash -LiteralPath $afterPath -Algorithm SHA256).Hash
}
if ($binaryHashes.before -ne '46A19EC66B300DF79D7345724DB5CA2440160AC29A23202655965A41088AD36F' -or
    $binaryHashes.after -ne '41293FD91D399D680488B43AB1CCE547204776864FDDE309C32F1C07D76A3411') {
    throw 'Frozen binary identity changed'
}

function Read-ScannerCpu {
    try {
        $scanner = @(Get-Process -Name MsMpEng -ErrorAction SilentlyContinue)
        if ($scanner.Count -ne 1 -or $null -eq $scanner[0].CPU) {
            return @{ status = 'UNMEASURED'; reason = 'Single readable MsMpEng CPU value unavailable' }
        }
        return @{ status = 'MEASURED'; pid = $scanner[0].Id; cpu_seconds = $scanner[0].CPU }
    }
    catch {
        return @{ status = 'UNMEASURED'; reason = $_.Exception.GetType().Name }
    }
}

function Read-BenchmarkCpu($Process) {
    try {
        return @{ status = 'MEASURED'; pid = $Process.Id; cpu_seconds = $Process.TotalProcessorTime.TotalSeconds }
    }
    catch {
        return @{ status = 'UNMEASURED'; reason = $_.Exception.GetType().Name }
    }
}

$priorFiles = $env:SKILLPORT_BENCH_FILES
$priorSamples = $env:SKILLPORT_BENCH_SAMPLES
$priorMode = $env:SKILLPORT_BENCH_MODE
$runs = [System.Collections.Generic.List[object]]::new()
try {
    $env:SKILLPORT_BENCH_FILES = '13016'
    $env:SKILLPORT_BENCH_SAMPLES = '1'
    $env:SKILLPORT_BENCH_MODE = 'stage'
    for ($pair = 0; $pair -lt $Pairs; $pair++) {
        $order = if ($pair % 2 -eq 0) { @('before', 'after') } else { @('after', 'before') }
        foreach ($version in $order) {
            $label = "alternating-t-pair$pair-$version-$timestamp"
            $outputPath = Join-Path $PSScriptRoot "$label.log"
            $errorPath = Join-Path $PSScriptRoot "$label.stderr.log"
            $started = (Get-Date).ToUniversalTime().ToString('o')
            $scannerStart = Read-ScannerCpu
            $process = Start-Process -FilePath $binaryPaths[$version] -ArgumentList @(
                'small_files_service_baseline', '--ignored', '--nocapture', '--test-threads=1'
            ) -WorkingDirectory $repositoryRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput $outputPath -RedirectStandardError $errorPath
            try {
                $benchmarkStart = Read-BenchmarkCpu $process
                $process.WaitForExit()
                $benchmarkEnd = Read-BenchmarkCpu $process
                $scannerEnd = Read-ScannerCpu
                $exitCode = $process.ExitCode
            }
            finally {
                $process.Dispose()
            }
            $run = [ordered]@{
                pair_index = $pair
                pair_order = $order -join '-'
                version = $version
                started_utc = $started
                ended_utc = (Get-Date).ToUniversalTime().ToString('o')
                binary = $binaryPaths[$version]
                binary_sha256 = $binaryHashes[$version]
                harness_sha256 = $harnessHash
                profile = 'release'
                files = 13016
                samples = 1
                mode = 'stage'
                raw_log = $outputPath
                stderr_log = $errorPath
                raw_log_sha256 = (Get-FileHash -LiteralPath $outputPath -Algorithm SHA256).Hash
                exit_code = $exitCode
                benchmark_cpu_start = $benchmarkStart
                benchmark_cpu_end = $benchmarkEnd
                scanner_cpu_start = $scannerStart
                scanner_cpu_end = $scannerEnd
                cpu_note = 'Read-only cumulative process CPU endpoints. Scanner CPU is system-wide and does not identify fixture scanning. No worker IO, cache, ETW or peak resource measurement.'
            }
            $run | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath ([System.IO.Path]::ChangeExtension($outputPath, '.identity.json')) -Encoding utf8
            $runs.Add($run)
            $runs | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $PSScriptRoot "alternating-t-runs-$timestamp.json") -Encoding utf8
            Write-Output "Completed pair=$pair order=$($order -join '-') version=$version exit=$exitCode log=$outputPath"
            if ($exitCode -ne 0) { throw "Alternating benchmark failed: $label" }
        }
    }
}
finally {
    $env:SKILLPORT_BENCH_FILES = $priorFiles
    $env:SKILLPORT_BENCH_SAMPLES = $priorSamples
    $env:SKILLPORT_BENCH_MODE = $priorMode
}
