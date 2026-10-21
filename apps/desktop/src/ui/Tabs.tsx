type Props<T extends string> = { label: string; tabs: { id: T; label: string }[]; current: T; onChange: (id: T) => void };

/** Underlined tab row, as in the PRD page header. */
export function Tabs<T extends string>({ label, tabs, current, onChange }: Props<T>) {
  return (
    <div role="tablist" aria-label={label} className="ui-tabs">
      {tabs.map((t) => (
        <button key={t.id} role="tab" aria-selected={t.id === current} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}
