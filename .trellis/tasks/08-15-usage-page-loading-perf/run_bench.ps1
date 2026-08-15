param(
  [string]$Bin,
  [string]$OutFile,
  [string]$ErrFile,
  [string]$Filter = "usage_bench"
)
$args = @($Filter, "--ignored", "--nocapture", "--test-threads=1")
$p = Start-Process -FilePath $Bin -ArgumentList $args -NoNewWindow -PassThru `
  -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile
$max = 0
while (-not $p.HasExited) {
  $p.Refresh()
  if ($p.PeakWorkingSet64 -gt $max) { $max = $p.PeakWorkingSet64 }
  Start-Sleep -Milliseconds 250
}
$p.WaitForExit()
$p.Refresh()
if ($p.PeakWorkingSet64 -gt $max) { $max = $p.PeakWorkingSet64 }
Write-Output ("EXIT_CODE={0}" -f $p.ExitCode)
Write-Output ("PEAK_WS_BYTES={0}" -f $max)
