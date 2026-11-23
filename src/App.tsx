import { createContext, useCallback, useContext, useEffect, useState } from "react";
import { BookA, History as HistoryIcon, Home as HomeIcon, LayoutGrid, Settings2 } from "lucide-react";
import { api, on, type Settings, type Status } from "./lib/api";
import Home from "./pages/Home";
import History from "./pages/History";
import Dictionary from "./pages/Dictionary";
import Apps from "./pages/Apps";
import SettingsPage from "./pages/Settings";
import Setup from "./pages/Setup";
import logo from "../assets/logo.svg";

interface Ctx {
  settings: Settings;
  status: Status;
  /** Apply a change, persist it, roll back and toast on failure. */
  update: (fn: (s: Settings) => void) => void;
  toast: (msg: string, error?: boolean) => void;
}

const AppCtx = createContext<Ctx>(null!);
export const useApp = () => useContext(AppCtx);

const PAGES = [
  { id: "home", label: "Home", icon: HomeIcon },
  { id: "history", label: "History", icon: HistoryIcon },
  { id: "dictionary", label: "Dictionary", icon: BookA },
  { id: "apps", label: "Modes & apps", icon: LayoutGrid },
  { id: "settings", label: "Settings", icon: Settings2 },
] as const;
type PageId = (typeof PAGES)[number]["id"];

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<Status>({ asr: "…", llm: "…", missing: [], paused: false, gpu: null });
  const [page, setPage] = useState<PageId>("home");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<{ msg: string; error: boolean } | null>(null);

  const toast = useCallback((msg: string, error = false) => {
    setToastMsg({ msg, error });
    setTimeout(() => setToastMsg(null), error ? 5000 : 2200);
  }, []);

  const reload = useCallback(() => {
    api.getSettings().then(setSettings).catch((e) => setLoadError(String(e)));
    api.getStatus().then(setStatus).catch(() => {});
  }, []);

  useEffect(() => {
    reload();
    const offs = [on<Status>("status", (s) => setStatus((p) => ({ ...p, ...s }))), on("settings", reload)];
    return () => offs.forEach((f) => f());
  }, [reload]);

  useEffect(() => {
    const t = settings?.theme ?? "system";
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  }, [settings?.theme]);

  const update = useCallback(
    (fn: (s: Settings) => void) => {
      if (!settings) return;
      const next: Settings = structuredClone(settings);
      fn(next);
      setSettings(next);
      api.saveSettings(next).catch((e) => {
        setSettings(settings);
        toast(String(e), true);
      });
    },
    [settings, toast],
  );

  if (!settings)
    return loadError ? (
      <div className="empty" role="alert">
        Voxora could not load its settings: {loadError}
      </div>
    ) : null;
  const needsSetup = status.missing.length > 0;

  return (
    <AppCtx.Provider value={{ settings, status, update, toast }}>
      <div className="shell">
        <nav className="side" aria-label="Sections">
          <div className="brand">
            <img src={logo} alt="" />
            <span>Voxora</span>
          </div>
          {PAGES.map((p) => (
            <button key={p.id} className={`nav ${page === p.id ? "active" : ""}`} onClick={() => setPage(p.id)}>
              <p.icon size={17} />
              <span>{p.label}</span>
            </button>
          ))}
          <div className="side-foot">
            100% local · no account
            <br />
            Your voice never leaves this PC.
          </div>
        </nav>
        <main className="main">
          {needsSetup ? (
            <Setup onDone={reload} />
          ) : page === "home" ? (
            <Home go={(p) => setPage(p as PageId)} />
          ) : page === "history" ? (
            <History />
          ) : page === "dictionary" ? (
            <Dictionary />
          ) : page === "apps" ? (
            <Apps />
          ) : (
            <SettingsPage />
          )}
        </main>
      </div>
      {toastMsg && <div className={`toast ${toastMsg.error ? "err" : ""}`}>{toastMsg.msg}</div>}
    </AppCtx.Provider>
  );
}
