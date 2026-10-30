# Builds Langflip from this checkout and installs it for the current user, in place of the running copy.
# Run it with `mise run install`. The installed exe rewrites the autostart path on its first start.
$ErrorActionPreference = "Stop"

$dir = Join-Path $env:LOCALAPPDATA "Programs\Langflip"
$exe = Join-Path $dir "langflip.exe"
# A date version like the release one, so Explorer shows which build is installed.
$env:LANGFLIP_VERSION = (Get-Date -Format "yyyy.M.d") + ".0"

cargo build --release -p langflip
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# A running copy locks the exe, and two copies would fix each word twice.
Get-Process langflip -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

New-Item -ItemType Directory -Force $dir | Out-Null
Copy-Item "target\release\langflip.exe" $exe -Force
Start-Process $exe

Start-Sleep -Seconds 2
if (-not (Get-Process langflip -ErrorAction SilentlyContinue)) { Write-Output "langflip.exe didn't start"; exit 1 }
$run = (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -ErrorAction SilentlyContinue).Langflip
Write-Output "installed $($env:LANGFLIP_VERSION) at $exe"
Write-Output "autostart: $(if ($run) { $run } else { 'off' })"
