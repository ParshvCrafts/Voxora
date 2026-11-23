# Captures README images from the real running app (release build started with
# VOXORA_TEST_INJECTED=1 and the mic set to VB-Audio "CABLE Output").
#   docs/assets/home.png     the settings window
#   docs/assets/overlay.png  the pill while listening (dictation is cancelled with Esc, so
#                            nothing is pasted anywhere)
# A solid backdrop window sits behind the pill so no personal screen content is captured.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class C {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte s, uint f, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  public static IntPtr ByTitle(uint pid, string title) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); var sb = new System.Text.StringBuilder(256);
      GetWindowText(h, sb, 256); if (p == pid && sb.ToString() == title) { found = h; return false; } return true; }, IntPtr.Zero);
    return found;
  }
}
"@
[void][C]::SetProcessDPIAware()
$root = Resolve-Path "$PSScriptRoot\.."
$out = Join-Path $root "docs\assets"; New-Item -ItemType Directory -Force $out | Out-Null
function Shot([int]$x, [int]$y, [int]$w, [int]$h, [string]$file) {
  $bmp = New-Object Drawing.Bitmap $w, $h
  [Drawing.Graphics]::FromImage($bmp).CopyFromScreen($x, $y, 0, 0, $bmp.Size)
  $bmp.Save((Join-Path $out $file)); Write-Host "saved $file ${w}x$h"
}

# 1. Settings window: launching a second instance brings the existing one to the front.
Start-Process (Join-Path $root "target\release\voxora.exe"); Start-Sleep 2
$h = (Get-Process voxora | Where-Object { $_.MainWindowTitle -eq "Voxora" } | Select-Object -First 1).MainWindowHandle
if (-not $h) { throw "Voxora main window not found" }
$r = New-Object C+RECT; [void][C]::GetWindowRect($h, [ref]$r)
# Trim the invisible 7 px resize borders Windows 11 adds around the frame.
Shot ($r.L + 7) $r.T ($r.R - $r.L - 14) ($r.B - $r.T - 7) "home.png"

# 2. Overlay pill over a neutral backdrop.
$wa = [Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$bg = New-Object Windows.Forms.Form -Property @{ FormBorderStyle = "None"; BackColor = [Drawing.Color]::FromArgb(30, 32, 44); TopMost = $false; ShowInTaskbar = $false; StartPosition = "Manual" }
$bg.Bounds = New-Object Drawing.Rectangle ($wa.X + $wa.Width / 2 - 400), ($wa.Bottom - 260), 800, 260
$bg.Show(); [Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 300
[C]::keybd_event(0xA5, 0, 1, [UIntPtr]::Zero)
$player = Start-Process (Join-Path $root "target\debug\examples\play_to_device.exe") -ArgumentList "`"$root\bench\audio\jarg-01.wav`" `"CABLE Input`"" -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1800
$vpid = (Get-Process voxora | Select-Object -First 1).Id
$ov = [C]::ByTitle([uint32]$vpid, "Voxora overlay"); [void][C]::GetWindowRect($ov, [ref]$r)
if ($r.R -le $r.L) { throw "overlay window not found" }
$pad = 24
Shot ($r.L - $pad) ($r.T - $pad) ($r.R - $r.L + 2 * $pad) ($r.B - $r.T + 2 * $pad) "overlay.png"
[C]::keybd_event(0x1B, 0, 0, [UIntPtr]::Zero); [C]::keybd_event(0x1B, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 100; [C]::keybd_event(0xA5, 0, 3, [UIntPtr]::Zero)
$player.WaitForExit(); $bg.Close()
