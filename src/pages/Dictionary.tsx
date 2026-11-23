import { useState } from "react";
import { ArrowRight, Plus, X } from "lucide-react";
import { useApp } from "../App";

export default function Dictionary() {
  const { settings, update } = useApp();
  const [term, setTerm] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const d = settings.dictionary;

  const addTerm = () => {
    const t = term.trim();
    if (t && !d.terms.some((x) => x.toLowerCase() === t.toLowerCase())) update((s) => s.dictionary.terms.push(t));
    setTerm("");
  };
  const addRep = () => {
    if (!from.trim() || !to.trim()) return;
    update((s) => s.dictionary.replacements.push({ from: from.trim(), to: to.trim() }));
    setFrom("");
    setTo("");
  };

  return (
    <div className="page">
      <div>
        <h1>Dictionary</h1>
        <p className="sub">
          Teach Voxora your words. Terms bias the speech model toward the right spelling <em>and</em> fix casing
          afterwards (“lang graph” → LangGraph).
        </p>
      </div>
      <section className="card">
        <h2>Vocabulary</h2>
        <div className="row" style={{ marginBottom: 12 }}>
          <input
            className="input"
            placeholder="Add a name, library or term (e.g. heteroscedasticity, Kubernetes, Priya)"
            value={term}
            onChange={(e) => setTerm(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addTerm()}
          />
          <button className="btn primary" onClick={addTerm}>
            <Plus size={15} /> Add
          </button>
        </div>
        <div className="chips">
          {d.terms.map((t, i) => (
            <span className="chip" key={t}>
              {t}
              <button aria-label={`Remove ${t}`} onClick={() => update((s) => s.dictionary.terms.splice(i, 1))}>
                <X size={13} />
              </button>
            </span>
          ))}
        </div>
      </section>
      <section className="card">
        <h2>Replacements &amp; snippets</h2>
        <p className="sub" style={{ marginTop: -6, marginBottom: 12 }}>
          When you say the left side, Voxora types the right side. Great for misheard names or boilerplate (“my
          email” → you@example.com).
        </p>
        <div className="row" style={{ marginBottom: 12 }}>
          <input className="input" placeholder="When I say…" value={from} onChange={(e) => setFrom(e.target.value)} />
          <ArrowRight size={16} className="muted" style={{ flex: "none" }} />
          <input
            className="input"
            placeholder="…type this"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addRep()}
          />
          <button className="btn primary" onClick={addRep}>
            <Plus size={15} /> Add
          </button>
        </div>
        {d.replacements.length === 0 && <div className="empty">No replacements yet.</div>}
        {d.replacements.map((r, i) => (
          <div className="row between entry" key={i}>
            <span>
              <span className="mono">{r.from}</span> <ArrowRight size={13} className="muted" /> <span className="mono">{r.to}</span>
            </span>
            <button className="btn ghost danger" aria-label="Remove" onClick={() => update((s) => s.dictionary.replacements.splice(i, 1))}>
              <X size={14} />
            </button>
          </div>
        ))}
      </section>
    </div>
  );
}
