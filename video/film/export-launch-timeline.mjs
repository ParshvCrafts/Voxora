// public/launch.json -> out/launch.timeline.json for qa_media.py (scene holds are scene-relative there).
import fs from "node:fs";
const d = JSON.parse(fs.readFileSync("public/launch.json", "utf8"));
const r = (v) => +v.toFixed(3);
let at = 0;
const scenes = [], texts = [];
for (const s of d.scenes) {
  const holds = (s.holds || []).map(([a, b]) => [r(at + a), r(at + b)]);
  scenes.push({ id: s.id, start: r(at), end: r(at + s.dur), holds, ...(s.field && !["live", "katex"].includes(s.id) ? { field: s.field } : {}) });
  if (s.text) texts.push({ id: s.id, text: s.text, from: holds[0][0], to: holds[0][1], role: s.id === "hook" ? "hook" : null });
  at += s.dur;
}
const cuts = scenes.slice(1).map((s) => s.start);
fs.mkdirSync("out", { recursive: true });
fs.writeFileSync("out/launch.timeline.json", JSON.stringify({ meta: { w: d.w, h: d.h, fps: d.fps, dur: r(at), clock: "reads", loop: false }, scenes, texts, cuts }, null, 2));
console.log(`out/launch.timeline.json: ${scenes.length} scenes, ${at.toFixed(2)} s`);
