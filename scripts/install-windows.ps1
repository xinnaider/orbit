# Orbit Windows installer
# Downloads the latest .exe installer from GitHub and runs it.
#
# Usage (PowerShell):
#   irm https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-windows.ps1 | iex

$ErrorActionPreference = 'Stop'
$repo = 'xinnaider/orbit'

Write-Host ''
Write-Host '  Installing Orbit...' -ForegroundColor Green
Write-Host ''

# Resolve latest release
Write-Host '  Fetching latest release from GitHub...'
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$asset   = $release.assets | Where-Object { $_.name -like '*-setup.exe' } | Select-Object -First 1

if (-not $asset) {
    Write-Host '  ERROR: Could not find installer in the latest release.' -ForegroundColor Red
    Write-Host "  Check https://github.com/$repo/releases for available assets."
    exit 1
}

# Download installer to temp
$installer = Join-Path $env:TEMP $asset.name
Write-Host "  Downloading $($asset.name)..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installer -UseBasicParsing

# Run installer
Write-Host '  Running installer...'
Start-Process -FilePath $installer -Wait

# Cleanup
Remove-Item $installer -ErrorAction SilentlyContinue

Write-Host ''
Write-Host '  Orbit installed. Open it from the Start Menu.' -ForegroundColor Green
Write-Host ''
