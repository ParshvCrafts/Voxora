// Voxora launch film v2: editorial cards (hard cuts between colour fields, mask-up type) around
// real Chrome captures. Silent by design. Scene timing lives in public/launch.json (also read by
// export-launch-timeline.mjs for QA); every word and UI state on screen is in video/video.md.
import React, { useEffect, useState } from "react";
import {
  AbsoluteFill, Easing, OffthreadVideo, Sequence, continueRender, delayRender, interpolate, staticFile,
  useCurrentFrame, useVideoConfig,
} from "remotion";
import plan from "../public/launch.json";

const INK = "#0d0d0d", CREAM = "#f1ece2", RED = "#ff453a", PLATE_BG = "#18181b";
const OUT = Easing.bezier(0.16, 1, 0.3, 1); // expo-out: snappy, no float
const INOUT = Easing.bezier(0.7, 0, 0.3, 1);

const useFont = () => {
  const [handle] = useState(() => delayRender("Archivo"));
  useEffect(() => {
    const f = new FontFace("Archivo", `url(${staticFile("fonts/Archivo.ttf")}) format("truetype")`, {
      weight: "100 900", stretch: "62% 125%",
    });
    f.load().then((ff) => { (document.fonts as unknown as Set<FontFace>).add(ff); continueRender(handle); });
  }, [handle]);
};

const display = (size: number, color: string, extra: React.CSSProperties = {}): React.CSSProperties => ({
  fontFamily: "Archivo", fontWeight: 900, fontStretch: "125%", fontSize: size, lineHeight: 0.92,
  letterSpacing: "-0.02em", textTransform: "uppercase", color, whiteSpace: "nowrap", ...extra,
});
const label = (size: number, color: string, extra: React.CSSProperties = {}): React.CSSProperties => ({
  fontFamily: "Archivo", fontWeight: 800, fontStretch: "100%", fontSize: size, lineHeight: 1.1,
  textTransform: "uppercase", letterSpacing: "0.01em", color, whiteSpace: "nowrap", ...extra,
});

/** One line that rises out of a mask (line-height < 1, padded so descenders don't clip). */
const Rise: React.FC<{ t: number; at: number; children: React.ReactNode; style?: React.CSSProperties }> = ({ t, at, children, style }) => {
  const p = interpolate(t, [at, at + 0.55], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: OUT });
  return (
    <div style={{ overflow: "hidden", padding: "0.04em 0 0.08em" }}>
      <div style={{ transform: `translateY(${(1 - p) * 110}%)`, ...style }}>{children}</div>
    </div>
  );
};

/** Corner brackets + progress line, no text: ties the scenes together; colour flips with the field. */
const Frame: React.FC<{ color: string; progress: number }> = ({ color, progress }) => {
  const corner = (pos: React.CSSProperties, [x, y]: [string, string]) => (
    <div style={{ position: "absolute", width: 30, height: 30, ...pos, [x]: `3px solid ${color}`, [y]: `3px solid ${color}` }} />
  );
  return (
    <AbsoluteFill style={{ pointerEvents: "none" }}>
      {corner({ left: 36, top: 36 }, ["borderLeft", "borderTop"])}
      {corner({ right: 36, top: 36 }, ["borderRight", "borderTop"])}
      {corner({ left: 36, bottom: 36 }, ["borderLeft", "borderBottom"])}
      {corner({ right: 36, bottom: 36 }, ["borderRight", "borderBottom"])}
      <div style={{ position: "absolute", left: 84, right: 84, bottom: 50, height: 3, background: color, opacity: 0.2 }} />
      <div style={{ position: "absolute", left: 84, bottom: 50, height: 3, width: (1080 - 168) * progress, background: color }} />
    </AbsoluteFill>
  );
};

// ───────────── plates: real footage with an eased camera ─────────────
type Key = { t: number; s: number; cx: number; cy: number }; // s = screen px per plate px
type Seg = { at: number; src: number; rate: number };
const camAt = (keys: Key[], t: number) => {
  const pick = (k: "s" | "cx" | "cy") => {
    for (let i = 0; i < keys.length - 1; i++) {
      const a = keys[i], b = keys[i + 1];
      if (t <= b.t) return interpolate(t, [a.t, b.t], [a[k], b[k]], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: INOUT });
    }
    return keys[keys.length - 1][k];
  };
  return { s: pick("s"), cx: pick("cx"), cy: pick("cy") };
};
const Plate: React.FC<{ file: string; segs: Seg[]; keys: Key[]; dur: number; t: number }> = ({ file, segs, keys, dur, t }) => {
  const { fps } = useVideoConfig();
  const { s, cx, cy } = camAt(keys, t);
  return (
    <AbsoluteFill style={{ background: PLATE_BG }}>
      <div style={{ position: "absolute", left: 0, top: 0, width: 1500, height: 1206, transformOrigin: "0 0",
        transform: `translate(${540 - cx * s}px, ${675 - cy * s}px) scale(${s})` }}>
        {segs.map((g, i) => {
          const end = i + 1 < segs.length ? segs[i + 1].at : dur;
          return (
            <Sequence key={i} from={Math.round(g.at * fps)} durationInFrames={Math.round((end - g.at) * fps)} layout="none">
              <OffthreadVideo src={staticFile(file)} startFrom={Math.round(g.src * fps)} playbackRate={g.rate} muted
                style={{ width: 1500, height: 1206, display: "block" }} />
            </Sequence>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
/** Speed chip: the speech is sped up (and says so); key release to text plays at real speed. */
const Chip: React.FC<{ fast: boolean }> = ({ fast }) => (
  <div style={{ position: "absolute", left: 72, top: 84, display: "flex", alignItems: "center", gap: 16, padding: "14px 28px",
    borderRadius: 999, background: fast ? CREAM : RED, ...label(44, fast ? INK : CREAM) }}>
    {fast ? "3× speed" : <><span style={{ width: 16, height: 16, borderRadius: 8, background: CREAM }} />Real time</>}
  </div>
);

// ───────────── scenes ─────────────
const Hook: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: INK, padding: "0 96px", justifyContent: "center" }}>
    <Rise t={t} at={-0.2}><div style={display(150, CREAM)}>Stop</div></Rise>
    <Rise t={t} at={-0.1}><div style={display(150, CREAM)}>typing</div></Rise>
    <Rise t={t} at={0.5}><div style={label(78, CREAM, { marginTop: 36 })}>to your AI<span style={{ color: RED }}>.</span></div></Rise>
  </AbsoluteFill>
);
const Talk: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: RED, padding: "0 96px", justifyContent: "center" }}>
    <Rise t={t} at={0.0}><div style={display(250, INK)}>Just</div></Rise>
    <Rise t={t} at={0.08}><div style={display(250, INK)}>talk<span style={{ color: CREAM }}>.</span></div></Rise>
  </AbsoluteFill>
);
const Live: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill>
    <Plate file="p1.mp4" dur={8.4} t={t}
      segs={[{ at: 0, src: 1.3, rate: 3 }, { at: 5.15, src: 16.75, rate: 1 }]}
      keys={[
        { t: 0, s: 0.72, cx: 750, cy: 603 }, { t: 0.6, s: 0.72, cx: 750, cy: 603 },
        { t: 1.5, s: 1.36, cx: 750, cy: 1060 }, { t: 2.1, s: 1.36, cx: 750, cy: 1060 }, { t: 2.9, s: 1.9, cx: 850, cy: 1230 }, { t: 4.95, s: 1.9, cx: 850, cy: 1230 }, { t: 5.5, s: 1.6, cx: 750, cy: 1250 }, { t: 6.3, s: 1.6, cx: 750, cy: 1250 },
        { t: 6.95, s: 1.12, cx: 905, cy: 600 }, { t: 8.4, s: 1.12, cx: 905, cy: 600 },
      ]} />
    <Chip fast={t < 5.15} />
    <Caption t={t} from={2.9} to={5.0}>Every word, live<span style={{ color: RED }}>.</span></Caption>
    <Caption t={t} from={5.2} to={6.25}>Let go<span style={{ color: RED }}>.</span></Caption>
  </AbsoluteFill>
);
/** Big line in the dark band under the HUD; rises in, fades out before the camera moves on. */
const Caption: React.FC<{ t: number; from: number; to: number; children: React.ReactNode }> = ({ t, from, to, children }) => {
  if (t < from || t > to) return null;
  const out = interpolate(t, [to - 0.2, to], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <div style={{ position: "absolute", left: 96, right: 96, top: 930, opacity: out }}>
      <Rise t={t} at={from}><div style={display(70, CREAM)}>{children}</div></Rise>
    </div>
  );
};

const RAW = "Hey, um, can you write me a Python script that renames every photo in a folder by the date it was taken? Actually, no, by the date in the file name, and, uh, skip anything that isn't a JPEG.";
const FINAL = "Hey, can you write me a Python script that renames every photo in a folder by the date in the file name, and skip anything that isn't a JPEG?";
// Word indexes (in RAW) that the cleanup removed: "um," / "by the date it was taken? Actually, no," / "uh,".
const STRUCK = new Set([1, 16, 17, 18, 19, 20, 21, 22, 23, 32]);
const Said: React.FC<{ t: number }> = ({ t }) => {
  const words = RAW.split(" ");
  let k = 0;
  return (
    <AbsoluteFill style={{ background: INK, padding: "150px 96px 0" }}>
      <Rise t={t} at={0}><div style={label(48, RED)}>You said</div></Rise>
      <div style={{ marginTop: 40, display: "flex", flexWrap: "wrap", columnGap: "0.28em", rowGap: 6, ...label(60, CREAM, { whiteSpace: "normal", textTransform: "none", lineHeight: 1.12, fontWeight: 700 }) }}>
        {words.map((w, i) => {
          const struck = STRUCK.has(i);
          const at = 0.6 + (struck ? k++ : 0) * 0.06;
          const p = struck ? interpolate(t, [at, at + 0.25], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: OUT }) : 0;
          const inP = interpolate(t, [0.1 + i * 0.012, 0.45 + i * 0.012], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: OUT });
          return (
            <span key={i} style={{ position: "relative", opacity: inP * (1 - 0.55 * p), transform: `translateY(${(1 - inP) * 24}px)`, color: struck && p > 0 ? RED : undefined }}>
              {w}
              {struck && <span style={{ position: "absolute", left: -4, right: -4, top: "52%", height: 7, background: RED, transformOrigin: "left", transform: `scaleX(${p})` }} />}
            </span>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
const Typed: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: CREAM, padding: "150px 96px 0" }}>
    <Rise t={t} at={0}><div style={label(48, RED)}>It typed</div></Rise>
    <div style={{ marginTop: 40, display: "flex", flexWrap: "wrap", columnGap: "0.28em", rowGap: 6, ...label(60, INK, { whiteSpace: "normal", textTransform: "none", lineHeight: 1.12, fontWeight: 700 }) }}>
      {FINAL.split(" ").map((w, i) => {
        const p = interpolate(t, [0.1 + i * 0.02, 0.5 + i * 0.02], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: OUT });
        return <span key={i} style={{ opacity: p, transform: `translateY(${(1 - p) * 24}px)` }}>{w}</span>;
      })}
    </div>
    <Rise t={t} at={1.0} style={{ marginTop: 64 }}>
      <div style={label(48, INK, { opacity: 0.55, textTransform: "none" })}>Into ChatGPT, 1.3 s after letting go</div>
    </Rise>
  </AbsoluteFill>
);
const SayMath: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: RED, padding: "0 96px", justifyContent: "center" }}>
    <Rise t={t} at={0}><div style={display(210, INK)}>Say</div></Rise>
    <Rise t={t} at={0.08}><div style={display(210, INK)}>math<span style={{ color: CREAM }}>.</span></div></Rise>
  </AbsoluteFill>
);
const Katex: React.FC<{ t: number }> = ({ t }) => {
  const band = interpolate(t, [2.65, 3.15], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: OUT });
  return (
    <AbsoluteFill>
      <Plate file="m1.mp4" dur={5.2} t={t}
        segs={[{ at: 0, src: 1.5, rate: 3 }, { at: 2.22, src: 8.15, rate: 1 }]}
        keys={[
          { t: 0, s: 0.885, cx: 750, cy: 560 }, { t: 0.3, s: 0.885, cx: 750, cy: 560 }, { t: 2.2, s: 1.25, cx: 650, cy: 800 }, { t: 2.6, s: 1.25, cx: 650, cy: 800 },
          { t: 3.4, s: 1.95, cx: 1045, cy: 430 }, { t: 5.2, s: 1.95, cx: 1045, cy: 430 },
        ]} />
      <Chip fast={t < 2.22} />
      <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, height: 330, background: INK, padding: "56px 96px 0",
        transform: `translateY(${(1 - band) * 100}%)` }}>
        <div style={label(44, RED)}>Pasted</div>
        <div style={{ marginTop: 18, fontFamily: "Consolas, 'Cascadia Mono', monospace", fontWeight: 700, fontSize: 56, color: CREAM, whiteSpace: "nowrap" }}>
          {"\\int_{0}^{1} x^2 \\, dx"}
        </div>
      </div>
    </AbsoluteFill>
  );
};
const Local: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: INK, padding: "0 96px", justifyContent: "center" }}>
    <Rise t={t} at={0}><div style={display(98, CREAM)}>All on your</div></Rise>
    <Rise t={t} at={0.08}><div style={display(98, CREAM)}>laptop<span style={{ color: RED }}>.</span></div></Rise>
    <div style={{ marginTop: 64 }}>
      {["No cloud.", "No account.", "Free."].map((x, i) => (
        <Rise key={x} t={t} at={0.6 + i * 0.22}><div style={label(66, CREAM, { opacity: 0.8 })}>{x}</div></Rise>
      ))}
    </div>
  </AbsoluteFill>
);
const End: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill style={{ background: CREAM, padding: "0 96px", justifyContent: "center" }}>
    <Rise t={t} at={0}><div style={display(150, INK)}>Voxora<span style={{ color: RED }}>.</span></div></Rise>
    <Rise t={t} at={0.25}><div style={label(56, INK, { marginTop: 44 })}>Free and open source</div></Rise>
    <Rise t={t} at={0.35}><div style={label(56, INK)}>Windows · macOS beta</div></Rise>
    <Rise t={t} at={0.6}><div style={label(52, RED, { marginTop: 72, textTransform: "none" })}>github.com/ParshvCrafts/Voxora</div></Rise>
  </AbsoluteFill>
);

const VIEWS: Record<string, React.FC<{ t: number }>> = { hook: Hook, talk: Talk, live: Live, said: Said, typed: Typed, math: SayMath, katex: Katex, local: Local, end: End };
export const LAUNCH_SECS = plan.scenes.reduce((a, s) => a + s.dur, 0);

export const Launch: React.FC = () => {
  useFont();
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const now = frame / fps;
  let at = 0;
  return (
    <AbsoluteFill style={{ background: INK }}>
      {plan.scenes.map((s) => {
        const start = at;
        at += s.dur;
        const View = VIEWS[s.id];
        const from = Math.round(start * fps), len = Math.round(s.dur * fps);
        const frameColor = s.field === CREAM || s.field === RED ? INK : CREAM;
        return (
          <Sequence key={s.id} from={from} durationInFrames={len}>
            <View t={now - start} />
            <Frame color={frameColor} progress={now / LAUNCH_SECS} />
          </Sequence>
        );
      })}
    </AbsoluteFill>
  );
};
