# Starts Langflip, types words into a test window with injected scan codes and checks the text.
# Run it with `mise run smoke`. It takes the keyboard for about 10 seconds.
param([string]$Exe = "target\debug\langflip.exe")

$ErrorActionPreference = "Stop"
# Another layout switcher fixes the words before the check reads them.
if (Get-Process punto -ErrorAction SilentlyContinue) { Write-Output "close Punto Switcher first"; exit 1 }
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Keys2 {
    [StructLayout(LayoutKind.Sequential)] struct KEYBDINPUT { public ushort vk; public ushort scan; public uint flags; public uint time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit, Size = 40)] struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public KEYBDINPUT ki; }
    [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
    // Windows lets a process take the foreground right after it sends a key, so tap Shift first. Alt would open the window menu.
    public static void Front(IntPtr hwnd) { keybd_event(0x10, 0, 0, IntPtr.Zero); keybd_event(0x10, 0, 2, IntPtr.Zero); SetForegroundWindow(hwnd); }
    public static void TapVk(byte vk) { keybd_event(vk, 0, 0, IntPtr.Zero); keybd_event(vk, 0, 2, IntPtr.Zero); }
    public static void Tap(ushort scan) {
        var down = new INPUT { type = 1, ki = new KEYBDINPUT { scan = scan, flags = 0x8 } };
        var up = new INPUT { type = 1, ki = new KEYBDINPUT { scan = scan, flags = 0x8 | 0x2 } };
        SendInput(2, new[] { down, up }, Marshal.SizeOf(typeof(INPUT)));
    }
}
"@

# Scan codes of the US keys, as a physical keyboard sends them.
$scan = @{
    'q'=0x10;'w'=0x11;'e'=0x12;'r'=0x13;'t'=0x14;'y'=0x15;'u'=0x16;'i'=0x17;'o'=0x18;'p'=0x19
    'a'=0x1E;'s'=0x1F;'d'=0x20;'f'=0x21;'g'=0x22;'h'=0x23;'j'=0x24;'k'=0x25;'l'=0x26
    'z'=0x2C;'x'=0x2D;'c'=0x2E;'v'=0x2F;'b'=0x30;'n'=0x31;'m'=0x32;','=0x33;'.'=0x34;' '=0x39
}

function Pump([int]$ms) {
    $end = [DateTime]::Now.AddMilliseconds($ms)
    while ([DateTime]::Now -lt $end) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 10 }
}

$form = New-Object System.Windows.Forms.Form
$form.Text = "Langflip smoke test"
$form.TopMost = $true
$box = New-Object System.Windows.Forms.TextBox
$box.Dock = "Fill"
$box.Multiline = $true
$form.Controls.Add($box)
$form.Show()
[Keys2]::Front($form.Handle)
$box.Focus() | Out-Null
$en = [System.Windows.Forms.InputLanguage]::InstalledInputLanguages | Where-Object { $_.Culture.Name -like "en*" } | Select-Object -First 1
[System.Windows.Forms.InputLanguage]::CurrentInputLanguage = $en
Pump 300
if (-not [System.Windows.Forms.InputLanguage]::CurrentInputLanguage.Culture.Name.StartsWith("en")) { Write-Output "the test window did not switch to English"; exit 1 }

# Undo writes to the settings file, so the test gets a folder of its own.
$appData = $env:APPDATA
$env:APPDATA = Join-Path $PWD "target\smoke-appdata"
Remove-Item -Recurse -Force $env:APPDATA -ErrorAction SilentlyContinue
$app = Start-Process -FilePath $Exe -PassThru
$env:APPDATA = $appData
Pump 1500
if ([Keys2]::GetForegroundWindow() -ne $form.Handle) { [Keys2]::Front($form.Handle); Pump 300 }
if ([Keys2]::GetForegroundWindow() -ne $form.Handle) { Stop-Process -Id $app.Id; Write-Output "the test window could not take the foreground"; exit 1 }

# Typed with the English layout on, then whatever layout Langflip switched to.
$typed = "ghbdtn hello rfr k.,k. "
foreach ($c in $typed.ToCharArray()) { [Keys2]::Tap($scan["$c"]); Pump 40 }
Pump 500
# Pause undoes the last fix, so the last word comes back as typed.
[Keys2]::TapVk(0x13)
Pump 800

Stop-Process -Id $app.Id
$text = $box.Text
$form.Close()

# PowerShell 5.1 reads this file as ANSI, so the Russian text is spelled in code points.
function Text([int[]]$codes) { -join ($codes | ForEach-Object { [char]$_ }) }
$expected = (Text 0x43F,0x440,0x438,0x432,0x435,0x442) + " hello rfr k.,k. "
$report = "typed:    '$typed'`ngot:      '$text'`nexpected: '$expected'`n"
[System.IO.File]::WriteAllText("$PWD\target\smoke.txt", $report, (New-Object System.Text.UTF8Encoding $false))
if ($text -ne $expected) { Write-Output "smoke test failed, see target\smoke.txt"; exit 1 }
Write-Output "smoke test passed"
