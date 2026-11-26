import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { FolderOpen, Power, RefreshCw } from "lucide-react";
import { useApp } from "../App";
import { Item, PageHead, Range, Section, Toggle } from "../components/ui";
import { api, IS_MAC, keyLabel, MOD } from "../lib/api";

const LLM_MODELS = [
  { file: "Qwen3.5-4B-Q4_K_M.gguf", label: "Qwen3.5 4B, best quality" },
  { file: "Qwen3.5-2B-Q4_K_M.gguf", label: "Qwen3.5 2B, about 2x faster" },
];
const LANGUAGES: [string, string][] = [
  ["en", "English"],
  ["auto", "Detect automatically"],
  ["es", "Spanish"],
  ["fr", "French"],
  ["de", "German"],
  ["hi", "Hindi"],
  ["zh", "Chinese"],
  ["ja", "Japanese"],
  ["pt", "Portuguese"],
  ["it", "Italian"],
];

export default function SettingsPage() {
  const { settings: s, update, status, toast } = useApp();
  const [devices, setDevices] = useState<string[]>([]);
  const [keys, setKeys] = useState<string[]>([]);
  const [version, setVersion] = useState("");

  useEffect(() => {
    api.listDevices().then(setDevices).catch(() => {});
    api.listKeys().then(setKeys).catch(() => {});
    getVersion().then(setVersion).catch(() => {});
  }, []);

  const savedDevice = s.audio.device;
  const deviceNames = [...new Set(devices)];

  const letter = (v: string) => v.slice(-1).toUpperCase();

  return (
    <div className="page">
      <PageHead title="Settings" />

      <Section title="Keyboard">
        <Item name="Dictation key" help={
            <>
              Hold to talk, tap for hands-free. Voxora reserves this key while it runs.
              {!IS_MAC && " On keyboard layouts where Right Alt is AltGr (German, French, Polish and others), pick a different key."}
            </>
          }>
          <select
            className="input"
            aria-label="Dictation key"
            value={s.hotkey.dictation_key}
            onChange={(e) => update((x) => (x.hotkey.dictation_key = e.target.value))}
          >
            {keys.map((k) => (
              <option key={k} value={k}>
                {keyLabel(k)}
              </option>
            ))}
          </select>
        </Item>
        <Item name="Tap or hold" help="Presses shorter than this start hands-free listening.">
          <Range
            label="Tap or hold threshold"
            value={s.hotkey.hold_threshold_ms}
            min={120}
            max={600}
            step={10}
            unit="ms"
            onChange={(v) => update((x) => (x.hotkey.hold_threshold_ms = v))}
          />
        </Item>
        <Item name="Switch mode" help={`${MOD.ctrl} + ${MOD.alt} + this letter`}>
          <input
            className="input mono"
            aria-label="Switch mode letter"
            maxLength={1}
            value={s.hotkey.cycle_mode_key}
            onChange={(e) => e.target.value && update((x) => (x.hotkey.cycle_mode_key = letter(e.target.value)))}
            style={{ width: 56, textAlign: "center" }}
          />
        </Item>
        <Item name="Paste last dictation" help={`${MOD.ctrl} + ${MOD.alt} + this letter`}>
          <input
            className="input mono"
            aria-label="Paste last dictation letter"
            maxLength={1}
            value={s.hotkey.repaste_key}
            onChange={(e) => e.target.value && update((x) => (x.hotkey.repaste_key = letter(e.target.value)))}
            style={{ width: 56, textAlign: "center" }}
          />
        </Item>
      </Section>

      <Section title="Microphone">
        <Item name="Input">
          <select
            className="input"
            aria-label="Microphone"
            value={s.audio.device ?? ""}
            onChange={(e) => update((x) => (x.audio.device = e.target.value || null))}
          >
            <option value="">System default</option>
            {savedDevice && !deviceNames.includes(savedDevice) && (
              <option value={savedDevice}>{savedDevice} (not connected)</option>
            )}
            {deviceNames.map((d) => (
              <option key={d}>{d}</option>
            ))}
          </select>
        </Item>
        <Item name="Hands-free stops after" help="How long a pause ends hands-free listening.">
          <Range
            label="Silence before stopping"
            value={s.audio.hands_free_silence_ms}
            min={500}
            max={6000}
            step={100}
            unit="ms"
            onChange={(v) => update((x) => (x.audio.hands_free_silence_ms = v))}
          />
        </Item>
      </Section>

      <Section title="Speech and rewriting">
        <Item name="Language">
          <select className="input" aria-label="Language" value={s.asr.language} onChange={(e) => update((x) => (x.asr.language = e.target.value))}>
            {LANGUAGES.map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
        </Item>
        <Item name="Use the GPU" help={status.gpu ?? (IS_MAC ? "Uses Apple silicon through Metal." : "No NVIDIA GPU found, so the CPU is used.")}>
          <Toggle label="Use the GPU" checked={s.asr.use_gpu} onChange={(v) => update((x) => (x.asr.use_gpu = v))} />
        </Item>
        <Item name="AI rewrite" help="When off, only the instant rule-based cleanup runs: fillers, punctuation, math and your dictionary.">
          <Toggle label="AI rewrite" checked={s.llm.enabled} onChange={(v) => update((x) => (x.llm.enabled = v))} />
        </Item>
        {s.llm.enabled && (
          <>
            <Item name="Rewrite engine" help="Built in runs llama.cpp on this computer. External uses any OpenAI-compatible server, such as Ollama.">
              <select
                className="input"
                aria-label="Rewrite engine"
                value={s.llm.managed ? "managed" : "external"}
                onChange={(e) => update((x) => (x.llm.managed = e.target.value === "managed"))}
              >
                <option value="managed">Built in</option>
                <option value="external">External server</option>
              </select>
            </Item>
            {s.llm.managed ? (
              <Item name="Model" help="Switching downloads the model if it isn't on disk yet.">
                <select className="input" aria-label="Model" value={s.llm.model} onChange={(e) => update((x) => (x.llm.model = e.target.value))}>
                  {LLM_MODELS.map((m) => (
                    <option key={m.file} value={m.file}>
                      {m.label}
                    </option>
                  ))}
                </select>
              </Item>
            ) : (
              <>
                <Item name="Server address" help="Ends in /v1. Ollama uses http://127.0.0.1:11434/v1">
                  <input
                    className="input mono"
                    aria-label="Server address"
                    defaultValue={s.llm.endpoint}
                    onBlur={(e) => update((x) => (x.llm.endpoint = e.target.value.trim()))}
                    style={{ width: 260 }}
                  />
                </Item>
                <Item name="Model name">
                  <input
                    className="input mono"
                    aria-label="Model name"
                    defaultValue={s.llm.model}
                    onBlur={(e) => update((x) => (x.llm.model = e.target.value.trim()))}
                    style={{ width: 260 }}
                  />
                </Item>
              </>
            )}
            <Item name="Time limit" help="If the rewrite takes longer, the rule-based text is used instead.">
              <Range
                label="Rewrite time limit"
                value={s.llm.timeout_ms}
                min={800}
                max={8000}
                step={100}
                unit="ms"
                onChange={(v) => update((x) => (x.llm.timeout_ms = v))}
              />
            </Item>
          </>
        )}
      </Section>

      <Section title="Clipboard and privacy">
        <Item name="Put my clipboard back" help="Whatever you had copied returns right after the paste.">
          <Toggle label="Put my clipboard back" checked={s.paste.restore_clipboard} onChange={(v) => update((x) => (x.paste.restore_clipboard = v))} />
        </Item>
        <Item name="Wait before restoring" help="Raise this if a slow web app pastes your old clipboard.">
          <Range
            label="Restore delay"
            value={s.paste.restore_delay_ms}
            min={120}
            max={800}
            step={20}
            unit="ms"
            onChange={(v) => update((x) => (x.paste.restore_delay_ms = v))}
          />
        </Item>
        <Item name="Keep history" help="Text only, stored on this computer. Audio is never saved.">
          <Toggle label="Keep history" checked={s.history.enabled} onChange={(v) => update((x) => (x.history.enabled = v))} />
        </Item>
        <Item name="Delete history after">
          <Range
            label="History retention"
            value={s.history.retention_days}
            min={1}
            max={365}
            unit="days"
            onChange={(v) => update((x) => (x.history.retention_days = v))}
          />
        </Item>
      </Section>

      <Section title="General">
        <Item name={IS_MAC ? "Open at login" : "Start with Windows"} help="Starts quietly in the menu bar or tray.">
          <Toggle label="Open at login" checked={s.autostart} onChange={(v) => update((x) => (x.autostart = v))} />
        </Item>
        <Item name="Appearance">
          <div className="seg" role="radiogroup" aria-label="Appearance">
            {(["system", "light", "dark"] as const).map((t) => (
              <button key={t} role="radio" aria-checked={s.theme === t} className={s.theme === t ? "on" : ""} onClick={() => update((x) => (x.theme = t))}>
                {t === "system" ? "Auto" : t[0].toUpperCase() + t.slice(1)}
              </button>
            ))}
          </div>
        </Item>
        <Item name="Something not working?" help="Restarting the engines fixes most problems. Logs help when reporting a bug.">
          <button className="btn" onClick={() => api.restartEngines().then(() => toast("Restarting the engines"))}>
            <RefreshCw size={15} /> Restart
          </button>
          <button className="btn" onClick={() => api.openFolder("logs")}>
            <FolderOpen size={15} /> Logs
          </button>
        </Item>
      </Section>

      <Section title="About">
        <Item name={`Voxora${version ? ` ${version}` : ""}`} help="Free and open source. Everything runs on this computer.">
          <button className="btn" onClick={() => api.quit()}>
            <Power size={15} /> Quit Voxora
          </button>
        </Item>
        <Item
          name="Your data"
          help={`Models, history, settings and logs live in one folder. Closing the window only hides Voxora to the ${IS_MAC ? "menu bar" : "tray"}; Quit stops it. After uninstalling, delete that folder to remove everything.`}
        >
          <button className="btn" onClick={() => api.openFolder("data")}>
            <FolderOpen size={15} /> Open data folder
          </button>
        </Item>
      </Section>
    </div>
  );
}
