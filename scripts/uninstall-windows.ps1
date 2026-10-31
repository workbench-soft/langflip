# Removes Langflip for the current user. Settings, Apps runs it from the program folder.
# The settings file in %APPDATA%\Langflip stays, so a new install keeps them.
$ErrorActionPreference = "SilentlyContinue"

$dir = Join-Path $env:LOCALAPPDATA "Programs\Langflip"

# The sign in task goes first, so it can't start Langflip again.
schtasks.exe /Delete /TN Langflip /F 2>&1 | Out-Null
Get-Process langflip | Stop-Process -Force
Start-Sleep -Milliseconds 500

Remove-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name Langflip
Remove-Item (Join-Path ([Environment]::GetFolderPath("Desktop")) "Langflip.lnk")
Remove-Item (Join-Path ([Environment]::GetFolderPath("Programs")) "Langflip.lnk")
Remove-Item "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Langflip" -Recurse
Remove-Item $dir -Recurse -Force
