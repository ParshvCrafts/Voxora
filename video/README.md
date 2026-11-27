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
