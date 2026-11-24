import { useState } from "react";
import { ArrowRight, Plus, X } from "lucide-react";
import { useApp } from "../App";
import { PageHead, Section } from "../components/ui";

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
      <PageHead title="Dictionary">
        Teach Voxora the names and terms you use. It listens for them and always spells them your way.
      </PageHead>

      <Section
        title="Vocabulary"
        footer={'Spoken as "lang graph", typed as LangGraph. Names, libraries, acronyms and product names work best.'}
      >
        <div style={{ padding: 20, display: "flex", flexDirection: "column", gap: 18 }}>
          <div className="row">
            <input
              className="input"
              placeholder="Add a word, like Kubernetes or Priya"
              aria-label="New vocabulary word"
              value={term}
              onChange={(e) => setTerm(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addTerm()}
            />
            <button className="btn primary" onClick={addTerm} disabled={!term.trim()}>
              <Plus size={16} /> Add
            </button>
          </div>
          <div className="chips">
            {d.terms.map((t, i) => (
              <span className="chip" key={t}>
                {t}
                <button aria-label={`Remove ${t}`} onClick={() => update((s) => s.dictionary.terms.splice(i, 1))}>
                  <X size={14} />
                </button>
              </span>
            ))}
          </div>
        </div>
      </Section>

      <Section
        title="Shortcuts and corrections"
        footer={'Say the phrase on the left and Voxora types the text on the right. Handy for an email address or a name it keeps mishearing.'}
      >
        <div className="row" style={{ padding: 20 }}>
          <input className="input" placeholder="When I say" aria-label="When I say" value={from} onChange={(e) => setFrom(e.target.value)} />
          <ArrowRight size={18} className="muted" style={{ flex: "none" }} />
          <input
            className="input"
            placeholder="Type this"
            aria-label="Type this"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addRep()}
          />
          <button className="btn primary" onClick={addRep} disabled={!from.trim() || !to.trim()}>
            <Plus size={16} /> Add
          </button>
        </div>
        {d.replacements.map((r, i) => (
          <div className="item" key={i}>
            <div className="row" style={{ gap: 14, minWidth: 0 }}>
              <span>{r.from}</span>
              <ArrowRight size={15} className="muted" style={{ flex: "none" }} />
              <span className="mono">{r.to}</span>
            </div>
            <button className="btn plain icon" aria-label="Remove" onClick={() => update((s) => s.dictionary.replacements.splice(i, 1))}>
              <X size={16} />
            </button>
          </div>
        ))}
      </Section>
    </div>
  );
}
