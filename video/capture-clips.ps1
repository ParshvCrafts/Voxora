# Records the demo video's dictation clips from the real app: Voxora's own window (Home page,
# "Try it" box), the real mode buttons, the real HUD, real pasting. The speech is a synthesized
# voice (video/speech) played into VB-Audio Virtual Cable, which Voxora uses as its microphone;
# ffmpeg records the screen region and the cable together, so the clip's audio is exactly what
# Voxora heard.
#
# Needs: Voxora release build running with VOXORA_TEST_INJECTED=1, its main window open on Home,
# and its mic set to the cable's "CABLE Output"; ffmpeg on PATH; 16 kHz speech WAVs in
# video/speech/out16. Nobody using the PC: the app pastes into whatever has focus, so every key
# press is guarded like scripts/e2e.ps1. Clicking a mode button changes the saved mode.
# (Notepad restores the user's other tabs and a second VS Code cannot start from a VS Code
# terminal, so the demo uses Voxora's own window.)
param(
  [ValidateSet("math", "code", "corr")][string]$Clip = "math",
  [string]$Out = "CABLE Input",
  [string]$CableIn = "CABLE Output (2- VB-Audio Virtual Cable)"
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing, UIAutomationClient, UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
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
# 1.2 s of faint noise first (see video/README.md): the cable carries exact digital zeros until
# the player is up, which a real mic never does, and that trips the "No sound from the mic" HUD.
$speech = Join-Path $PSScriptRoot "speech\out16\$Clip.padded.wav"
$clips = Join-Path $PSScriptRoot "clips"; New-Item -ItemType Directory -Force $clips | Out-Null
$modeButton = @{ math = "Math / LaTeX"; code = "Technical"; corr = "Clean" }[$Clip]

# Layout in physical pixels: the window centred above the HUD (84 px above the work area bottom).
$wa = [Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$winW = 1500; $winH = 1000; $winX = [int](($wa.Width - $winW) / 2); $hudTop = $wa.Bottom - 84 - 96
$winY = $hudTop - 30 - $winH
# Not $H/$W/$X/$Y: PowerShell names are case-insensitive and $h is the window handle.
$cap = @{ x = $winX; y = $winY; w = $winW; h = ($hudTop + 96 + 30 - $winY) }
if ($cap.h % 2) { $cap.h++ }

# A plain backdrop so nothing personal is in the capture box around the window and the HUD.
$bg = New-Object Windows.Forms.Form -Property @{ FormBorderStyle = "None"; BackColor = [Drawing.Color]::FromArgb(24, 24, 27); ShowInTaskbar = $false; StartPosition = "Manual" }
$bg.Bounds = $wa; $bg.Show(); $bg.Activate(); [Windows.Forms.Application]::DoEvents()

$h = [K]::FindWindow("Tauri Window", "Voxora")
if ($h -eq [IntPtr]::Zero) { $bg.Close(); throw "Voxora main window not found (open it from the tray)" }
[void][K]::ShowWindow($h, 9)
[void][K]::SetWindowPos($h, [IntPtr]::Zero, $winX, $winY, $winW, $winH, 0x40)
# Before the key goes down, taking focus back is safe; while it is held, losing focus aborts.
function Ensure-Focus([string]$where) {
  for ($i = 0; $i -lt 10 -and -not [K]::ForceForeground($h); $i++) { Start-Sleep -Milliseconds 100 }
  if ([K]::GetForegroundWindow() -ne $h) {
    # Windows' foreground lock can refuse a background process; a click on Voxora's own title
    # bar is what a user would do and sends no keys anywhere.
    [Windows.Forms.Cursor]::Position = New-Object Drawing.Point(($winX + 400), ($winY + 15))
    [K]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [K]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300
  }
  if ([K]::GetForegroundWindow() -ne $h) { $bg.Close(); throw "Could not focus Voxora ($where); aborting before any key press." }
}
function Assert-Focus([string]$where) {
  if ([K]::GetForegroundWindow() -ne $h) {
    [K]::Tap($ESC); Start-Sleep -Milliseconds 100; [K]::Key($RALT, $true)
    $bg.Close(); throw "Voxora lost focus ($where); recording cancelled, nothing pasted elsewhere."
  }
}
Ensure-Focus "start"; Start-Sleep -Milliseconds 500

function Find([Windows.Automation.ControlType]$type, [scriptblock]$match) {
  $cond = New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty, $type)
  for ($i = 0; $i -lt 20; $i++) {   # WebView2 builds its accessibility tree lazily
    $e = [Windows.Automation.AutomationElement]::FromHandle($h).FindAll([Windows.Automation.TreeScope]::Descendants, $cond) |
      Where-Object $match | Select-Object -First 1
    if ($e) { return $e }
    Start-Sleep -Milliseconds 250
  }
  $bg.Close(); throw "UI element not found"
}
function Click($e) {   # a real click: UIA focus alone can leave focus outside the webview
  $r = $e.Current.BoundingRectangle
  if ($r.Y -lt $winY + 40 -or $r.Bottom -gt $winY + $winH -or $r.X -lt $winX -or $r.Right -gt $winX + $winW) {
    $bg.Close(); throw "click target $($e.Current.Name) at $r is outside the window ($winX,$winY $($winW)x$($winH)); aborting"
  }
  [Windows.Forms.Cursor]::Position = New-Object Drawing.Point([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2))
  [K]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [K]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}

# Bring "Try it" into view and empty it, then park the pointer outside the capture box.
$try = Find ([Windows.Automation.ControlType]::Edit) { $_.Current.Name -like "*hold*" }
$try.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue("")   # before scrolling: it scrolls the box into view
Start-Sleep -Milliseconds 300
# Scroll Home with the wheel (over Voxora only) until the mode row is near the top, so the mode
# buttons and the box are both in frame; this also scrolls the usage stats out of frame.
[Windows.Forms.Cursor]::Position = New-Object Drawing.Point(($winX + [int]($winW * 0.6)), ($winY + [int]($winH / 2)))
for ($i = 0; $i -lt 15; $i++) { [K]::mouse_event(0x800, 0, 0, 120, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }   # to the top
Start-Sleep -Milliseconds 300
$modeEl = Find ([Windows.Automation.ControlType]::RadioButton) { $_.Current.Name -eq $modeButton }
for ($i = 0; $i -lt 15 -and $modeEl.Current.BoundingRectangle.Y -gt ($winY + 220); $i++) {
  [K]::mouse_event(0x800, 0, 0, [uint32]4294967176, [UIntPtr]::Zero); Start-Sleep -Milliseconds 450   # smooth scroll: let the rect settle
}
Start-Sleep -Milliseconds 400
$mp4 = Join-Path $clips "$Clip.mkv"
$speechSecs = (Get-Item $speech).Length / 32000   # 16 kHz, 16-bit mono
$dur = [math]::Ceiling(2.5 + 0.4 + $speechSecs + 0.3 + 5)   # ffmpeg stops itself and finalizes
$ff = Start-Process ffmpeg -PassThru -WindowStyle Hidden -RedirectStandardError (Join-Path $clips "$Clip.ffmpeg.log") -ArgumentList @(
  "-y", "-f", "gdigrab", "-framerate", "30", "-draw_mouse", "0", "-offset_x", $cap.x, "-offset_y", $cap.y,
  "-video_size", "$($cap.w)x$($cap.h)", "-i", "desktop",
  "-f", "dshow", "-i", "audio=`"$CableIn`"",
  "-t", $dur, "-c:v", "libx264", "-preset", "ultrafast", "-crf", "12", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "`"$mp4`"")
$t0 = Get-Date
Start-Sleep -Milliseconds 1200
# On camera: pick the mode, then click into the box.
Ensure-Focus "mode"; Click $modeEl; $tMode = ((Get-Date) - $t0).TotalSeconds
Start-Sleep -Milliseconds 700
Ensure-Focus "box"; Click $try; Start-Sleep -Milliseconds 300
[Windows.Forms.Cursor]::Position = New-Object Drawing.Point(($cap.x + $cap.w + 200), 50)
Start-Sleep -Milliseconds 300
Ensure-Focus "key down"
[K]::Key($RALT, $false); $tDown = ((Get-Date) - $t0).TotalSeconds
Start-Sleep -Milliseconds 400
& $player $speech $Out | Out-Null
Start-Sleep -Milliseconds 300
Assert-Focus "key up"
[K]::Key($RALT, $true); $tUp = ((Get-Date) - $t0).TotalSeconds
$ff.WaitForExit(($dur + 15) * 1000) | Out-Null
$text = $try.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).Current.Value
$bg.Close()
@{ clip = $Clip; mode = $modeButton; modeClick = $tMode; keyDown = $tDown; keyUp = $tUp; result = $text; capture = $cap; window = @{ x = $winX; y = $winY; w = $winW; h = $winH } } |
  ConvertTo-Json | Set-Content (Join-Path $clips "$Clip.json") -Encoding utf8
Write-Host "saved $mp4 (mode click $([math]::Round($tMode,2)) s, key down $([math]::Round($tDown,2)) s, up $([math]::Round($tUp,2)) s)"
Write-Host "result: $text"
