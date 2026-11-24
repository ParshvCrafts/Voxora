# Synthesizes bench/cases.json into 16 kHz mono WAVs with the built-in Windows voices, so the
# benchmark runs end to end (audio -> ASR -> rewrite) without a human at the microphone.
# Synthetic speech is cleaner than a real mic: treat the WER as a lower bound.
param(
    [string]$Cases = "$PSScriptRoot\..\bench\cases.json",
    [string]$OutDir = "$PSScriptRoot\..\bench\audio"
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
New-Item -ItemType Directory -Force $OutDir | Out-Null
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000,
    [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$voices = @($synth.GetInstalledVoices() | Where-Object Enabled | ForEach-Object { $_.VoiceInfo.Name })
$i = 0
foreach ($c in (Get-Content $Cases -Raw -Encoding UTF8 | ConvertFrom-Json)) {
    if (-not $c.spoken) { continue }  # audio-only cases (noise) are generated below
    $synth.SelectVoice($voices[$i % $voices.Count]); $i++
    $synth.SetOutputToWaveFile((Join-Path $OutDir "$($c.id).wav"), $fmt)
    # "[pause 3500]" in the text becomes a real 3.5 s gap in the audio.
    $p = New-Object System.Speech.Synthesis.PromptBuilder
    foreach ($part in [regex]::Split($c.spoken, '(\[pause \d+\])')) {
        if ($part -match '^\[pause (\d+)\]$') { $p.AppendBreak([TimeSpan]::FromMilliseconds([int]$Matches[1])) }
        elseif ($part.Trim()) { $p.AppendText($part) }
    }
    $synth.Speak($p)
}
$synth.SetOutputToNull()
function Write-Wav([string]$path, [byte[]]$pcm) {
    $h = [byte[]](0x52,0x49,0x46,0x46) + [BitConverter]::GetBytes([int](36 + $pcm.Length)) +
        [byte[]](0x57,0x41,0x56,0x45,0x66,0x6D,0x74,0x20) + [BitConverter]::GetBytes([int]16) +
        [BitConverter]::GetBytes([int16]1) + [BitConverter]::GetBytes([int16]1) +
        [BitConverter]::GetBytes([int]16000) + [BitConverter]::GetBytes([int]32000) +
        [BitConverter]::GetBytes([int16]2) + [BitConverter]::GetBytes([int16]16) +
        [byte[]](0x64,0x61,0x74,0x61) + [BitConverter]::GetBytes([int]$pcm.Length)
    [IO.File]::WriteAllBytes($path, $h + $pcm)
}
# 4 s of synthetic "keyboard clatter": low room hiss plus ~9 key clicks per second
# (short decaying noise bursts). No speech: Voxora must insert nothing.
$rng = New-Object Random 42
$n = 16000 * 4; $pcm = New-Object byte[] ($n * 2); $v = New-Object double[] $n
for ($k = 0; $k -lt $n; $k++) { $v[$k] = ($rng.NextDouble() - 0.5) * 300 }
for ($t = 800; $t -lt $n - 1200; $t += 1200 + $rng.Next(1400)) {
    $amp = 4000 + $rng.Next(5000)
    for ($k = 0; $k -lt 900; $k++) { $v[$t + $k] += ($rng.NextDouble() - 0.5) * 2 * $amp * [Math]::Exp(-$k / 120.0) }
}
for ($k = 0; $k -lt $n; $k++) {
    $s = [int16][Math]::Max(-32768, [Math]::Min(32767, $v[$k]))
    $b = [BitConverter]::GetBytes($s); $pcm[2 * $k] = $b[0]; $pcm[2 * $k + 1] = $b[1]
}
Write-Wav (Join-Path $OutDir "noise-4s.wav") $pcm
# 5 s of silence for the hallucination test.
$silence = New-Object byte[] (16000 * 2 * 5)
$header = [byte[]](0x52,0x49,0x46,0x46) + [BitConverter]::GetBytes([int](36 + $silence.Length)) +
    [byte[]](0x57,0x41,0x56,0x45,0x66,0x6D,0x74,0x20) + [BitConverter]::GetBytes([int]16) +
    [BitConverter]::GetBytes([int16]1) + [BitConverter]::GetBytes([int16]1) +
    [BitConverter]::GetBytes([int]16000) + [BitConverter]::GetBytes([int]32000) +
    [BitConverter]::GetBytes([int16]2) + [BitConverter]::GetBytes([int16]16) +
    [byte[]](0x64,0x61,0x74,0x61) + [BitConverter]::GetBytes([int]$silence.Length)
[IO.File]::WriteAllBytes((Join-Path $OutDir "silence-5s.wav"), $header + $silence)
Write-Host "Wrote $i cases + silence to $OutDir using voices: $($voices -join ', ')"
