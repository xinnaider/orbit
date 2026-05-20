# Clean Orbit build artifacts
# Run this to free up disk space (target typically 5-10 GB)

Write-Host "Orbit Build Cleanup" -ForegroundColor Cyan
Write-Host "===================" -ForegroundColor Cyan

# 1. Clean Rust target directories
Write-Host "`nCleaning Rust target directories..." -ForegroundColor Yellow
if (Test-Path "tauri\target") {
    $size = (Get-ChildItem "tauri\target" -Recurse -File -ErrorAction SilentlyContinue | 
             Measure-Object -Property Length -Sum).Sum
    Write-Host "  Current size: $([math]::Round($size/1GB, 2)) GB" -ForegroundColor Red
    
    cargo clean --manifest-path tauri/Cargo.toml
    
    Write-Host "  Target directory cleaned." -ForegroundColor Green
} else {
    Write-Host "  No target directory found." -ForegroundColor Gray
}

# 2. Clean Node modules cache
Write-Host "`nCleaning Node.js cache..." -ForegroundColor Yellow
if (Test-Path "node_modules\.cache") {
    Remove-Item -Path "node_modules\.cache" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "  Cache cleared." -ForegroundColor Green
} else {
    Write-Host "  No cache found." -ForegroundColor Gray
}

# 3. Clean Vite/SvelteKit build artifacts
Write-Host "`nCleaning frontend build artifacts..." -ForegroundColor Yellow
$paths = @(
    ".svelte-kit",
    "build",
    "ui\build",
    ".vite"
)
foreach ($p in $paths) {
    if (Test-Path $p) {
        Remove-Item -Path $p -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  Removed: $p" -ForegroundColor Green
    }
}

# 4. Clean test artifacts
Write-Host "`nCleaning test artifacts..." -ForegroundColor Yellow
$testFiles = Get-ChildItem -Path "." -Recurse -Filter "*.db" -ErrorAction SilentlyContinue
foreach ($f in $testFiles) {
    Remove-Item -Path $f.FullName -Force -ErrorAction SilentlyContinue
    Write-Host "  Removed: $($f.FullName)" -ForegroundColor Green
}

Write-Host "`nDone! Project size reduced." -ForegroundColor Cyan
Write-Host "Next: run 'cargo test' to rebuild only what you need." -ForegroundColor Cyan
