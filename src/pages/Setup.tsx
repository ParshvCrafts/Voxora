import { useEffect, useState } from "react";
import { Check, Cpu, ShieldCheck, WifiOff } from "lucide-react";
import { useApp } from "../App";
import { Item, Section } from "../components/ui";
import { api, IS_MAC, on, type SetupProgress } from "../lib/api";
import logo from "../../assets/logo.svg";

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
      toast(`Setup stopped: ${e}`, true);
    } finally {
      setBusy(false);
    }
  };

  const gpu = status.gpu?.split(" · ")[0] ?? (IS_MAC ? "Apple silicon" : null);
  const size = IS_MAC ? "3.6" : gpu ? "4.7" : "1.9"; // Mac: no CUDA libraries to fetch

  return (
    <div className="page" style={{ maxWidth: 600, paddingTop: 24 }}>
      <div style={{ textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center", gap: 18 }}>
        <img src={logo} alt="" width={88} height={88} style={{ filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.12))" }} />
        <h1 style={{ fontSize: 30, fontWeight: 700, letterSpacing: "-0.025em", margin: 0 }}>Welcome to Voxora</h1>
        <p className="muted" style={{ margin: 0, maxWidth: 460 }}>
          Voxora runs entirely on this {IS_MAC ? "Mac" : "PC"}. First it downloads its open-source speech and writing
          models, about {size} GB, once.
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

      <div style={{ display: "flex", justifyContent: "center" }}>
        <button className="btn primary lg" disabled={busy} onClick={start}>
          {busy ? "Downloading" : "Download and start"}
        </button>
      </div>
    </div>
  );
}
