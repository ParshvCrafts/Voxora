#!/usr/bin/env bash
# After `npx remotion render Launch out/launch.mp4 --codec=h264 --crf=16`: bake the poster in as
# frame 0, write the silent master, the README loop GIF, and run QA.
set -euo pipefail
cd "$(dirname "$0")"
node export-launch-timeline.mjs
cd out
POSTER_T=${POSTER_T:-8.0}   # live words in the HUD + "EVERY WORD, LIVE."
ffmpeg -loglevel error -y -ss "$POSTER_T" -i launch.mp4 -frames:v 1 launch-poster.png
ffmpeg -loglevel error -y -i launch.mp4 -framerate 30 -i launch-poster.png -filter_complex \
  "[1:v]format=yuv420p[p];[0:v]trim=start_frame=1,setpts=PTS-STARTPTS[r];[p][r]concat=n=2:v=1,setparams=range=tv:colorspace=bt709:color_primaries=bt709:color_trc=bt709[o]" \
  -map "[o]" -an -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -color_range tv -colorspace bt709 \
  -color_primaries bt709 -color_trc bt709 -movflags +faststart voxora-launch-v2.mp4
# README loop: from the live words to the pasted prompt.
ffmpeg -loglevel error -y -ss 5.6 -t 7.2 -i launch.mp4 -vf "fps=12,scale=540:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4" voxora-demo-v2.gif
python ~/.claude/skills/product-video/scripts/qa_media.py voxora-launch-v2.mp4 --timeline launch.timeline.json --has-poster
ls -la voxora-launch-v2.mp4 voxora-demo-v2.gif
