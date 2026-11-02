# Types words in the wrong layout into a demo window while Langflip runs, then saves the window
# as docs/screenshots/<name>.png. Run it with `mise run demo`. It takes the keyboard for about 5 seconds.
$ErrorActionPreference = "Stop"
if (-not (Get-Process langflip -ErrorAction SilentlyContinue)) { Write-Output "start Langflip first"; exit 1 }
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class DemoKeys {
    [StructLayout(LayoutKind.Sequential)] struct KEYBDINPUT { public ushort vk; public ushort scan; public uint flags; public uint time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit, Size = 40)] struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public KEYBDINPUT ki; }
    [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
    // Windows lets a process take the foreground right after it sends a key, so tap Shift first.
    public static void Front(IntPtr hwnd) { keybd_event(0x10, 0, 0, IntPtr.Zero); keybd_event(0x10, 0, 2, IntPtr.Zero); SetForegroundWindow(hwnd); }
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

# Keys of the US keyboard, pressed with the layout of $culture on, and the text Langflip should leave.
function Shot([string]$name, [string]$culture, [string]$hint, [string]$keys) {
    $form = New-Object System.Windows.Forms.Form
    $form.Text = "Langflip"
    $form.TopMost = $true
    $form.ClientSize = New-Object System.Drawing.Size 760, 230
    $form.StartPosition = "CenterScreen"
    $form.BackColor = [System.Drawing.Color]::White
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $hint
    $label.Font = New-Object System.Drawing.Font "Segoe UI", 13
    $label.ForeColor = [System.Drawing.Color]::DimGray
    $label.SetBounds(24, 20, 712, 70)
    $box = New-Object System.Windows.Forms.TextBox
    $box.Font = New-Object System.Drawing.Font "Segoe UI", 26
    $box.SetBounds(24, 110, 712, 60)
    $form.Controls.AddRange(@($label, $box))
    $form.Show()
    [DemoKeys]::Front($form.Handle)
    $box.Focus() | Out-Null
    $lang = [System.Windows.Forms.InputLanguage]::InstalledInputLanguages | Where-Object { $_.Culture.Name -eq $culture } | Select-Object -First 1
    [System.Windows.Forms.InputLanguage]::CurrentInputLanguage = $lang
    Pump 600
    foreach ($c in $keys.ToCharArray()) { [DemoKeys]::Tap($scan["$c"]); Pump 90 }
    Pump 1200
    $box.SelectionStart = $box.Text.Length
    Pump 300
    $bounds = $form.RectangleToScreen($form.ClientRectangle)
    $image = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    [System.Drawing.Graphics]::FromImage($image).CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $image.Save("$PWD\docs\screenshots\$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
    $form.Close()
}

Shot "01-to-russian" "en-US" "Typed with the English layout on: ghbdtn rfr ltkf" "ghbdtn rfr ltkf "
Shot "02-to-english" "ru-RU" "The keys of hello world, pressed with the Russian layout on" "hello world "
