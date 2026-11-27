# Launch film v2 (2026-11-27)

Job: LinkedIn launch, 4:5 1080x1350, ~29 s, silent (feeds autoplay muted; owner rejected the
music and the cut-up voice). Viewer: students and developers who prompt AI all day.

Claim: talk to your AI instead of typing; Voxora shows your words live and types what you meant.
Rejected: "local dictation" (crowded, invisible), "fast" (a number, not a picture).
Archetype: editorial bold cards (hard cuts between colour fields, mask-up type) + real capture.

## Claims inventory
| Claim on screen | Source |
|---|---|
| Live words in the HUD while speaking | p1.mkv, release build 2f65d00 |
| Raw words "Hey, um, ... taken? Actually, no, ... uh, ..." | history raw_text of that take |
| Pasted "Hey, can you write me a Python script ... JPEG?" | history final_text, clips/p1.result.txt |
| Release to text 1.27 s in that take | history total_ms 1272, frames 16.75 -> 18.02 s |
| Spoken integral -> `\int_{0}^{1} x^2 \, dx`, rendered by KaTeX | m1.mkv, clips/m1.result.txt |
| Runs on your laptop, no account, free, MIT | README, D-log |
Speech parts play at 3x, labelled "3x"; key release to paste plays at 1x, labelled.
Voice is synthesized (Kokoro) into a virtual mic; silent film, so no voice is heard.

## Beats
| t | beat | picture | text |
|---|---|---|---|
| 0-2.4 | hook | ink field, mask-up type | STOP TYPING / TO YOUR AI. |
| 2.4-3.7 | turn | red field | JUST TALK. |
| 3.7-8.6 | live | ChatGPT in Chrome, camera to HUD, 3x | chip 3x |
| 8.6-11.1 | paste | same, 1x, camera to composer | chip REAL TIME |
| 11.1-14.1 | said | ink field, raw words, strikes draw | YOU SAID |
| 14.1-16.5 | typed | cream field, final words | IT TYPED |
| 16.5-17.9 | card | red field | SAY MATH. |
| 17.9-22.5 | math | KaTeX demo, 3x then 1x, push into render | GET LATEX. |
| 22.5-25.5 | turn | ink | ALL ON YOUR LAPTOP. / NO CLOUD. NO ACCOUNT. FREE. |
| 25.5-29.7 | end | cream | VOXORA / github.com/ParshvCrafts/Voxora |

Deliverables: MP4 (poster frame 0), GIF loop, SRT not needed (all text is on screen), credits.md.
