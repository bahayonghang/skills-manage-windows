# Role-aware typography migration script.
# Migrates arbitrary text-[...] sizes to semantic tokens + removes alpha foregrounds.
# Rules per design.md §2-3 and prd.md R2-R3.

$ErrorActionPreference = 'Stop'
$root = Resolve-Path 'src'

function Get-ProducionFiles {
  Get-ChildItem -Path $root -Recurse -Include *.ts,*.tsx -File |
    Where-Object { $_.FullName -notlike "*\test\*" -and $_.FullName -notlike "*\test\*" }
}

function Convert-Line([string]$line) {
  $out = $line

  # --- Unambiguous display / control values ---
  $out = $out -replace 'text-\[3\.35rem\]', 'text-display-score'
  $out = $out -replace 'text-\[3\.25rem\]', 'text-display-hero-xl'
  $out = $out -replace 'text-\[2\.35rem\]', 'text-display-hero'
  $out = $out -replace 'text-\[3rem\]', 'text-5xl'
  $out = $out -replace 'text-\[1\.05rem\]', 'text-base'
  $out = $out -replace 'text-\[0\.95rem\]', 'text-sm'
  $out = $out -replace 'text-\[0\.8rem\]', 'text-xs'
  $out = $out -replace 'text-\[13px\]', 'text-xs'
  $out = $out -replace 'text-\[12px\]', 'text-ui-meta'

  # --- 0.7rem: compact button label (font-medium, no muted) -> text-xs; else meta ---
  if ($out -match 'text-\[0\.7rem\]') {
    if ($out -match 'font-medium' -and $out -notmatch 'text-muted-foreground') {
      $out = $out -replace 'text-\[0\.7rem\]', 'text-xs'
    } else {
      $out = $out -replace 'text-\[0\.7rem\]', 'text-ui-meta'
    }
  }

  # --- 0.68rem / 0.72rem / 11px: label (uppercase OR bold/semibold+colored, no mono) -> text-xs; else text-ui-meta ---
  $isLabel = ($out -match 'uppercase') -or
    ($out -match 'font-(?:bold|semibold)' -and
     $out -match 'text-(?:primary|warning|success|destructive|info|primary-text|warning-foreground|success-foreground|destructive-foreground)' -and
     $out -notmatch 'font-mono')
  if ($isLabel) {
    $out = $out -replace 'text-\[0\.68rem\]', 'text-xs'
    $out = $out -replace 'text-\[0\.72rem\]', 'text-xs'
    $out = $out -replace 'text-\[11px\]', 'text-xs'
  } else {
    $out = $out -replace 'text-\[0\.68rem\]', 'text-ui-meta'
    $out = $out -replace 'text-\[0\.72rem\]', 'text-ui-meta'
    $out = $out -replace 'text-\[11px\]', 'text-ui-meta'
  }

  # --- 10px: micro (counts, axes, decorative glyphs) ---
  $out = $out -replace 'text-\[10px\]', 'text-ui-micro'

  # --- Alpha foreground remediation (R3): remove alpha, use full tested tokens ---
  $out = $out -replace 'text-muted-foreground/(?:[5-8][0-9]|90)', 'text-muted-foreground'
  $out = $out -replace 'text-foreground/(?:[5-8][0-9]|90)', 'text-foreground'
  # primary/85 on text was an alpha-prefixed accent text; use readable primary-text token
  $out = $out -replace 'text-primary/85', 'text-primary-text'
  $out = $out -replace 'text-primary/(?:[5-8][0-9]|90)\b', 'text-primary-text'

  return $out
}

$changed = 0
$files = Get-ProducionFiles
foreach ($file in $files) {
  $orig = Get-Content $file.FullName -Raw -Encoding UTF8
  $lines = $orig -split "(`r?`n)"
  $newLines = $lines | ForEach-Object { Convert-Line $_ }
  $new = $newLines -join ""
  if ($new -ne $orig) {
    [System.IO.File]::WriteAllText($file.FullName, $new, (New-Object System.Text.UTF8Encoding $false))
    $changed++
  }
}
"migrated $changed files"
