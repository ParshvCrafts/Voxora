import type { ReactNode } from "react";

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="switch" title={label}>
      <input type="checkbox" checked={checked} aria-label={label} onChange={(e) => onChange(e.target.checked)} />
      <span />
    </label>
  );
}

export function Field({ name, help, children }: { name: string; help?: ReactNode; children: ReactNode }) {
  return (
    <div className="field">
      <div>
        <div className="name">{name}</div>
        {help && <div className="help">{help}</div>}
      </div>
      <div style={{ display: "flex", justifyContent: "flex-end" }}>{children}</div>
    </div>
  );
}

export function NumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  suffix,
}: {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
}) {
  return (
    <div className="row" style={{ gap: 8, width: "100%" }}>
      <input
        className="input"
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ padding: 0, border: 0, boxShadow: "none", accentColor: "var(--accent)" }}
      />
      <span className="mono" style={{ minWidth: 64, textAlign: "right" }}>
        {value}
        {suffix}
      </span>
    </div>
  );
}

export function StatusPill({ label, value }: { label: string; value: string }) {
  const tone = value.startsWith("ready")
    ? "good"
    : value.startsWith("error")
      ? "bad"
      : value === "off" || value === "stopped"
        ? ""
        : "warn";
  return (
    <span className={`pill ${tone}`} title={value}>
      <span className="led" />
      {label}: {value.length > 28 ? value.slice(0, 28) + "…" : value}
    </span>
  );
}
