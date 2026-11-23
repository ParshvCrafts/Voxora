import { useEffect, useState } from "react";
import { Cpu, Download, ShieldCheck } from "lucide-react";
import { useApp } from "../App";
import { api, on, type SetupProgress } from "../lib/api";

const GB = 1e9;

export default function Setup({ onDone }: { onDone: () => void }) {
  const { status, toast } = useApp();
  const [busy, setBusy] = useState(false);
  const [items, setItems] = useState<Record<string, SetupProgress>>({});

  useEffect(
    () =>
      on<SetupProgress>("setup-progress", (p) => {
        if (p.phase === "done") return onDone();
        if (p.id) setItems((m) => ({ ...m, [p.id!]: { ...m[p.id!], ...p } }));
      }),
    [onDone],
  );

  const start = async () => {
    setBusy(true);
    try {
      await api.install();
      onDone();
    } catch (e) {
      toast(`Setup failed: ${e}`, true);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page" style={{ maxWidth: 640 }}>
      <div>
        <h1>Set up Voxora</h1>
        <p className="sub">
          Voxora runs entirely on your computer. It needs to download the open-source speech and rewrite models once
          (about {status.gpu ? "4.7" : "1.9"} GB). After that it works offline, for free, forever.
        </p>
      </div>
      <section className="card stack">
        <div className="row">
          <Cpu size={18} style={{ color: "var(--accent)" }} />
          <span>{status.gpu ?? "No NVIDIA GPU found — CPU builds and the smaller model will be used."}</span>
        </div>
        <div className="row">
          <ShieldCheck size={18} style={{ color: "var(--good)" }} />
          <span>Every file is pinned to an exact version and verified by SHA-256 before use.</span>
        </div>
        {Object.entries(items).map(([id, p]) => {
          const pct = p.total ? Math.min(100, ((p.done ?? 0) / p.total) * 100) : 0;
          return (
            <div key={id}>
              <div className="row between" style={{ fontSize: 12.5, marginBottom: 4 }}>
                <span>{p.label}</span>
                <span className="muted mono">
                  {p.phase === "unpacking" ? "unpacking…" : `${((p.done ?? 0) / GB).toFixed(2)} / ${((p.total ?? 0) / GB).toFixed(2)} GB`}
                </span>
              </div>
              <div className="progress">
                <div style={{ width: `${p.phase === "unpacking" ? 100 : pct}%` }} />
              </div>
            </div>
          );
        })}
        <button className="btn primary" style={{ alignSelf: "flex-start" }} disabled={busy} onClick={start}>
          <Download size={15} /> {busy ? "Downloading…" : "Download and start"}
        </button>
      </section>
    </div>
  );
}
