import type { ReactNode } from "react";

type Props = { checked: boolean; onChange: (checked: boolean) => void; disabled?: boolean; boxed?: boolean; children: ReactNode };

export function Checkbox({ checked, onChange, disabled, boxed, children }: Props) {
  return (
    <label className={`ui-checkbox ${boxed ? "ui-checkbox-boxed" : ""}`}>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span>{children}</span>
    </label>
  );
}
