# Demo video

A 4:5 launch video built from real recordings of the app. Every product shot is the real
Voxora window, the real HUD and real pasted output; the only staged part is the voice, which is
synthesized and labelled on screen.

## Re-capture (needs the PC to yourself for about a minute per clip)

1. Generate the speech: `python ~/.claude/skills/product-video/scripts/tts.py speech/script.json speech/out --engine kokoro`
   (Kokoro-82M, Apache-2.0), then convert each file to 16 kHz 16-bit mono in `speech/out16/` and
   prepend 1.2 s of faint pink noise as `<id>.padded.wav` (the virtual cable sends exact digital
   silence until the player starts, which a real mic never does, and that trips the "No sound
   from the mic" warning).
2. Run the release build with `VOXORA_TEST_INJECTED=1`, mic set to VB-Audio "CABLE Output",
   main window open on Home.
3. `powershell -File video/capture-clips.ps1 -Clip math|corr|code -Out "<cable playback name>"`.
   Each run prints what Voxora actually typed; a take is used only if that text is right.
   The script aborts (Esc, nothing pasted) if Voxora loses focus.

Lines were picked by first running candidate audio through `bench_eval` offline and keeping only
lines that passed (spoken "camel case ..." identifiers did not, see docs/ROADMAP.md).

## Render

```
cd video/film && npm install
# trim clips into public/plate.mp4 (offsets are the scene "from" values in public/film.json)
npx remotion render Film out/film.mp4 --codec=h264 --crf=18
```

Audio (dictation audio from the clips plus music) is mixed with ffmpeg afterwards; see
`credits.md` for sources and licences.

### Audio mix (before `film/finish.sh`)

Scene starts in the film: math 2.6 s, correction 15.1 s, list 31.5 s (clip trims 0.9-10.8,
0.9-14.7 and 2.3-21.5 s). Music is ducked under the dictation voice, then normalized to
-14 LUFS with a -3 dBTP ceiling (AAC adds about 1 dB):

```
ffmpeg -i clips/math.mkv -i clips/corr.mkv -i clips/code.mkv -i music/Wallpaper.mp3 -filter_complex "[0:a]atrim=0.9:10.8,asetpts=PTS-STARTPTS,aresample=48000,pan=stereo|c0=c0|c1=c0,adelay=2600|2600[a1];[1:a]atrim=0.9:14.7,asetpts=PTS-STARTPTS,aresample=48000,pan=stereo|c0=c0|c1=c0,adelay=15100|15100[a2];[2:a]atrim=2.3:21.5,asetpts=PTS-STARTPTS,aresample=48000,pan=stereo|c0=c0|c1=c0,adelay=31500|31500[a3];[a1][a2][a3]amix=inputs=3:normalize=0:duration=longest,apad=whole_dur=58.5[speech];[3:a]aresample=48000,atrim=0:58.5,asetpts=PTS-STARTPTS,afade=t=in:d=1,afade=t=out:st=56:d=2.5,volume=0.4[mus];[speech]asplit[sp1][sp2];[mus][sp1]sidechaincompress=threshold=0.03:ratio=10:attack=15:release=500[duck];[duck][sp2]amix=inputs=2:normalize=0:duration=first,atrim=0:58.5[mix]" -map "[mix]" -c:a pcm_s16le film/out/mix.wav
# pass 1: measure; pass 2: loudnorm=I=-14:TP=-3:LRA=11 with the measured_* values and linear=true -> film/out/mixn.wav
```

Outputs in `film/out/`: `voxora-launch.mp4` (master, captions burned in), `voxora-launch.srt`,
`voxora-demo.gif` (README loop), `timeline.json`, QA sheets.
