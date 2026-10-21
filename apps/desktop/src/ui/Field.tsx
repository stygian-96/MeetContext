import type { ReactNode } from "react";

/** A label above its control (input, select). */
export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="ui-field">
      <span>{label}</span>
      {children}
    </label>
  );
}
