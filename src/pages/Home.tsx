import { useEffect, useState } from "react";
import { Pause, Play } from "lucide-react";
import { useApp } from "../App";
import { EngineStatus, Item, Keys, Section } from "../components/ui";
import { api, IS_MAC, keyLabel, MOD, MODES, modeLabel, on, type Dictation, type Stats } from "../lib/api";

const fmtMs = (v: number | null) => (v == null ? "Not yet" : v >= 1000 ? `${(v / 1000).toFixed(2)} s` : `${v} ms`);

export default function Home({ go }: { go: (page: string) => void }) {
  const { settings, status, update, toast } = useApp();
  const [stats, setStats] = useState<Stats | null>(null);
  const [recent, setRecent] = useState<Dictation[]>([]);
  const key = keyLabel(settings.hotkey.dictation_key);

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

  const togglePause = () =>
    api
      .setPaused(!status.paused)
      .then(() => api.getStatus())
      .then((s) => toast(s.paused ? "Paused. The key does nothing until you resume." : "Listening for the key again"));

  return (
    <div className="page">
      <div className="group hero">
        <div>
          <h1>{status.paused ? "Voxora is paused." : `Hold ${key} and speak.`}</h1>
          <p>
            Let go, and clean text appears wherever your cursor is. Tap the key once to keep listening hands-free.
          </p>
          <div className="status-line">
            <EngineStatus label="Speech" value={status.asr} />
            <EngineStatus label="Rewrite" value={status.llm} />
            {status.gpu && <span>{status.gpu}</span>}
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 16 }}>
          <div className="keycap" aria-hidden>
            {key}
          </div>
          <button className="btn plain" onClick={togglePause}>
            {status.paused ? <Play size={15} /> : <Pause size={15} />}
            {status.paused ? "Resume" : "Pause"}
          </button>
        </div>
      </div>

      <div className="group stats">
        {(
          [
            ["Dictations", (stats?.count ?? 0).toLocaleString()],
            ["Words", (stats?.words_total ?? 0).toLocaleString()],
            ["Typical speed", fmtMs(stats?.p50_total_ms ?? null)],
            ["Slowest 5%", fmtMs(stats?.p95_total_ms ?? null)],
          ] as const
        ).map(([lbl, num]) => (
          <div className="stat" key={lbl}>
            <div className="num">{num}</div>
            <div className="lbl">{lbl}</div>
          </div>
        ))}
      </div>

      <Section
        title="Mode"
        aside={<Keys keys={[MOD.ctrl, MOD.alt, settings.hotkey.cycle_mode_key]} />}
        footer={
          <>
            {MODES.find((m) => m.id === settings.mode)?.blurb}{" "}
            {settings.auto_context && (
              <>
                Code editors, terminals and email switch modes on their own.{" "}
                <a href="#" onClick={() => go("apps")}>
                  Edit app rules
                </a>
              </>
            )}
          </>
        }
        pad
      >
        <div className="seg" role="radiogroup" aria-label="Default mode">
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
      </Section>

      <Section title="Try it">
        <div style={{ padding: 16 }}>
          <textarea
            className="input"
            placeholder={`Click here, hold ${key}, and say: "um, so the meeting is Tuesday at 2, actually no, Wednesday at 4."`}
          />
        </div>
      </Section>

      <Section title="Shortcuts">
        <Item name="Dictate">
          <span className="muted">Hold</span> <Keys keys={[key]} />
        </Item>
        <Item name="Hands-free" help="Stops by itself when you pause, or tap again.">
          <span className="muted">Tap</span> <Keys keys={[key]} />
        </Item>
        <Item name="Cancel while listening">
          <Keys keys={["Esc"]} />
        </Item>
        <Item name="Paste the last dictation again" help="For apps that missed the paste.">
          <Keys keys={[MOD.ctrl, MOD.alt, settings.hotkey.repaste_key]} />
        </Item>
        <Item name="Undo an insertion" help="Works in the app you dictated into.">
          <Keys keys={[IS_MAC ? "Command" : "Ctrl", "Z"]} />
        </Item>
      </Section>

      <Section
        title="Recent"
        aside={
          recent.length > 0 && (
            <a href="#" onClick={() => go("history")}>
              Show all
            </a>
          )
        }
      >
        {recent.length === 0 ? (
          <div className="empty">Your dictations will show up here. They stay on this {IS_MAC ? "Mac" : "PC"}.</div>
        ) : (
          recent.map((d) => (
            <div className="entry" key={d.id}>
              <div className="meta">
                <span className="tag">{modeLabel(d.mode)}</span>
                {d.app_exe && <span>{d.app_exe}</span>}
                <span>{new Date(d.created_ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}</span>
                {d.total_ms != null && <span>{fmtMs(d.total_ms)}</span>}
              </div>
              <div className="text">{d.final_text ?? d.raw_text}</div>
            </div>
          ))
        )}
      </Section>
    </div>
  );
}
