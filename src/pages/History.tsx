import { useCallback, useEffect, useState } from "react";
import { Copy, RotateCcw, Search, Trash2 } from "lucide-react";
import { useApp } from "../App";
import { PageHead } from "../components/ui";
import { api, MODES, modeLabel, on, type Dictation, type Mode } from "../lib/api";

const PAGE = 200;

const STATUS: Record<Dictation["status"], string> = {
  inserted: "Inserted",
  clipboard_only: "Left on clipboard",
  transcribed: "Not inserted",
  failed: "Failed",
};

export default function History() {
  const { toast } = useApp();
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<Dictation[]>([]);
  const [rerun, setRerun] = useState<Record<number, string>>({});
  const [more, setMore] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  // Reloads the first page; "Show more" pages are dropped, which is fine after a change or a new search.
  const load = useCallback(() => {
    api
      .history(q, PAGE, 0)
      .then((r) => {
        setRows(r);
        setMore(r.length === PAGE);
      })
      .catch((e) => toast(String(e), true));
  }, [q, toast]);

  const showMore = () =>
    api
      .history(q, PAGE, rows.length)
      .then((r) => {
        setRows((prev) => [...prev, ...r.filter((d) => !prev.some((p) => p.id === d.id))]);
        setMore(r.length === PAGE);
      })
      .catch((e) => toast(String(e), true));

  useEffect(() => {
    if (!confirmClear) return;
    const t = setTimeout(() => setConfirmClear(false), 4000);
    return () => clearTimeout(t);
  }, [confirmClear]);

  const clearAll = () => {
    if (!confirmClear) return setConfirmClear(true);
    setConfirmClear(false);
    api.historyClear().then(load).catch((e) => toast(String(e), true));
  };
  const remove = (id: number) => api.historyDelete(id).then(load).catch((e) => toast(String(e), true));

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
      <div className="row between" style={{ alignItems: "flex-end" }}>
        <PageHead title="History">
          Every dictation is saved before it is pasted, so nothing is lost if an app misses it.
        </PageHead>
        <button
          className="btn danger"
          disabled={rows.length === 0}
          onClick={clearAll}
        >
          <Trash2 size={15} /> {confirmClear ? "Click again to clear all" : "Clear all"}
        </button>
      </div>

      <div className="search">
        <Search size={17} />
        <input className="input" placeholder="Search" aria-label="Search dictations" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>

      <div className="group">
        {rows.length === 0 && <div className="empty">{q ? "No matches." : "No dictations yet."}</div>}
        {rows.map((d) => {
          const text = d.final_text ?? d.raw_text;
          const edited = d.final_text != null && d.final_text !== d.raw_text;
          return (
            <div className="entry" key={d.id}>
              <div className="meta">
                <span className="tag">{modeLabel(d.mode)}</span>
                <span>
                  {new Date(d.created_ms).toLocaleString([], {
                    month: "short",
                    day: "numeric",
                    hour: "numeric",
                    minute: "2-digit",
                  })}
                </span>
                {d.app_exe && <span>{d.app_exe}</span>}
                <span>{STATUS[d.status]}</span>
                {d.total_ms != null && (
                  <span title={`Speech ${d.asr_ms} ms${d.llm_ms != null ? `, rewrite ${d.llm_ms} ms` : ""}`}>{d.total_ms} ms</span>
                )}
              </div>
              <div className="text">{text}</div>
              {edited && <div className="note">You said: {d.raw_text}</div>}
              {d.error && <div className="note warn">{d.error}</div>}
              {rerun[d.id] && (
                <div className="note">
                  {rerun[d.id]}
                  <div style={{ marginTop: 8 }}>
                    <button className="btn plain" onClick={() => copy(rerun[d.id])}>
                      <Copy size={14} /> Copy this version
                    </button>
                  </div>
                </div>
              )}
              <div className="actions">
                <button className="btn plain" onClick={() => copy(text)}>
                  <Copy size={15} /> Copy
                </button>
                {edited && (
                  <button className="btn plain" onClick={() => copy(d.raw_text)}>
                    Copy what I said
                  </button>
                )}
                <label className="btn plain">
                  <RotateCcw size={15} />
                  <select
                    aria-label="Redo in another mode"
                    value=""
                    onChange={(e) => e.target.value && reprocess(d, e.target.value as Mode)}
                    style={{ border: 0, background: "none", cursor: "pointer", color: "inherit" }}
                  >
                    <option value="">Redo as</option>
                    {MODES.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                </label>
                <span className="spacer" />
                <button className="btn plain icon" onClick={() => remove(d.id)} aria-label="Delete" title="Delete">
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          );
        })}
      </div>
      {more && (
        <div style={{ display: "flex", justifyContent: "center", marginTop: 16 }}>
          <button className="btn" onClick={showMore}>
            Show more
          </button>
        </div>
      )}
    </div>
  );
}
