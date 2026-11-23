import { Plus, X } from "lucide-react";
import { useApp } from "../App";
import { Field, Toggle } from "../components/ui";
import { MODES, type Mode, type PasteKeys } from "../lib/api";

const PASTE: { id: PasteKeys; label: string }[] = [
  { id: "ctrl_v", label: "Ctrl+V" },
  { id: "ctrl_shift_v", label: "Ctrl+Shift+V" },
  { id: "shift_insert", label: "Shift+Insert" },
];

export default function Apps() {
  const { settings, update } = useApp();
  return (
    <div className="page">
      <div>
        <h1>Modes &amp; apps</h1>
        <p className="sub">
          Voxora reads only the foreground app’s name and window title (never the screen) to pick a mode. First matching
          rule wins.
        </p>
      </div>
      <section className="card">
        <Field name="Switch modes automatically" help="Off = always use your default mode.">
          <Toggle label="Auto context" checked={settings.auto_context} onChange={(v) => update((s) => (s.auto_context = v))} />
        </Field>
        <Field name="Spoken commands" help={<>Say “new line” or “new paragraph” to insert breaks.</>}>
          <Toggle label="Spoken commands" checked={settings.spoken_commands} onChange={(v) => update((s) => (s.spoken_commands = v))} />
        </Field>
      </section>
      <section className="card">
        <div className="row between" style={{ marginBottom: 8 }}>
          <h2 style={{ margin: 0 }}>App rules</h2>
          <button
            className="btn"
            onClick={() => update((s) => s.app_rules.unshift({ exe: "", title_contains: null, mode: "clean", paste: null, terminal_period: null }))}
          >
            <Plus size={15} /> Add rule
          </button>
        </div>
        <table className="rules">
          <thead>
            <tr>
              <th>App (.exe)</th>
              <th>or window title contains</th>
              <th>Mode</th>
              <th>Paste with</th>
              <th title="Add a final period">Period</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {settings.app_rules.map((r, i) => (
              <tr key={i}>
                <td>
                  <input className="input mono" value={r.exe ?? ""} placeholder="any" onChange={(e) => update((s) => (s.app_rules[i].exe = e.target.value || null))} />
                </td>
                <td>
                  <input className="input" value={r.title_contains ?? ""} placeholder="—" onChange={(e) => update((s) => (s.app_rules[i].title_contains = e.target.value || null))} />
                </td>
                <td>
                  <select className="input" value={r.mode} onChange={(e) => update((s) => (s.app_rules[i].mode = e.target.value as Mode))}>
                    {MODES.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <select className="input" value={r.paste ?? "ctrl_v"} onChange={(e) => update((s) => (s.app_rules[i].paste = e.target.value as PasteKeys))}>
                    {PASTE.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.label}
                      </option>
                    ))}
                  </select>
                </td>
                <td style={{ textAlign: "center" }}>
                  <input
                    type="checkbox"
                    aria-label="Add final period"
                    checked={r.terminal_period ?? true}
                    onChange={(e) => update((s) => (s.app_rules[i].terminal_period = e.target.checked))}
                    style={{ accentColor: "var(--accent)" }}
                  />
                </td>
                <td>
                  <button className="btn ghost danger" aria-label="Delete rule" onClick={() => update((s) => s.app_rules.splice(i, 1))}>
                    <X size={14} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
