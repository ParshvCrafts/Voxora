# One dictation into a real web page in Chrome, recorded: places the Chrome window above the HUD
# on a plain backdrop, records that box with ffmpeg, holds the dictation key while a synthesized
# voice plays into VB-Audio Virtual Cable (Voxora's mic), and releases it. The page itself (which
# box has focus, clearing it, reading the result) is driven by capture-chrome.cjs over CDP, so
# the only OS keystroke is the dictation key (and Esc if focus is lost).
#
# Nobody may use the PC meanwhile: Voxora pastes into whatever has focus. Every key press checks
# that the foreground window is this Chrome first, like scripts/e2e.ps1; never weaken that.
param(
  [Parameter(Mandatory)][int]$ChromePid,
  [Parameter(Mandatory)][string]$Clip,
  [Parameter(Mandatory)][string]$Wav,
  [string]$Out = "CABLE Input",
  [string]$CableIn = "CABLE Output (2- VB-Audio Virtual Cable)",
  [int]$WinW = 1500, [int]$WinH = 1050,
  [double]$Tail = 5.5,
  # Where to click so the text box gets keyboard focus: CSS px from the window's top-left at
  # Chrome's 1.5 scale (computed by capture-chrome.cjs). Activating Chrome alone focuses the
  # address bar, which would receive the paste.
  [Parameter(Mandatory)][double]$ClickX, [Parameter(Mandatory)][double]$ClickY
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int hh, uint f);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  public static bool ForceForeground(IntPtr h) {
    IntPtr fg = GetForegroundWindow(); if (fg == h) return true;
    uint t = GetWindowThreadProcessId(fg, IntPtr.Zero), me = GetCurrentThreadId();
    AttachThreadInput(me, t, true); SetForegroundWindow(h); AttachThreadInput(me, t, false);
    return GetForegroundWindow() == h;
  }
  public static void Key(byte vk, bool up) { keybd_event(vk, 0, (uint)(1 | (up ? 2 : 0)), UIntPtr.Zero); }
  public static void Tap(byte vk) { keybd_event(vk, 0, 0, UIntPtr.Zero); keybd_event(vk, 0, 2, UIntPtr.Zero); }
}
"@
[void][K]::SetProcessDPIAware()
$RALT = 0xA5; $ESC = 0x1B
$root = Resolve-Path "$PSScriptRoot\.."
$player = Join-Path $root "target\debug\examples\play_to_device.exe"
$clips = Join-Path $PSScriptRoot "clips"; New-Item -ItemType Directory -Force $clips | Out-Null

# Layout in physical pixels: Chrome centred above the HUD (the HUD sits 84 px above the work
# area bottom and is 96 px tall at 150 %). Not $H/$W: PowerShell names are case-insensitive.
$wa = [Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$hudTop = $wa.Bottom - 84 - 96
$winX = [int](($wa.Width - $WinW) / 2); $winY = $hudTop - 30 - $WinH
$cap = @{ x = $winX; y = $winY; w = $WinW; h = ($hudTop + 96 + 30 - $winY) }
if ($cap.h % 2) { $cap.h++ }

$h = (Get-Process -Id $ChromePid).MainWindowHandle
if ($h -eq [IntPtr]::Zero) { throw "Chrome window for pid $ChromePid not found" }

$bg = New-Object Windows.Forms.Form -Property @{ FormBorderStyle = "None"; BackColor = [Drawing.Color]::FromArgb(24, 24, 27); ShowInTaskbar = $false; StartPosition = "Manual" }
$bg.Bounds = $wa; $bg.Show(); $bg.Activate(); [Windows.Forms.Application]::DoEvents()
[void][K]::ShowWindow($h, 9)
[void][K]::SetWindowPos($h, [IntPtr]::Zero, $winX, $winY, $WinW, $WinH, 0x40)

function Ensure-Focus([string]$where) {
  for ($i = 0; $i -lt 10 -and -not [K]::ForceForeground($h); $i++) { Start-Sleep -Milliseconds 100 }
  if ([K]::GetForegroundWindow() -ne $h) {
    # Foreground lock: click Chrome's own empty tab-strip area, as a user would. No keys sent.
    [Windows.Forms.Cursor]::Position = New-Object Drawing.Point(($winX + $WinW - 260), ($winY + 12))
    [K]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [K]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300
  }
  if ([K]::GetForegroundWindow() -ne $h) { $bg.Close(); throw "Could not focus Chrome ($where); aborting before any key press." }
}
function Assert-Focus([string]$where) {
  if ([K]::GetForegroundWindow() -ne $h) {
    [K]::Tap($ESC); Start-Sleep -Milliseconds 100; [K]::Key($RALT, $true)
    $bg.Close(); throw "Chrome lost focus ($where); recording cancelled, nothing pasted elsewhere."
  }
}
Ensure-Focus "start"
$cx = $winX + [int]($ClickX * 1.5); $cy = $winY + [int]($ClickY * 1.5)
if ($cx -lt $winX + 10 -or $cx -gt $winX + $WinW - 10 -or $cy -lt $winY + 120 -or $cy -gt $winY + $WinH - 10) {
  $bg.Close(); throw "click point ($cx,$cy) is outside the page area; aborting"
}
[Windows.Forms.Cursor]::Position = New-Object Drawing.Point($cx, $cy)
[K]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [K]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300
Ensure-Focus "after click"
[Windows.Forms.Cursor]::Position = New-Object Drawing.Point(($cap.x + $cap.w + 200), 50)
Start-Sleep -Milliseconds 700

$mkv = Join-Path $clips "$Clip.mkv"
$speechSecs = (Get-Item $Wav).Length / 32000   # 16 kHz, 16-bit mono
$dur = [math]::Ceiling(1.5 + 0.4 + $speechSecs + 0.3 + $Tail)
$ff = Start-Process ffmpeg -PassThru -WindowStyle Hidden -RedirectStandardError (Join-Path $clips "$Clip.ffmpeg.log") -ArgumentList @(
  "-y", "-f", "gdigrab", "-framerate", "30", "-draw_mouse", "0", "-offset_x", $cap.x, "-offset_y", $cap.y,
  "-video_size", "$($cap.w)x$($cap.h)", "-i", "desktop",
  "-t", $dur, "-c:v", "libx264", "-preset", "ultrafast", "-crf", "12", "-pix_fmt", "yuv420p", "`"$mkv`"")
$t0 = Get-Date
Start-Sleep -Milliseconds 1500
Ensure-Focus "key down"
[K]::Key($RALT, $false); $tDown = ((Get-Date) - $t0).TotalSeconds
Start-Sleep -Milliseconds 400
$p = Start-Process $player -ArgumentList @("`"$Wav`"", "`"$Out`"") -PassThru -WindowStyle Hidden
$null = $p.Handle   # cache it so ExitCode is readable after exit
# Watch focus while the key is held; the voice plays meanwhile.
while (-not $p.HasExited) { Assert-Focus "while held"; Start-Sleep -Milliseconds 50 }
if ($p.ExitCode -ne 0) { [K]::Tap($ESC); [K]::Key($RALT, $true); $bg.Close(); throw "player failed ($($p.ExitCode))" }
Start-Sleep -Milliseconds 300
Assert-Focus "key up"
[K]::Key($RALT, $true); $tUp = ((Get-Date) - $t0).TotalSeconds
$ff.WaitForExit(($dur + 15) * 1000) | Out-Null
$bg.Close()
@{ clip = $Clip; keyDown = $tDown; keyUp = $tUp; capture = $cap; window = @{ x = $winX; y = $winY; w = $WinW; h = $WinH } } |
  ConvertTo-Json | Set-Content (Join-Path $clips "$Clip.keys.json") -Encoding utf8
Write-Host "saved $mkv (key down $([math]::Round($tDown,2)) s, up $([math]::Round($tUp,2)) s)"
