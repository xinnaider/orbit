#Requires -Version 5.1
# Orbit — Windows Installer
# Usage: irm https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-windows.ps1 | iex

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'
$repo = 'xinnaider/orbit'

# ── Helpers ──────────────────────────────────────────────────────────────────
function Write-Step    { param($m) Write-Host "  `u{25C6} $m" -ForegroundColor Green }
function Write-Info    { param($m) Write-Host "    $m" -ForegroundColor DarkGray }
function Write-Success { param($m) Write-Host "  `u{2713} $m" -ForegroundColor Green }
function Write-Fail    { param($m) Write-Host "  `u{2717} $m" -ForegroundColor Red; exit 1 }
function Write-Sep     { Write-Host "  $([string][char]0x2500 * 35)" -ForegroundColor DarkGray }

function Show-Progress {
    param([long]$bytes, [long]$total, [int]$pct)
    $filled = [math]::Min([int]($pct / 5), 20)
    $empty  = 20 - $filled
    $bar    = ([string]"`u{2588}" * $filled) + ([string]"`u{2591}" * $empty)
    $dlMB   = [math]::Round($bytes / 1MB, 1)
    $totMB  = [math]::Round($total / 1MB, 1)
    Write-Host "`r    [$bar] $pct%  $dlMB MB / $totMB MB  " -NoNewline -ForegroundColor Green
}

# ── Header ───────────────────────────────────────────────────────────────────
Clear-Host
Write-Host ''
Write-Host '  ██████╗ ██████╗ ██████╗ ██╗████████╗' -ForegroundColor Green
Write-Host ' ██╔═══██╗██╔══██╗██╔══██╗██║╚══██╔══╝' -ForegroundColor Green
Write-Host ' ██║   ██║██████╔╝██████╔╝██║   ██║   ' -ForegroundColor Green
Write-Host ' ██║   ██║██╔══██╗██╔══██╗██║   ██║   ' -ForegroundColor Green
Write-Host ' ╚██████╔╝██║  ██╗██████╔╝██║   ██║   ' -ForegroundColor Green
Write-Host '  ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚═╝   ╚═╝  ' -ForegroundColor Green
Write-Host ''
Write-Host '  Claude Code Agent Dashboard' -ForegroundColor White
Write-Host ''
Write-Sep
Write-Host ''

# ── Fetch release ─────────────────────────────────────────────────────────────
Write-Step 'Fetching latest release...'
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$asset   = $release.assets | Where-Object { $_.name -like '*-setup.exe' } | Select-Object -First 1

if (-not $asset) { Write-Fail "No installer found. Visit github.com/$repo/releases" }

Write-Info "-> Orbit $($release.tag_name) found"
Write-Host ''

# ── Download ──────────────────────────────────────────────────────────────────
Write-Step "Downloading $($asset.name)"
$dest = Join-Path $env:TEMP $asset.name

$http = New-Object System.Net.Http.HttpClient
try {
    $response  = $http.GetAsync(
        $asset.browser_download_url,
        [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead
    ).GetAwaiter().GetResult()

    $total     = $response.Content.Headers.ContentLength
    $src       = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
    $dst       = [System.IO.File]::Create($dest)
    $buf       = New-Object byte[] 32768
    $totalRead = [long]0

    while (($n = $src.Read($buf, 0, $buf.Length)) -gt 0) {
        $dst.Write($buf, 0, $n)
        $totalRead += $n
        $pct = if ($total -gt 0) { [int](($totalRead / $total) * 100) } else { 0 }
        Show-Progress $totalRead $total $pct
    }
    $dst.Close(); $src.Close()
}
finally {
    $http.Dispose()
}

$finalMB = [math]::Round((Get-Item $dest).Length / 1MB, 1)
Write-Host "`r    [$([string]"`u{2588}" * 20)] 100%  $finalMB MB / $finalMB MB  " -ForegroundColor Green
Write-Host ''

# ── Install ───────────────────────────────────────────────────────────────────
Write-Step 'Running installer...'
Write-Info '(the installation window will open)'
Write-Host ''
Start-Process -FilePath $dest -Wait

# ── Cleanup ───────────────────────────────────────────────────────────────────
Remove-Item $dest -ErrorAction SilentlyContinue

# ── Done ─────────────────────────────────────────────────────────────────────
Write-Host ''
Write-Sep
Write-Host ''
Write-Success 'Orbit installed successfully.'
Write-Host ''
Write-Info 'Open Orbit from the Start Menu.'
Write-Info 'Docs  -> github.com/xinnaider/orbit'
Write-Host ''
Write-Sep
Write-Host ''
