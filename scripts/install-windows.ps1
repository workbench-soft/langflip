# Builds Langflip from this checkout and installs it for the current user, in place of the running copy,
# with a desktop and a Start menu shortcut and an entry in Settings, Apps.
# Run it with `mise run install` or a double click on Langflip.cmd. The installed exe rewrites the autostart path on its first start.
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
Copy-Item "scripts\uninstall-windows.ps1" (Join-Path $dir "uninstall.ps1") -Force

$shell = New-Object -ComObject WScript.Shell
foreach ($folder in "Desktop", "Programs") {
    $link = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath($folder)) "Langflip.lnk"))
    $link.TargetPath = $exe
    $link.WorkingDirectory = $dir
    $link.Description = "Fixes words typed in the wrong keyboard layout"
    $link.Save()
}

# The entry in Settings, Apps, with the uninstaller behind its Uninstall button.
$key = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Langflip"
New-Item $key -Force | Out-Null
$entry = @{
    DisplayName     = "Langflip"
    DisplayVersion  = $env:LANGFLIP_VERSION
    Publisher       = "Workbench Soft"
    DisplayIcon     = $exe
    InstallLocation = $dir
    UninstallString = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$dir\uninstall.ps1`""
}
foreach ($name in $entry.Keys) { Set-ItemProperty $key $name $entry[$name] }
Set-ItemProperty $key EstimatedSize ([int]((Get-Item $exe).Length / 1KB)) -Type DWord
Set-ItemProperty $key NoModify 1 -Type DWord
Set-ItemProperty $key NoRepair 1 -Type DWord

Start-Process $exe

Start-Sleep -Seconds 2
if (-not (Get-Process langflip -ErrorAction SilentlyContinue)) { Write-Output "langflip.exe didn't start"; exit 1 }
$run = (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -ErrorAction SilentlyContinue).Langflip
Write-Output "installed $($env:LANGFLIP_VERSION) at $exe"
Write-Output "autostart: $(if ($run) { $run } else { 'off' })"
