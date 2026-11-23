// The floating pill. Never takes focus (Win32 WS_EX_NOACTIVATE is set by the Rust side); it only
// renders state pushed from the engine.
import { StrictMode, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { AlertTriangle, Check, Info, Lock, XCircle } from "lucide-react";
import { on } from "./lib/api";
import "./styles/overlay.css";

type Kind = "listening" | "processing" | "done" | "notice" | "warning" | "error" | "hidden";
interface OverlayState {
  kind: Kind;
  mode: string;
  message: string | null;
  hands_free: boolean;
}

const BARS = 14;

function Pill() {
  const [state, setState] = useState<OverlayState>({ kind: "hidden", mode: "CLEAN", message: null, hands_free: false });
  const [levels, setLevels] = useState<number[]>(() => Array(BARS).fill(0));
  const latest = useRef(0);

  useEffect(() => on<OverlayState>("overlay", (s) => {
    setState(s);
    if (s.kind === "listening" && !s.hands_free) setLevels(Array(BARS).fill(0));
  }), []);
  useEffect(() => on<number>("level", (l) => (latest.current = l)), []);

  // Sample the latest level at a steady 30 fps so the bars scroll smoothly.
  useEffect(() => {
    if (state.kind !== "listening") return;
    const t = setInterval(() => setLevels((prev) => [...prev.slice(1), latest.current]), 33);
    return () => clearInterval(t);
  }, [state.kind]);

  const visible = state.kind !== "hidden";
  return (
    <div className={`pill ${visible ? "in" : "out"} ${state.kind}`} role="status" aria-live="polite">
      {state.kind === "listening" && (
        <>
          <span className="dot" />
          {state.hands_free && <Lock size={12} className="lock" aria-label="hands-free" />}
          <span className="bars" aria-hidden>
            {levels.map((l, i) => (
              <span key={i} style={{ height: `${Math.max(3, Math.min(22, 3 + l * 26))}px` }} />
            ))}
          </span>
        </>
      )}
      {state.kind === "processing" && (
        <span className="wave" aria-label="processing">
          <span />
          <span />
          <span />
        </span>
      )}
      {state.kind === "done" && <Check size={16} className="ok" />}
      {state.kind === "notice" && <Info size={15} className="info" />}
      {state.kind === "warning" && <AlertTriangle size={15} className="warn" />}
      {state.kind === "error" && <XCircle size={15} className="err" />}
      {state.message && <span className="msg">{state.message}</span>}
      {!state.message && state.kind !== "done" && <span className="badge">{state.mode}</span>}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Pill />
  </StrictMode>,
);
