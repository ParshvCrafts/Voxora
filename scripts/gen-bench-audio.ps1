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
foreach ($c in (Get-Content $Cases -Raw | ConvertFrom-Json)) {
    $synth.SelectVoice($voices[$i % $voices.Count]); $i++
    $synth.SetOutputToWaveFile((Join-Path $OutDir "$($c.id).wav"), $fmt)
    $synth.Speak($c.spoken)
}
$synth.SetOutputToNull()
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
