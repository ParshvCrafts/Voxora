import { useEffect, useState } from "react";
import { FolderOpen, RefreshCw } from "lucide-react";
import { useApp } from "../App";
import { Field, NumberInput, Toggle } from "../components/ui";
import { api } from "../lib/api";

const LLM_MODELS = [
  { file: "Qwen3.5-4B-Q4_K_M.gguf", label: "Qwen3.5 4B — best quality (default)" },
  { file: "Qwen3.5-2B-Q4_K_M.gguf", label: "Qwen3.5 2B — ~2× faster" },
];

export default function SettingsPage() {
  const { settings: s, update, status, toast } = useApp();
  const [devices, setDevices] = useState<string[]>([]);
  const [keys, setKeys] = useState<string[]>([]);

  useEffect(() => {
    api.listDevices().then(setDevices);
    api.listKeys().then(setKeys);
  }, []);

  return (
    <div className="page">
      <h1>Settings</h1>

      <section className="card">
        <h2>Hotkeys</h2>
        <Field name="Dictation key" help="Hold to talk, tap for hands-free. The key is reserved for Voxora while it runs.">
          <select className="input" value={s.hotkey.dictation_key} onChange={(e) => update((x) => (x.hotkey.dictation_key = e.target.value))}>
            {keys.map((k) => (
              <option key={k}>{k}</option>
            ))}
          </select>
        </Field>
        <Field name="Hold vs. tap threshold" help="Presses shorter than this start hands-free mode.">
          <NumberInput value={s.hotkey.hold_threshold_ms} min={120} max={600} step={10} suffix=" ms" onChange={(v) => update((x) => (x.hotkey.hold_threshold_ms = v))} />
        </Field>
        <Field name="Cycle mode" help="Ctrl + Alt + this key.">
          <input className="input mono" maxLength={1} value={s.hotkey.cycle_mode_key} onChange={(e) => e.target.value && update((x) => (x.hotkey.cycle_mode_key = e.target.value.toUpperCase()))} style={{ width: 60 }} />
        </Field>
        <Field name="Paste last dictation" help="Ctrl + Alt + this key. Recovers text an app missed.">
          <input className="input mono" maxLength={1} value={s.hotkey.repaste_key} onChange={(e) => e.target.value && update((x) => (x.hotkey.repaste_key = e.target.value.toUpperCase()))} style={{ width: 60 }} />
        </Field>
      </section>

      <section className="card">
        <h2>Microphone</h2>
        <Field name="Input device">
          <select className="input" value={s.audio.device ?? ""} onChange={(e) => update((x) => (x.audio.device = e.target.value || null))}>
            <option value="">Windows default</option>
            {devices.map((d) => (
              <option key={d}>{d}</option>
            ))}
          </select>
        </Field>
        <Field name="Hands-free stops after silence">
          <NumberInput value={s.audio.hands_free_silence_ms} min={500} max={4000} step={100} suffix=" ms" onChange={(v) => update((x) => (x.audio.hands_free_silence_ms = v))} />
        </Field>
      </section>

      <section className="card">
        <h2>AI engines</h2>
        <Field name="Speech language" help="“auto” detects the language per dictation (slightly slower).">
          <select className="input" value={s.asr.language} onChange={(e) => update((x) => (x.asr.language = e.target.value))}>
            {["en", "auto", "es", "fr", "de", "hi", "zh", "ja", "pt", "it"].map((l) => (
              <option key={l}>{l}</option>
            ))}
          </select>
        </Field>
        <Field name="Use GPU for speech" help={status.gpu ?? "No NVIDIA GPU detected — CPU will be used."}>
          <Toggle label="GPU" checked={s.asr.use_gpu} onChange={(v) => update((x) => (x.asr.use_gpu = v))} />
        </Field>
        <Field name="AI rewrite" help="Off = instant, rule-based cleanup only (fillers, punctuation, math, dictionary).">
          <Toggle label="AI rewrite" checked={s.llm.enabled} onChange={(v) => update((x) => (x.llm.enabled = v))} />
        </Field>
        {s.llm.enabled && (
          <>
            <Field name="Rewrite engine" help="Built-in runs llama.cpp locally. External = any OpenAI-compatible server (Ollama, LM Studio).">
              <select className="input" value={s.llm.managed ? "managed" : "external"} onChange={(e) => update((x) => (x.llm.managed = e.target.value === "managed"))}>
                <option value="managed">Built-in (recommended)</option>
                <option value="external">External server</option>
              </select>
            </Field>
            {s.llm.managed ? (
              <Field name="Rewrite model" help="Switching downloads the model if needed.">
                <select className="input" value={s.llm.model} onChange={(e) => update((x) => (x.llm.model = e.target.value))}>
                  {LLM_MODELS.map((m) => (
                    <option key={m.file} value={m.file}>
                      {m.label}
                    </option>
                  ))}
                </select>
              </Field>
            ) : (
              <>
                <Field name="Endpoint" help="Base URL ending in /v1, e.g. http://127.0.0.1:11434/v1 for Ollama.">
                  <input className="input mono" defaultValue={s.llm.endpoint} onBlur={(e) => update((x) => (x.llm.endpoint = e.target.value.trim()))} />
                </Field>
                <Field name="Model name">
                  <input className="input mono" defaultValue={s.llm.model} onBlur={(e) => update((x) => (x.llm.model = e.target.value.trim()))} />
                </Field>
              </>
            )}
            <Field name="Rewrite time limit" help="If the AI is slower than this, the rule-based text is inserted instead.">
              <NumberInput value={s.llm.timeout_ms} min={800} max={8000} step={100} suffix=" ms" onChange={(v) => update((x) => (x.llm.timeout_ms = v))} />
            </Field>
          </>
        )}
      </section>

      <section className="card">
        <h2>Pasting &amp; privacy</h2>
        <Field name="Restore my clipboard after pasting" help="Your copied text or image comes back once the app has read the dictation.">
          <Toggle label="Restore clipboard" checked={s.paste.restore_clipboard} onChange={(v) => update((x) => (x.paste.restore_clipboard = v))} />
        </Field>
        <Field name="Restore delay" help="Raise this if a slow web app pastes your old clipboard instead.">
          <NumberInput value={s.paste.restore_delay_ms} min={120} max={800} step={20} suffix=" ms" onChange={(v) => update((x) => (x.paste.restore_delay_ms = v))} />
        </Field>
        <Field name="Keep history" help="Stored only in %LOCALAPPDATA%\Voxora. Audio is never saved.">
          <Toggle label="History" checked={s.history.enabled} onChange={(v) => update((x) => (x.history.enabled = v))} />
        </Field>
        <Field name="Delete history older than">
          <NumberInput value={s.history.retention_days} min={1} max={365} suffix=" days" onChange={(v) => update((x) => (x.history.retention_days = v))} />
        </Field>
      </section>

      <section className="card">
        <h2>General</h2>
        <Field name="Start with Windows" help="Starts minimized to the tray.">
          <Toggle label="Autostart" checked={s.autostart} onChange={(v) => update((x) => (x.autostart = v))} />
        </Field>
        <Field name="Theme">
          <div className="seg">
            {(["system", "light", "dark"] as const).map((t) => (
              <button key={t} className={s.theme === t ? "on" : ""} onClick={() => update((x) => (x.theme = t))}>
                {t[0].toUpperCase() + t.slice(1)}
              </button>
            ))}
          </div>
        </Field>
        <Field name="Troubleshooting">
          <div className="row" style={{ gap: 6 }}>
            <button className="btn" onClick={() => api.restartEngines().then(() => toast("Restarting engines…"))}>
              <RefreshCw size={14} /> Restart engines
            </button>
            <button className="btn" onClick={() => api.openFolder("logs")}>
              <FolderOpen size={14} /> Logs
            </button>
          </div>
        </Field>
      </section>
    </div>
  );
}
