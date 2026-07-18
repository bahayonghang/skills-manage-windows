# Pass 2: promote status/badge labels from text-ui-meta to text-xs.
# Per R2: section/status/action label at least text-xs. Badges (rounded-full,
# ring-1, border px- py-0.5) and elements with semantic colored foreground are
# labels, not secondary metadata. Secondary metadata (truncate descriptions,
# plain muted-foreground spans without badge shape) stay text-ui-meta.

$ErrorActionPreference = 'Stop'
$root = Resolve-Path 'src'

function Get-ProducionFiles {
  Get-ChildItem -Path $root -Recurse -Include *.ts,*.tsx -File |
    Where-Object { $_.FullName -notlike "*\test\*" }
}

function Convert-Line([string]$line) {
  if ($line -notmatch 'text-ui-meta') { return $line }

  # Badge shape: rounded-full or rounded-md with px- and py-0.5, or ring-1,
  # or border with px- py-0.5 -> these are status/origin/label badges.
  $isBadge = ($line -match 'rounded-full') -or
    ($line -match 'ring-1') -or
    ($line -match 'rounded-(?:md|full|lg)\b' -and $line -match 'px-' -and $line -match 'py-0\.5')

  # Semantic colored foreground (status label).
  $isColoredStatus = $line -match 'text-(?:primary\b|primary-text|warning-foreground|success-foreground|info-foreground|destructive\b|destructive-foreground)'

  # Section header shape: sticky top-0, or facet/filter group label with
  # tracking-normal/tracking-wide + font-medium + text-muted-foreground and
  # NOT a truncate description.
  $isSectionHeader = ($line -match 'sticky top-0') -or
    (($line -match 'tracking-(?:normal|wide|wider)') -and
     ($line -match 'px-2 pt-2' -or $line -match 'shrink-0 text-ui-meta font-medium text-muted-foreground'))

  if ($isBadge -or $isColoredStatus -or $isSectionHeader) {
    return ($line -replace 'text-ui-meta', 'text-xs')
  }
  return $line
}

$changed = 0
foreach ($file in Get-ProducionFiles) {
  $orig = Get-Content $file.FullName -Raw -Encoding UTF8
  $lines = $orig -split "(`r?`n)"
  $new = ($lines | ForEach-Object { Convert-Line $_ }) -join ""
  if ($new -ne $orig) {
    [System.IO.File]::WriteAllText($file.FullName, $new, (New-Object System.Text.UTF8Encoding $false))
    $changed++
  }
}
"pass2 migrated $changed files"
