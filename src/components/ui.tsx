import type { ReactNode } from "react";

/** A titled block of grouped rows, like a section of macOS System Settings. */
export function Section({
  title,
  aside,
  footer,
  children,
  pad,
}: {
  title?: ReactNode;
  aside?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  pad?: boolean;
}) {
  return (
    <section className="section">
      {(title || aside) && (
        <div className="section-head">
          {title && <h2>{title}</h2>}
          {aside && <div className="aside">{aside}</div>}
        </div>
      )}
      <div className={`group ${pad ? "pad" : ""}`}>{children}</div>
      {footer && <p className="section-foot">{footer}</p>}
    </section>
  );
}

/** One row: label and help on the left, the control on the right. */
export function Item({ name, help, children }: { name: ReactNode; help?: ReactNode; children?: ReactNode }) {
  return (
    <div className="item">
      <div className="label">
        <div className="name">{name}</div>
        {help && <div className="help">{help}</div>}
      </div>
      {children && <div className="control">{children}</div>}
    </div>
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <span className="switch">
      <input type="checkbox" role="switch" checked={checked} aria-label={label} onChange={(e) => onChange(e.target.checked)} />
      <span />
    </span>
  );
}

export function Range({
  value,
  onChange,
  min,
  max,
  step = 1,
  unit,
  label,
}: {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step?: number;
  unit: string;
  label: string;
}) {
  return (
    <div className="range">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <output>
        {value} {unit}
      </output>
    </div>
  );
}

export function Keys({ keys }: { keys: string[] }) {
  return (
    <span className="keys">
      {keys.map((k, i) => (
        <span key={k} className="keys">
          {i > 0 && "+"}
          <kbd>{k}</kbd>
        </span>
      ))}
    </span>
  );
}

/** "ready", "starting…", "error: …" from the engine → a colored dot and a short human label. */
export function EngineStatus({ label, value }: { label: string; value: string }) {
  const tone = value.startsWith("ready")
    ? "good"
    : value.startsWith("error")
      ? "bad"
      : value === "off" || value === "stopped"
        ? ""
        : "warn";
  const text = value.startsWith("ready") ? "Ready" : value === "off" ? "Off" : value.startsWith("error") ? "Needs attention" : "Starting";
  return (
    <span title={value}>
      <span className={`dot ${tone}`} />
      {label} {text.toLowerCase()}
    </span>
  );
}

export function PageHead({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <header className="page-head">
      <h1>{title}</h1>
      {children && <p>{children}</p>}
    </header>
  );
}
