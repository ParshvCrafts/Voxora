# product-video Remotion template

One data file, `public/film.json`, drives the whole film. Copy this folder into the project
(e.g. `video/`), `npm install`, put the capture outputs in `public/`, write `film.json`, render.

```bash
cp -r ~/.claude/skills/product-video/template ./video && cd video && npm install
cp <capture>/plate.mp4 public/            # from scripts/capture.mjs (clean plate, no cursor)
npx remotion studio                       # preview
npx remotion render Film out/film.mp4 --codec=h264 --crf=18 --concurrency=4
node export-timeline.mjs public/film.json out/timeline.json
python ~/.claude/skills/product-video/scripts/qa_media.py out/film.mp4 --timeline out/timeline.json
```
Deliver: bake the strongest settled frame in as frame 0 (the feed thumbnail) and keep BT.709 tags:
```bash
ffmpeg -ss 10.5 -i out/film.mp4 -frames:v 1 out/poster.png
ffmpeg -i out/film.mp4 -framerate 30 -i out/poster.png -filter_complex "[1:v]format=yuv420p[p];[0:v]trim=start_frame=1,setpts=PTS-STARTPTS[r];[p][r]concat=n=2:v=1,setparams=range=tv:colorspace=bt709:color_primaries=bt709:color_trc=bt709[o]" -map "[o]" -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -color_range tv -colorspace bt709 -color_primaries bt709 -color_trc bt709 -movflags +faststart out/final.mp4
```
Proven end to end 2026-11-26 on the live Interlace app (17.8 s, 4:5): capture 327/327 frames, render
2 min, qa_media 0 FAIL. Lessons now built in: Remotion's default output was full-range yuvj420p
(config fixes it); the last card must not exit; a caption wider than the window pushed it off-centre
(wrapper width is fixed); the cursor leaves 1.2 s after the last action.

## film.json
```json
{
  "w": 1080, "h": 1350, "fps": 30,
  "palette": { "bg": "#080808", "ink": "#e8e2d9", "accent": "#c9a96e", "window": "#0b0b0b" },
  "font": "Georgia, serif", "weight": 700,
  "plate": { "file": "plate.mp4", "cssW": 1440, "cssH": 900 },
  "window": { "aspect": 1.0 },
  "events": [ ...copy "events" from capture events.json... ],
  "scenes": [
    { "id": "hook", "type": "card", "dur": 2.6, "lines": ["Typos in."], "sub": "blak lether jaket" },
    { "id": "proof", "type": "plate", "dur": 7, "from": 1.0, "caption": "Real leather jackets, all under £150",
      "camera": [ { "t": 0, "zoom": 1 }, { "t": 1.2, "zoom": 1.7, "focus": 3 }, { "t": 4.5, "zoom": 1.25, "cy": 620 } ] },
    { "id": "end", "type": "card", "dur": 4, "lines": ["Interlace"], "sub": "Built by ...", "accentWord": "Interlace" }
  ],
  "audio": [ { "file": "vo/hook.wav", "at": 0.2 } ]
}
```
- `card`: lines reveal with a mask (one line = one read), optional `sub` and one `accentWord`; holds,
  then exits upward before the next scene enters (no crossfades).
- `plate`: plays the plate from `from` seconds. `camera` keyframes are scene-local; `zoom` 1 = whole
  plate; `focus` = index into `events` (centres on that element's box) or give `cx`/`cy` in CSS px.
  Keep zoom <= plate DSF x 0.9. The camera never shows past the plate edge. Cursor and click
  ripples come from the events (`"cursor": false` to hide).
- `window.aspect`: omit for 16:9 output; use ~1.0 for 4:5 and 9:16 so UI reads on a phone (the plate
  is cropped to cover the window; aim the camera with `cx`/`cy`). Typed text needs zoom ~2 to reach
  44 px from a 1440-wide capture.
- Sizes: card lines 92 px, sub and captions 48-56 px at 1080 wide (owner floor: >= 44 px).
- Fonts: system fonts render as-is; for web fonts use `@remotion/google-fonts` in `Film.tsx`.
