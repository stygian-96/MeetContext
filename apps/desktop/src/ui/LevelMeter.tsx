const HEIGHTS = [6, 11, 16, 9, 5];

/** Five-bar audio level; `level` is RMS in 0..1. */
export function LevelMeter({ label, level }: { label: string; level: number }) {
  // Speech RMS rarely passes 0.2, so scale it up before lighting bars.
  const lit = Math.min(5, Math.ceil(level * 25));
  return (
    <span className="ui-meter" role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={5} aria-valuenow={lit}>
      {HEIGHTS.map((h, i) => (
        <span key={i} className={i < lit ? "on" : ""} style={{ height: h }} />
      ))}
    </span>
  );
}
