#!/usr/bin/env bash
# After `npx remotion render Film out/film.mp4`: poster as frame 0, normalized mix, final MP4,
# README loop GIF, QA. Needs out/mixn.wav (see ../README.md for the mix command).
set -euo pipefail
cd "$(dirname "$0")/out"
POSTER_T=${POSTER_T:-11.6}   # math result, zoomed: the strongest settled frame
ffmpeg -loglevel error -y -ss "$POSTER_T" -i film.mp4 -frames:v 1 poster.png
ffmpeg -loglevel error -y -i film.mp4 -framerate 30 -i poster.png -i mixn.wav -filter_complex \
  "[1:v]format=yuv420p[p];[0:v]trim=start_frame=1,setpts=PTS-STARTPTS[r];[p][r]concat=n=2:v=1,setparams=range=tv:colorspace=bt709:color_primaries=bt709:color_trc=bt709[o]" \
  -map "[o]" -map 2:a -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -color_range tv -colorspace bt709 \
  -color_primaries bt709 -color_trc bt709 -c:a aac -b:a 192k -ar 48000 -shortest -movflags +faststart voxora-launch.mp4
# README loop: the math beat, from the card to the LaTeX result.
ffmpeg -loglevel error -y -ss 0 -t 12.4 -i film.mp4 -vf "fps=12,scale=540:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4" voxora-demo.gif
python ~/.claude/skills/product-video/scripts/qa_media.py voxora-launch.mp4 --timeline timeline.json --has-poster
ls -la voxora-launch.mp4 voxora-demo.gif
