import { useEffect, useState } from "react";
import { Pause, Play, Sparkles } from "lucide-react";
import { useApp } from "../App";
import { StatusPill } from "../components/ui";
import { api, MODES, modeLabel, on, type Dictation, type Stats } from "../lib/api";

const fmtMs = (v: number | null) => (v == null ? "—" : v >= 1000 ? `${(v / 1000).toFixed(2)} s` : `${v} ms`);

export default function Home({ go }: { go: (page: string) => void }) {
  const { settings, status, update, toast } = useApp();
  const [stats, setStats] = useState<Stats | null>(null);
  const [recent, setRecent] = useState<Dictation[]>([]);
  const key = settings.hotkey.dictation_key.replace(/^Right/, "Right ");

  useEffect(() => {
    const load = () => {
      api.stats().then(setStats);
      api.history("", 3).then(setRecent);
    };
    load();
    return on("history", load);
  }, []);

  useEffect(() => {
    if (!settings.onboarded) update((s) => (s.onboarded = true));
  }, [settings.onboarded, update]);

  return (
    <div className="page">
      <section className="card hero">
        <div>
          <h1>Hold {key} and speak.</h1>
          <p className="sub">
            Release to insert clean text wherever your cursor is. Tap once for hands-free; tap again (or pause) to finish.{" "}
            <kbd>Esc</kbd> cancels.
          </p>
          <div className="row" style={{ marginTop: 14, flexWrap: "wrap", gap: 8 }}>
            <StatusPill label="Speech" value={status.asr} />
            <StatusPill label="Rewrite" value={status.llm} />
            {status.gpu && <span className="pill">{status.gpu}</span>}
            <button
              className="btn ghost"
              onClick={() => api.setPaused(!status.paused).then(() => api.getStatus()).then((s) => toast(s.paused ? "Paused" : "Listening for the hotkey"))}
            >
              {status.paused ? <Play size={15} /> : <Pause size={15} />}
              {status.paused ? "Resume" : "Pause"}
            </button>
          </div>
        </div>
        <div className="keycap" aria-hidden>
          {key}
        </div>
      </section>

      <section className="grid-4">
        {[
          ["Dictations", stats?.count ?? 0],
          ["Words dictated", stats?.words_total ?? 0],
          ["Typical latency (P50)", fmtMs(stats?.p50_total_ms ?? null)],
          ["Slow case (P95)", fmtMs(stats?.p95_total_ms ?? null)],
        ].map(([lbl, num]) => (
          <div className="card stat" key={lbl}>
            <div className="num">{num}</div>
            <div className="lbl">{lbl}</div>
          </div>
        ))}
      </section>

      <section className="card">
        <div className="row between">
          <h2 style={{ margin: 0 }}>Default mode</h2>
          <span className="muted" style={{ fontSize: 12.5 }}>
            Cycle anytime with <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>{settings.hotkey.cycle_mode_key}</kbd>
          </span>
        </div>
        <div className="seg" style={{ margin: "12px 0 8px" }} role="radiogroup" aria-label="Default mode">
          {MODES.map((m) => (
            <button
              key={m.id}
              role="radio"
              aria-checked={settings.mode === m.id}
              className={settings.mode === m.id ? "on" : ""}
              onClick={() => update((s) => (s.mode = m.id))}
            >
              {m.label}
            </button>
          ))}
        </div>
        <p className="sub" style={{ margin: 0 }}>
          {MODES.find((m) => m.id === settings.mode)?.blurb}{" "}
          {settings.auto_context && (
            <>
              Apps with a rule (VS Code, terminals, Overleaf, Gmail…) switch automatically —{" "}
              <a href="#" onClick={() => go("apps")}>edit rules</a>.
            </>
          )}
        </p>
      </section>

      <section className="card">
        <h2>
          <Sparkles size={15} style={{ verticalAlign: -2, marginRight: 6, color: "var(--accent)" }} />
          Try it here
        </h2>
        <textarea className="input" placeholder={`Click here, hold ${key}, and say: “um, so the meeting is Tuesday at 2, actually no, Wednesday at 4.”`} />
      </section>

      <section className="card">
        <div className="row between">
          <h2 style={{ margin: 0 }}>Recent</h2>
          <button className="btn ghost" onClick={() => go("history")}>
            View all
          </button>
        </div>
        {recent.length === 0 ? (
          <div className="empty">Nothing yet. Your dictations appear here (stored only on this PC).</div>
        ) : (
          recent.map((d) => (
            <div className="entry" key={d.id}>
              <div className="meta">
                <span className="tag">{modeLabel(d.mode)}</span>
                {d.app_exe && <span>{d.app_exe}</span>}
                <span>{new Date(d.created_ms).toLocaleTimeString()}</span>
                {d.total_ms != null && <span>· {fmtMs(d.total_ms)}</span>}
              </div>
              <div className="text">{d.final_text ?? d.raw_text}</div>
            </div>
          ))
        )}
      </section>
    </div>
  );
}
