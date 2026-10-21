import type { ReactNode } from "react";

export type PillTone = "neutral" | "blue" | "good" | "record" | "warn";

/** A status label; the tone adds colour, the text always says it too. */
export function Pill({ tone = "neutral", children }: { tone?: PillTone; children: ReactNode }) {
  return <span className={`ui-pill ui-pill-${tone}`}>{children}</span>;
}
