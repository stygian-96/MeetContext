import type { ReactNode } from "react";

export function Alert({ children }: { children: ReactNode }) {
  return <div role="alert" className="ui-alert">{children}</div>;
}
