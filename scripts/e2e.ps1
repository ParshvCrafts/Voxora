# End-to-end test of the real app: hotkey hook -> WASAPI capture -> VAD -> whisper-server ->
# rewrite -> clipboard paste into a real Win32 text box -> clipboard restore -> SQLite history.
#
# Run it while nobody is using the PC: the app pastes into whatever window has focus.
# Needs: VB-Audio Virtual Cable; Voxora (debug or release build) started with VOXORA_TEST_INJECTED=1 and
# its microphone set to "CABLE Output"; bench audio generated (scripts/gen-bench-audio.ps1).
param([string]$Audio = "$PSScriptRoot\..\bench\audio")
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
& $player "$Audio\corr-01.wav" "CABLE Input" | Out-Null
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
& $player "$Audio\jarg-01.wav" "CABLE Input" | Out-Null
Assert-Focus
$got = WaitText "" 9000
Check "hands-free auto-stops on silence" ($got -match "FAISS") $got

# 3. Escape cancels: nothing inserted.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\fast-02.wav" "CABLE Input" | Out-Null
[K]::Down($ESC); [K]::Up($ESC); Pump 100; [K]::Up($RALT)
$got = WaitText "" 2500
Check "Esc cancels without inserting" ($got -eq "") "text='$got'"

# 4. Silence: no hallucinated text.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\silence-5s.wav" "CABLE Input" | Out-Null
Assert-Focus; [K]::Up($RALT)
$got = WaitText "" 3000
Check "silence inserts nothing" ($got -eq "") "text='$got'"

# 5. Spoken math in a technical context.
Focus-Box; $box.Text = ""
[K]::Down($RALT); Pump 300
& $player "$Audio\math-03.wav" "CABLE Input" | Out-Null
Pump 200; Assert-Focus; [K]::Up($RALT)
$got = WaitText ""
Check "spoken math rendered" ($got.Contains([string][char]0x00B2)) $got

$form.Close()
$failed = @($results | Where-Object { -not $_.Pass }).Count
Write-Host "`n$($results.Count - $failed)/$($results.Count) end-to-end checks passed"
exit $failed
