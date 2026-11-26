import { useEffect, useState } from "react";
import { Check, Cpu, ShieldCheck, WifiOff } from "lucide-react";
import { useApp } from "../App";
import { Item, Section, Toggle } from "../components/ui";
import { api, IS_MAC, on, type SetupPlan, type SetupProgress } from "../lib/api";
import logo from "../../assets/logo.svg";

const GB = 1e9;
const gb = (bytes: number) => (bytes / GB).toFixed(1);

export default function Setup({ onDone }: { onDone: () => void }) {
  const { status, settings, update } = useApp();
  const [started, setStarted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState<Record<string, SetupProgress>>({});
  const [plan, setPlan] = useState<SetupPlan | null>(null);
  const busy = started || status.installing;

  // The backend decides what this machine needs (GPU or CPU builds, model size); ask again when
  // the GPU check finishes or the AI rewrite choice changes.
  useEffect(() => {
    api.setupPlan().then(setPlan).catch(() => {});
  }, [status.gpu, settings.llm.enabled, settings.llm.model]);

  useEffect(
    () =>
      on<SetupProgress>("setup-progress", (p) => {
        if (p.phase === "done") return onDone();
        if (p.id) setItems((m) => ({ ...m, [p.id!]: { ...m[p.id!], ...p } }));
      }),
    [onDone],
  );

  const start = async () => {
    setStarted(true);
    setError(null);
    try {
      await api.install();
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setStarted(false);
    }
  };

  const gpu = status.gpu?.split(" · ")[0] ?? (IS_MAC ? "Apple silicon" : null);
  const rewriteOn = settings.llm.enabled && settings.llm.managed;
  const total = plan ? plan.speech_bytes + (rewriteOn ? plan.rewrite_bytes : 0) : 0;
  const rewriteBytes = plan?.rewrite_bytes ?? 0;
  const size = total > 0 ? `about ${gb(total)} GB` : "a few GB";

  return (
    <div className="page" style={{ maxWidth: 600, paddingTop: 24 }}>
      <div style={{ textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center", gap: 18 }}>
        <img src={logo} alt="" width={88} height={88} style={{ filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.12))" }} />
        <h1 style={{ fontSize: 30, fontWeight: 700, letterSpacing: "-0.025em", margin: 0 }}>Welcome to Voxora</h1>
        <p className="muted" style={{ margin: 0, maxWidth: 460 }}>
          Voxora runs entirely on this {IS_MAC ? "Mac" : "PC"}. First it downloads its open-source speech and writing
          models, {size}, once.
        </p>
      </div>

      <Section>
        <Item name="Works offline" help="After this download, nothing you say ever leaves your computer.">
          <WifiOff size={20} className="muted" />
        </Item>
        <Item name="Verified files" help="Every file is pinned to an exact version and checked with SHA-256.">
          <ShieldCheck size={20} className="muted" />
        </Item>
        <Item name={gpu ?? "CPU mode"} help={gpu ? "Speech and rewriting run on your GPU." : "No supported GPU found. The smaller model keeps it quick."}>
          <Cpu size={20} className="muted" />
        </Item>
      </Section>

      {settings.llm.managed && (
        <Section>
          <Item
            name={`AI rewrite${rewriteBytes > 0 ? `, ${gb(rewriteBytes)} GB` : ""}`}
            help="Cleans up fillers and punctuation with a local language model. Speech-to-text works without it."
          >
            <span style={busy ? { opacity: 0.4, pointerEvents: "none" } : undefined}>
              <Toggle label="AI rewrite" checked={settings.llm.enabled} onChange={(v) => update((x) => (x.llm.enabled = v))} />
            </span>
          </Item>
        </Section>
      )}

      {Object.keys(items).length > 0 && (
        <Section>
          {Object.entries(items).map(([id, p]) => {
            const pct = p.total ? Math.min(100, ((p.done ?? 0) / p.total) * 100) : 0;
            const finished = p.phase !== "unpacking" && pct >= 100;
            return (
              <div className="item" key={id} style={{ flexDirection: "column", alignItems: "stretch", gap: 10 }}>
                <div className="row between">
                  <span>{p.label}</span>
                  <span className="muted" style={{ fontSize: 14, fontVariantNumeric: "tabular-nums" }}>
                    {finished ? (
                      <Check size={16} />
                    ) : p.phase === "unpacking" ? (
                      "Unpacking"
                    ) : (
                      `${((p.done ?? 0) / GB).toFixed(2)} of ${((p.total ?? 0) / GB).toFixed(2)} GB`
                    )}
                  </span>
                </div>
                <div className="progress">
                  <div style={{ width: `${p.phase === "unpacking" ? 100 : pct}%` }} />
                </div>
              </div>
            );
          })}
        </Section>
      )}

      {error && !busy && (
        <div className="note-box err" role="alert">
          <b>Setup stopped.</b> {error}
          <div className="muted" style={{ marginTop: 4 }}>
            Files that finished downloading are kept, and partial downloads resume where they stopped.
          </div>
        </div>
      )}

      <div style={{ display: "flex", justifyContent: "center" }}>
        <button className="btn primary lg" disabled={busy} onClick={start}>
          {busy ? "Downloading" : error ? "Retry" : "Download and start"}
        </button>
      </div>
    </div>
  );
}
