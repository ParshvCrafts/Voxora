# End-to-end test of the real app: hotkey hook -> WASAPI capture -> VAD -> whisper-server ->
# rewrite -> clipboard paste into a real Win32 text box -> clipboard restore -> SQLite history.
#
# Run it while nobody is using the PC: the app pastes into whatever window has focus.
# Needs: VB-Audio Virtual Cable; Voxora (debug or release build) started with VOXORA_TEST_INJECTED=1 and
# its microphone set to "CABLE Output"; bench audio generated (scripts/gen-bench-audio.ps1).
# -Out: the cable's playback endpoint (its name varies by driver install, e.g. "Speakers (2- VB-Audio Virtual Cable)").
param([string]$Audio = "$PSScriptRoot\..\bench\audio", [string]$Out = "CABLE Input")
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  // Windows only lets the foreground thread hand focus to another window; borrow its input queue.
  public static bool ForceForeground(IntPtr h) {
    IntPtr fg = GetForegroundWindow(); if (fg == h) return true;
    uint fgThread = GetWindowThreadProcessId(fg, IntPtr.Zero), me = GetCurrentThreadId();
    AttachThreadInput(me, fgThread, true); bool ok = SetForegroundWindow(h); AttachThreadInput(me, fgThread, false);
    return GetForegroundWindow() == h;
  }
  public static void Down(byte vk) { keybd_event(vk, 0, 1, UIntPtr.Zero); }
  public static void Up(byte vk) { keybd_event(vk, 0, 3, UIntPtr.Zero); }
}
"@
$RALT = 0xA5; $ESC = 0x1B
$root = Resolve-Path "$PSScriptRoot\.."
$player = Join-Path $root "target\debug\examples\play_to_device.exe"
if (-not (Test-Path $player)) { cargo build -p voxora --example play_to_device -q }

$form = New-Object Windows.Forms.Form -Property @{ Text = "Voxora E2E target"; Width = 700; Height = 300; TopMost = $true }
$box = New-Object Windows.Forms.TextBox -Property @{ Multiline = $true; Dock = "Fill"; Font = New-Object Drawing.Font("Segoe UI", 12) }
$form.Controls.Add($box); $form.Show(); [Windows.Forms.Application]::DoEvents()

function Pump([int]$ms) { $end = (Get-Date).AddMilliseconds($ms); while ((Get-Date) -lt $end) { [Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 15 } }
# Safety: the app pastes into whatever window has focus, so never press the hotkey unless the
# test window is verifiably in front. Abort instead of typing into someone's editor.
function Focus-Box {
  for ($i = 0; $i -lt 20; $i++) {
    if ([K]::ForceForeground($form.Handle)) { $box.Focus() | Out-Null; Pump 250; if ([K]::GetForegroundWindow() -eq $form.Handle) { return } }
    Pump 100
  }
  $form.Close(); throw "Could not bring the test window to the foreground; aborting so no text lands in another app."
}
# On focus loss: Esc first (cancels the recording, nothing is pasted), then release the key.
function Assert-Focus {
  if ([K]::GetForegroundWindow() -ne $form.Handle) {
    [K]::Down($ESC); [K]::Up($ESC); Start-Sleep -Milliseconds 100; [K]::Up($RALT)
    $form.Close(); throw "Test window lost focus mid-test; recording cancelled, aborting."
  }
}
function WaitText([string]$before, [int]$ms = 8000) {
  $end = (Get-Date).AddMilliseconds($ms)
  while ((Get-Date) -lt $end) { [Windows.Forms.Application]::DoEvents(); if ($box.Text -ne $before) { Pump 300; return $box.Text }; Start-Sleep -Milliseconds 20 }
  return $box.Text
}
$results = @()
function Check([string]$name, [bool]$ok, [string]$detail) {
  $script:results += [pscustomobject]@{ Test = $name; Pass = $ok; Detail = $detail }
  Write-Host ("{0} {1}: {2}" -f ($(if ($ok) { "PASS" } else { "FAIL" })), $name, $detail)
}

# 1. Push-to-talk, self-correction, clipboard preservation.
Set-Clipboard -Value "user's precious clipboard"
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 400
& $player "$Audio\corr-01.wav" $Out | Out-Null
Pump 300; Assert-Focus; [K]::Up($RALT)
$t0 = Get-Date; $got = WaitText ""
$lat = [int]((Get-Date) - $t0).TotalMilliseconds
Check "push-to-talk inserts text" ($got.Length -gt 10) "$got  (release->text ${lat} ms incl. UI polling)"
Check "self-correction resolved" ($got -match "Wednesday" -and $got -notmatch "Tuesday") $got
Pump 600
Check "clipboard restored" ((Get-Clipboard) -eq "user's precious clipboard") (Get-Clipboard)

# 2. Hands-free: quick tap, speak, auto-stop on silence.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 60; [K]::Up($RALT); Pump 300
& $player "$Audio\jarg-01.wav" $Out | Out-Null
Assert-Focus
$got = WaitText "" 9000
Check "hands-free auto-stops on silence" ($got -match "FAISS") $got

# 3. Escape cancels: nothing inserted.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\fast-02.wav" $Out | Out-Null
[K]::Down($ESC); [K]::Up($ESC); Pump 100; [K]::Up($RALT)
$got = WaitText "" 2500
Check "Esc cancels without inserting" ($got -eq "") "text='$got'"

# 4. Silence: no hallucinated text.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\silence-5s.wav" $Out | Out-Null
Assert-Focus; [K]::Up($RALT)
$got = WaitText "" 3000
Check "silence inserts nothing" ($got -eq "") "text='$got'"

# 5. Spoken math in a technical context.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\math-03.wav" $Out | Out-Null
Pump 200; Assert-Focus; [K]::Up($RALT)
$got = WaitText ""
Check "spoken math rendered" ($got.Contains([string][char]0x00B2)) $got

$form.Close()

# 6. Voxora's own window (D15): Windows hides key-downs aimed at it from the hook, so the window
# forwards them. Dictate into the Home "Try it" box, then check Alt is not left stuck down.
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class M {
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
}
"@
$main = [M]::FindWindow("Tauri Window", "Voxora")
if ($main -eq [IntPtr]::Zero) { Check "dictate into Voxora's own window" $false "main window not found (open it from the tray)" }
else {
  [M]::ShowWindow($main, 9) | Out-Null
  for ($i = 0; $i -lt 20 -and -not [K]::ForceForeground($main); $i++) { Start-Sleep -Milliseconds 100 }
  if ([K]::GetForegroundWindow() -ne $main) { throw "Could not bring Voxora to the foreground; aborting." }
  $edit = New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty, [Windows.Automation.ControlType]::Edit)
  $try = $null   # WebView2 builds its accessibility tree lazily: retry briefly
  for ($i = 0; $i -lt 20 -and -not $try; $i++) {
    $try = [Windows.Automation.AutomationElement]::FromHandle($main).FindAll([Windows.Automation.TreeScope]::Descendants, $edit) |
      Where-Object { $_.Current.Name -like "*hold*" } | Select-Object -First 1
    if (-not $try) { Start-Sleep -Milliseconds 250 }
  }
  if (-not $try) { Check "dictate into Voxora's own window" $false "Try it box not found (open the Home page)" }
  else {
    $value = $try.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern); $value.SetValue("")
    $r = $try.Current.BoundingRectangle   # a real click: UIA SetFocus can leave focus outside the webview
    [Windows.Forms.Cursor]::Position = New-Object Drawing.Point([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2))
    [M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300
    if ([K]::GetForegroundWindow() -ne $main) { throw "Voxora lost focus; aborting." }
    [K]::Down($RALT); Start-Sleep -Milliseconds 400
    & $player "$Audio\corr-01.wav" $Out | Out-Null
    Start-Sleep -Milliseconds 300; [K]::Up($RALT)
    $end = (Get-Date).AddSeconds(8); $got = ""
    while ((Get-Date) -lt $end -and -not $got) { Start-Sleep -Milliseconds 100; $got = $value.Current.Value }
    Check "dictate into Voxora's own window" ($got -match "Wednesday") $got
    Check "Alt not left stuck down" (-not ([M]::GetAsyncKeyState(0x12) -lt 0)) "after a press in Voxora's window"
  }
}

$failed = @($results | Where-Object { -not $_.Pass }).Count
Write-Host "`n$($results.Count - $failed)/$($results.Count) end-to-end checks passed"
exit $failed
