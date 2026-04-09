#Requires -Version 5.1
# Orbit — Windows Installer
# Usage: irm https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-windows.ps1 | iex

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'
$repo = 'xinnaider/orbit'

# ── Symbols (PS 5.1-compatible — no `u{} escape syntax) ──────────────────────
$S_DIAMOND = [char]0x25C6   # ◆
$S_CHECK   = [char]0x2713   # ✓
$S_CROSS   = [char]0x2717   # ✗
$S_SEP     = [char]0x2500   # ─
$S_BLOCK   = [char]0x2588   # █
$S_LIGHT   = [char]0x2591   # ░

# ── Helpers ───────────────────────────────────────────────────────────────────
function Write-Step    { param($m) Write-Host "  $S_DIAMOND $m" -ForegroundColor Green }
function Write-Info    { param($m) Write-Host "    $m" -ForegroundColor DarkGray }
function Write-Success { param($m) Write-Host "  $S_CHECK $m" -ForegroundColor Green }
function Write-Fail    { param($m) Write-Host "  $S_CROSS $m" -ForegroundColor Red; exit 1 }
function Write-Sep     { Write-Host "  $(($S_SEP.ToString()) * 35)" -ForegroundColor DarkGray }

function Show-Progress {
    param([long]$bytes, [long]$total, [int]$pct)
    $filled = [math]::Min([int]($pct / 5), 20)
    $empty  = 20 - $filled
    $bar    = ($S_BLOCK.ToString() * $filled) + ($S_LIGHT.ToString() * $empty)
    $dlMB   = [math]::Round($bytes / 1MB, 1)
    $totMB  = [math]::Round($total / 1MB, 1)
    Write-Host "`r    [$bar] $pct%  $dlMB MB / $totMB MB  " -NoNewline -ForegroundColor Green
}

# ── Header ────────────────────────────────────────────────────────────────────
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
$fullBar = $S_BLOCK.ToString() * 20
Write-Host "`r    [$fullBar] 100%  $finalMB MB / $finalMB MB  " -ForegroundColor Green
Write-Host ''

# ── Install ───────────────────────────────────────────────────────────────────
# The NSIS installer self-elevates via UAC, spawning a new elevated process.
# Start-Process -Wait only tracks the initial (non-elevated) launcher, which
# exits immediately after spawning the elevated child. We instead launch the
# installer and poll until no process with that name is running.
Write-Step 'Running installer...'
Write-Info '(complete the installation, then return here)'
Write-Host ''

$installerName = [System.IO.Path]::GetFileNameWithoutExtension($dest)
Start-Process -FilePath $dest

# Brief pause so the process list catches the elevated child before we poll
Start-Sleep -Seconds 2

$deadline = (Get-Date).AddMinutes(15)
while ((Get-Date) -lt $deadline) {
    $procs = Get-Process | Where-Object { $_.Name -like "$installerName*" -or $_.Name -like '*orbit*setup*' -or $_.Name -like '*orbit*install*' }
    if (-not $procs) { break }
    Start-Sleep -Milliseconds 800
}

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
