// product-video template: every visual decision comes from public/film.json (see README.md).
// Scenes play back to back. "card": sentence card with a line-mask reveal, a hold, an upward exit.
// "plate": the real capture (clean plate, no cursor) in a window; camera keyframes and a synthetic
// cursor driven by the capture's events.json. Old scene exits before the new one enters (no crossfade).
import React from "react";
import {
  AbsoluteFill, Audio, Easing, OffthreadVideo, Sequence, interpolate, staticFile, useCurrentFrame, useVideoConfig,
} from "remotion";

type Pt = { x: number; y: number };
type Ev = { t: number; do: string; point?: Pt; box?: { x: number; y: number; w: number; h: number } };
type Cam = { t: number; zoom: number; cx?: number; cy?: number; focus?: number };
export type Scene =
  | { id: string; type: "card"; dur: number; lines: string[]; sub?: string; accentWord?: string }
  | { id: string; type: "plate"; dur: number; from: number; camera?: Cam[]; caption?: string; cursor?: boolean; note?: string; hardIn?: boolean; hardOut?: boolean };
export type FilmData = {
  w: number; h: number; fps: number;
  palette: { bg: string; ink: string; accent: string; window?: string };
  font: string; weight?: number;
  plate?: { file: string; cssW: number; cssH: number };
  // Window aspect (w/h). Default = the plate's own. For 4:5 or 9:16 output use ~1.0 so the UI is
  // large enough to read on a phone; the plate is cropped (cover) and the camera picks the region.
  window?: { aspect?: number };
  events?: Ev[];
  scenes: Scene[];
  audio?: { file: string; at: number; volume?: number }[];
};

const ease = Easing.bezier(0.23, 1, 0.32, 1); // expo-like out: snappy, no float
const inOut = Easing.bezier(0.65, 0, 0.35, 1);

const Card: React.FC<{ s: Extract<Scene, { type: "card" }>; d: FilmData; last?: boolean }> = ({ s, d, last }) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const end = s.dur * fps;
  // The final card holds fully static so the feed viewer can read it (owner rule).
  const exit = last ? 0 : interpolate(f, [end - 0.35 * fps, end], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: inOut });
  return (
    <AbsoluteFill style={{ background: d.palette.bg, padding: "0 96px", justifyContent: "center" }}>
      <div style={{ transform: `translateY(${-exit * 120}px)`, opacity: 1 - exit }}>
        {s.lines.map((line, i) => {
          const p = interpolate(f, [(0.1 + i * 0.08) * fps, (0.7 + i * 0.08) * fps], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
          return (
            <div key={i} style={{ overflow: "hidden", paddingBottom: 26 }}>
              <div style={{ transform: `translateY(${(1 - p) * 110}%)`, font: `${d.weight ?? 700} 92px/1.0 ${d.font}`, color: d.palette.ink, letterSpacing: "-0.01em" }}>
                {s.accentWord && line.includes(s.accentWord)
                  ? <>{line.split(s.accentWord)[0]}<span style={{ color: d.palette.accent }}>{s.accentWord}</span>{line.split(s.accentWord)[1]}</>
                  : line}
              </div>
            </div>
          );
        })}
        {s.sub && (
          <div style={{ marginTop: 36, font: `600 48px/1.25 ${d.font}`, color: "rgba(245,245,247,0.72)",
            opacity: interpolate(f, [0.6 * fps, 1.0 * fps], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }) }}>{s.sub}</div>
        )}
      </div>
    </AbsoluteFill>
  );
};

// Camera keyframes in scene-local seconds; zoom 1 = whole plate; focus = index into events (centre on its box).
function camAt(t: number, cams: Cam[], ev: Ev[], cssW: number, cssH: number) {
  const pts = cams.map(c => {
    const e = c.focus !== undefined ? ev[c.focus] : undefined;
    const b = e?.box;
    return { t: c.t, zoom: c.zoom, cx: b ? b.x + b.w / 2 : c.cx ?? cssW / 2, cy: b ? b.y + b.h / 2 : c.cy ?? cssH / 2 };
  });
  if (!pts.length) return { zoom: 1, cx: cssW / 2, cy: cssH / 2 };
  if (t <= pts[0].t) return pts[0];
  for (let i = 0; i < pts.length - 1; i++) {
    const a = pts[i], b = pts[i + 1];
    if (t <= b.t) {
      const p = inOut((t - a.t) / (b.t - a.t));
      return { zoom: a.zoom + (b.zoom - a.zoom) * p, cx: a.cx + (b.cx - a.cx) * p, cy: a.cy + (b.cy - a.cy) * p };
    }
  }
  return pts[pts.length - 1];
}

const Plate: React.FC<{ s: Extract<Scene, { type: "plate" }>; d: FilmData }> = ({ s, d }) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const P = d.plate!;
  const ev = d.events ?? [];
  const t = f / fps, tp = s.from + t; // tp = time in the plate
  const winW = d.w - 2 * 48, winH = winW / (d.window?.aspect ?? P.cssW / P.cssH);
  const scale = Math.max(winW / P.cssW, winH / P.cssH); // cover: plate CSS px -> window px at zoom 1
  // Keep the camera inside the plate: clamp the centre so edges never show.
  const c = camAt(t, s.camera ?? [], ev, P.cssW, P.cssH);
  const z = Math.max(1, c.zoom);
  const half = { x: winW / (2 * scale * z), y: winH / (2 * scale * z) }; // visible half-extent, CSS px
  const cx = Math.min(P.cssW - half.x, Math.max(half.x, c.cx)), cy = Math.min(P.cssH - half.y, Math.max(half.y, c.cy));
  const toWin = (p: Pt) => ({ x: (p.x - cx) * scale * z + winW / 2, y: (p.y - cy) * scale * z + winH / 2 });
  // hardIn/hardOut: a jump cut inside one recording (no slide or fade at that boundary).
  const enter = s.hardIn ? 1 : interpolate(f, [0, 0.25 * fps], [0, 1], { extrapolateRight: "clamp", easing: ease });
  const exit = s.hardOut ? 0 : interpolate(f, [s.dur * fps - 0.25 * fps, s.dur * fps], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: inOut });

  // Cursor: eased travel between logged action points (0.6 s before each), ripple on clicks.
  const acts = ev.filter(e => e.point && (e.do === "click" || e.do === "hover" || e.do === "type"));
  let cur: Pt | null = null, ripple: { p: Pt; age: number } | null = null;
  if (s.cursor !== false && acts.length) {
    const next = acts.find(e => e.t >= tp), prev = [...acts].reverse().find(e => e.t < tp);
    if (prev && next) {
      const p = inOut(Math.min(1, Math.max(0, (tp - Math.max(prev.t, next.t - 0.6)) / Math.min(0.6, next.t - prev.t))));
      cur = { x: prev.point!.x + (next.point!.x - prev.point!.x) * p, y: prev.point!.y + (next.point!.y - prev.point!.y) * p };
    } else if (prev && tp - prev.t > 1.2) cur = null; // after the last action the cursor leaves, so it never "points" at content scrolling under it
    else cur = (prev ?? next)!.point!;
    const lastClick = [...acts].reverse().find(e => e.do === "click" && e.t <= tp && tp - e.t < 0.45);
    if (lastClick) ripple = { p: lastClick.point!, age: (tp - lastClick.t) / 0.45 };
  }
  const cw = cur && toWin(cur), rw = ripple && toWin(ripple.p);
  return (
    <AbsoluteFill style={{ background: d.palette.bg, alignItems: "center", justifyContent: "center" }}>
      {/* Fixed width: a long caption must wrap, never widen the wrapper and push the window off-centre. */}
      <div style={{ width: winW, transform: `translateY(${(1 - enter) * 80 - exit * 120}px)`, opacity: enter * (1 - exit) }}>
        {s.note && (
          <div style={{ marginBottom: 20, font: `500 44px/1.1 ${d.font}`, color: "rgba(245,245,247,0.78)" }}>{s.note}</div>
        )}
        <div style={{ width: winW, height: winH, borderRadius: 18, overflow: "hidden", position: "relative",
          background: d.palette.window ?? "#000", boxShadow: "0 30px 80px rgba(0,0,0,.45), 0 0 0 1px rgba(255,255,255,.08)" }}>
          <div style={{ position: "absolute", width: P.cssW * scale, height: P.cssH * scale, transformOrigin: "0 0",
            transform: `translate(${winW / 2}px, ${winH / 2}px) scale(${z}) translate(${-cx * scale}px, ${-cy * scale}px)` }}>
            <OffthreadVideo src={staticFile(P.file)} startFrom={Math.round(s.from * fps)} muted style={{ width: P.cssW * scale, height: P.cssH * scale }} />
          </div>
          {rw && ripple && (
            <div style={{ position: "absolute", left: rw.x, top: rw.y, width: 0, height: 0 }}>
              <div style={{ position: "absolute", left: -24 * (0.4 + ripple.age), top: -24 * (0.4 + ripple.age),
                width: 48 * (0.4 + ripple.age), height: 48 * (0.4 + ripple.age), borderRadius: "50%",
                border: `3px solid ${d.palette.accent}`, opacity: 1 - ripple.age }} />
            </div>
          )}
          {cw && (
            <svg style={{ position: "absolute", left: cw.x - 3, top: cw.y - 2 }} width="34" height="40" viewBox="0 0 17 20">
              <path d="M1 1 L1 16 L5 12 L8 19 L11 18 L8 11 L14 11 Z" fill="#fff" stroke="#111" strokeWidth="1.2" strokeLinejoin="round" />
            </svg>
          )}
        </div>
        {s.caption && (
          <div style={{ marginTop: 44, padding: "0 4px", font: `${d.weight ?? 700} 56px/1.15 ${d.font}`, color: d.palette.ink,
            opacity: interpolate(f, [0.3 * fps, 0.8 * fps], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }) }}>{s.caption}</div>
        )}
      </div>
    </AbsoluteFill>
  );
};

export const Film: React.FC<{ data: FilmData }> = ({ data }) => {
  const { fps } = useVideoConfig();
  let at = 0;
  return (
    <AbsoluteFill style={{ background: data.palette.bg }}>
      {data.scenes.map(s => {
        const from = Math.round(at * fps), dur = Math.round(s.dur * fps);
        at += s.dur;
        return (
          <Sequence key={s.id} from={from} durationInFrames={dur} premountFor={fps}>
            {s.type === "card" ? <Card s={s} d={data} last={s === data.scenes[data.scenes.length - 1]} /> : <Plate s={s} d={data} />}
          </Sequence>
        );
      })}
      {(data.audio ?? []).map((a, i) => (
        <Sequence key={`a${i}`} from={Math.round(a.at * fps)}>
          <Audio src={staticFile(a.file)} volume={a.volume ?? 1} />
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};
