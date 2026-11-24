import { Plus, X } from "lucide-react";
import { useApp } from "../App";
import { Item, PageHead, Section, Toggle } from "../components/ui";
import { IS_MAC, MODES, type Mode, type PasteKeys } from "../lib/api";

const PASTE: { id: PasteKeys; label: string }[] = [
  { id: "ctrl_v", label: "Ctrl+V" },
  { id: "ctrl_shift_v", label: "Ctrl+Shift+V" },
  { id: "shift_insert", label: "Shift+Insert" },
];

export default function Apps() {
  const { settings, update } = useApp();
  return (
    <div className="page">
      <PageHead title="Modes & Apps">
        Voxora looks at which app is in front (its name and window title, never the screen) to choose a mode.
      </PageHead>

      <Section>
        <Item name="Switch modes automatically" help="When off, your default mode is always used.">
          <Toggle label="Switch modes automatically" checked={settings.auto_context} onChange={(v) => update((s) => (s.auto_context = v))} />
        </Item>
        <Item
          name="Spoken commands"
          help={'Say "new line", "new paragraph", "scratch that", "quote ... end quote" or end with "period".'}
        >
          <Toggle label="Spoken commands" checked={settings.spoken_commands} onChange={(v) => update((s) => (s.spoken_commands = v))} />
        </Item>
      </Section>

      <Section
        title="App rules"
        aside={
          <button
            className="btn"
            onClick={() =>
              update((s) => s.app_rules.unshift({ exe: "", title_contains: null, mode: "clean", paste: null, terminal_period: null }))
            }
          >
            <Plus size={15} /> Add rule
          </button>
        }
        footer="Rules are checked from the top. The first one that matches wins."
      >
        <div className="rule head" style={IS_MAC ? { gridTemplateColumns: "1fr 1fr 1.25fr 24px 36px" } : undefined}>
          <span>App</span>
          <span>Or title contains</span>
          <span>Mode</span>
          {!IS_MAC && <span>Paste with</span>}
          <span title="Add a period at the end">Period</span>
          <span style={{ width: 36 }} />
        </div>
        {settings.app_rules.map((r, i) => (
          <div className="rule" key={i} style={IS_MAC ? { gridTemplateColumns: "1fr 1fr 1.25fr 24px 36px" } : undefined}>
            <input
              className="input"
              aria-label="App"
              value={r.exe ?? ""}
              placeholder="Any app"
              onChange={(e) => update((s) => (s.app_rules[i].exe = e.target.value || null))}
            />
            <input
              className="input"
              aria-label="Window title contains"
              value={r.title_contains ?? ""}
              placeholder="Any title"
              onChange={(e) => update((s) => (s.app_rules[i].title_contains = e.target.value || null))}
            />
            <select
              className="input"
              aria-label="Mode"
              value={r.mode}
              onChange={(e) => update((s) => (s.app_rules[i].mode = e.target.value as Mode))}
            >
              {MODES.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </select>
            {!IS_MAC && (
              <select
                className="input"
                aria-label="Paste with"
                value={r.paste ?? "ctrl_v"}
                onChange={(e) => update((s) => (s.app_rules[i].paste = e.target.value as PasteKeys))}
              >
                {PASTE.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            )}
            <input
              type="checkbox"
              aria-label="Add a period at the end"
              checked={r.terminal_period ?? true}
              onChange={(e) => update((s) => (s.app_rules[i].terminal_period = e.target.checked))}
              style={{ justifySelf: "center" }}
            />
            <button className="btn plain icon" aria-label="Delete rule" onClick={() => update((s) => s.app_rules.splice(i, 1))}>
              <X size={16} />
            </button>
          </div>
        ))}
      </Section>
    </div>
  );
}
