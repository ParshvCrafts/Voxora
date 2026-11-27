// Export public/film.json as timeline.json (video-director schema) for qa_media.py / check_timeline.py.
// Usage: node export-timeline.mjs [public/film.json] [out/timeline.json]
// Holds (calibrated on a real render 2026-11-26): eased card text looks still ~0.4 s after the scene
// starts, so a card's hold is [start+0.4, end-0.35] (the final card holds to the end). Plates: the
// whole scene minus enter/exit (static UI while the app responds is intended). Each boundary is a
// deliberate dip through the background (old out, new in), declared as a hold so its flat frames
// are not defects. Scene changes are animated, not hard cuts, so no planned cuts are listed.
import fs from "node:fs";
import path from "node:path";
const [src = "public/film.json", out = "out/timeline.json"] = process.argv.slice(2);
const d = JSON.parse(fs.readFileSync(src, "utf8"));
let at = 0;
const scenes = [], texts = [], holds = [];
const r = v => +v.toFixed(3);
d.scenes.forEach((s, i) => {
  const start = at, end = at + s.dur, last = i === d.scenes.length - 1;
  const hold = s.type === "card" ? [start + 0.4, last ? end : end - 0.35] : [start + 0.5, end - 0.35];
  scenes.push({ id: s.id, start: r(start), end: r(end), holds: [hold.map(r)] });
  if (s.type === "card") texts.push({ id: s.id, text: s.lines.join(" "), from: r(hold[0]), to: r(hold[1]),
    role: i === 0 ? "hook" : null });
  if (i > 0) holds.push([r(start - 0.4), r(start + 0.6)]);
  at = end;
});
fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, JSON.stringify({
  meta: { w: d.w, h: d.h, fps: d.fps, dur: r(at), clock: d.audio?.length ? "voice" : "reads", loop: false },
  scenes, texts, holds,
}, null, 2));
console.log(`${out}: ${scenes.length} scenes, ${at.toFixed(2)} s`);
