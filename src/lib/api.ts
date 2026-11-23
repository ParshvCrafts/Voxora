// Typed bridge to the Rust commands in src-tauri/src/commands.rs. Field names mirror the serde
// (snake_case) JSON of voxora-core's Settings and Dictation.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Mode = "verbatim" | "clean" | "concise" | "professional" | "technical" | "math";
export type PasteKeys = "ctrl_v" | "ctrl_shift_v" | "shift_insert";

export interface AppRule {
  exe?: string | null;
  title_contains?: string | null;
  mode: Mode;
  paste?: PasteKeys | null;
  terminal_period?: boolean | null;
}

export interface Settings {
  hotkey: { dictation_key: string; hold_threshold_ms: number; cycle_mode_key: string; repaste_key: string };
  mode: Mode;
  auto_context: boolean;
  app_rules: AppRule[];
  dictionary: { terms: string[]; replacements: { from: string; to: string }[] };
  spoken_commands: boolean;
  asr: { model: string; language: string; port: number; use_gpu: boolean };
  llm: {
    enabled: boolean;
    managed: boolean;
    endpoint: string;
    model: string;
    port: number;
    timeout_ms: number;
    fast_path_words: number;
  };
  audio: { device: string | null; hands_free_silence_ms: number; max_recording_secs: number };
  paste: { restore_clipboard: boolean; restore_delay_ms: number };
  history: { enabled: boolean; retention_days: number };
  autostart: boolean;
  theme: "system" | "light" | "dark";
  onboarded: boolean;
}

export interface Dictation {
  id: number;
  created_ms: number;
  app_exe: string;
  mode: Mode;
  raw_text: string;
  final_text: string | null;
  status: "transcribed" | "inserted" | "clipboard_only" | "failed";
  error: string | null;
  audio_ms: number;
  asr_ms: number;
  llm_ms: number | null;
  llm_used: boolean;
  inject_ms: number | null;
  total_ms: number | null;
}

export interface Status {
  asr: string;
  llm: string;
  missing: string[];
  paused: boolean;
  gpu: string | null;
}

export interface Stats {
  count: number;
  p50_total_ms: number | null;
  p95_total_ms: number | null;
  p50_asr_ms: number | null;
  p50_llm_ms: number | null;
  words_total: number;
}

export interface SetupProgress {
  id?: string;
  label?: string;
  done?: number;
  total?: number;
  phase?: "unpacking" | "done";
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  getStatus: () => invoke<Status>("get_status"),
  listDevices: () => invoke<string[]>("list_devices"),
  listKeys: () => invoke<string[]>("list_keys"),
  history: (query: string, limit = 100, offset = 0) => invoke<Dictation[]>("history", { query, limit, offset }),
  historyDelete: (id: number) => invoke<void>("history_delete", { id }),
  historyClear: () => invoke<void>("history_clear"),
  stats: () => invoke<Stats>("stats"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  reprocess: (id: number, mode: Mode) => invoke<string>("reprocess", { id, mode }),
  install: () => invoke<void>("install"),
  restartEngines: () => invoke<void>("restart_engines"),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  openFolder: (which: "logs" | "data") => invoke<void>("open_folder", { which }),
};

export function on<T>(event: string, cb: (payload: T) => void): () => void {
  let un: UnlistenFn | undefined;
  let dead = false;
  listen<T>(event, (e) => cb(e.payload)).then((u) => (dead ? u() : (un = u)));
  return () => {
    dead = true;
    un?.();
  };
}

export const MODES: { id: Mode; label: string; badge: string; blurb: string }[] = [
  { id: "clean", label: "Clean", badge: "CLEAN", blurb: "Removes fillers and false starts, fixes punctuation. Keeps your wording." },
  { id: "technical", label: "Technical", badge: "TECH", blurb: "Keeps code identifiers exact; spoken math becomes Unicode (θᵀx + b = 0)." },
  { id: "math", label: "Math / LaTeX", badge: "MATH", blurb: "Spoken math becomes LaTeX: “sum from i equals 1 to n” → \\sum_{i=1}^{n}." },
  { id: "concise", label: "Concise", badge: "BRIEF", blurb: "Tightens to the fewest words that keep every fact and request." },
  { id: "professional", label: "Professional", badge: "PRO", blurb: "Polished, friendly prose for email and documents." },
  { id: "verbatim", label: "Verbatim", badge: "RAW", blurb: "Exactly what you said. Only your dictionary is applied; no AI rewrite." },
];

export const modeLabel = (m: Mode) => MODES.find((x) => x.id === m)?.label ?? m;
