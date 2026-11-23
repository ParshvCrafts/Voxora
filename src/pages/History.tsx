import { useCallback, useEffect, useState } from "react";
import { Copy, RotateCcw, Search, Trash2 } from "lucide-react";
import { useApp } from "../App";
import { api, MODES, modeLabel, on, type Dictation, type Mode } from "../lib/api";

const STATUS: Record<Dictation["status"], string> = {
  inserted: "inserted",
  clipboard_only: "left on clipboard",
  transcribed: "not inserted",
  failed: "failed",
};

export default function History() {
  const { toast } = useApp();
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<Dictation[]>([]);
  const [rerun, setRerun] = useState<Record<number, string>>({});

  const load = useCallback(() => {
    api.history(q, 200).then(setRows).catch((e) => toast(String(e), true));
  }, [q, toast]);

  useEffect(() => {
    const t = setTimeout(load, 150);
    return () => clearTimeout(t);
  }, [load]);
  useEffect(() => on("history", load), [load]);

  const copy = (text: string) => api.copyText(text).then(() => toast("Copied"));
  const reprocess = async (d: Dictation, mode: Mode) => {
    try {
      const text = await api.reprocess(d.id, mode);
      setRerun((r) => ({ ...r, [d.id]: text }));
    } catch (e) {
      toast(String(e), true);
    }
  };

  return (
    <div className="page">
      <div className="row between">
        <div>
          <h1>History</h1>
          <p className="sub">Every dictation is saved before it is pasted, so nothing is lost if an app misses the paste.</p>
        </div>
        <button
          className="btn danger"
          disabled={rows.length === 0}
          onClick={() => confirm("Delete all history? This cannot be undone.") && api.historyClear().then(load)}
        >
          <Trash2 size={15} /> Clear all
        </button>
      </div>
      <div className="row" style={{ position: "relative" }}>
        <Search size={16} style={{ position: "absolute", left: 11, color: "var(--faint)" }} />
        <input className="input" style={{ paddingLeft: 34 }} placeholder="Search dictations" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <section className="card">
        {rows.length === 0 && <div className="empty">{q ? "No matches." : "No dictations yet."}</div>}
        {rows.map((d) => {
          const text = d.final_text ?? d.raw_text;
          return (
            <div className="entry" key={d.id}>
              <div className="meta">
                <span className="tag">{modeLabel(d.mode)}</span>
                <span>{new Date(d.created_ms).toLocaleString()}</span>
                {d.app_exe && <span>· {d.app_exe}</span>}
                <span>· {STATUS[d.status]}</span>
                {d.total_ms != null && (
                  <span className="mono" title="speech recognition / rewrite / total">
                    · asr {d.asr_ms} ms{d.llm_ms != null ? ` · llm ${d.llm_ms} ms` : ""} · total {d.total_ms} ms
                  </span>
                )}
              </div>
              <div className="text">{text}</div>
              {d.final_text && d.final_text !== d.raw_text && <div className="raw">heard: {d.raw_text}</div>}
              {d.error && <div className="raw" style={{ borderColor: "var(--warn)" }}>{d.error}</div>}
              {rerun[d.id] && (
                <div className="raw" style={{ borderColor: "var(--accent)" }}>
                  {rerun[d.id]}{" "}
                  <button className="btn ghost" onClick={() => copy(rerun[d.id])}>
                    <Copy size={13} /> Copy
                  </button>
                </div>
              )}
              <div className="actions">
                <button className="btn ghost" onClick={() => copy(text)}>
                  <Copy size={14} /> Copy
                </button>
                {d.final_text && d.final_text !== d.raw_text && (
                  <button className="btn ghost" onClick={() => copy(d.raw_text)}>
                    Copy original
                  </button>
                )}
                <label className="btn ghost" style={{ gap: 4 }}>
                  <RotateCcw size={14} />
                  <select
                    aria-label="Re-run in another mode"
                    value=""
                    onChange={(e) => e.target.value && reprocess(d, e.target.value as Mode)}
                    style={{ border: 0, background: "none", cursor: "pointer" }}
                  >
                    <option value="">Re-run as…</option>
                    {MODES.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                </label>
                <button className="btn ghost danger" onClick={() => api.historyDelete(d.id).then(load)} aria-label="Delete">
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          );
        })}
      </section>
    </div>
  );
}
