import type { ReactNode } from "react";
import { OWN_TITLE_BAR, WindowControls } from "./WindowControls";

type Props = {
  /** Nav items under the brand. */
  sidebar: ReactNode;
  /** No padding: the page draws its own full-width header (like the PRD page). */
  flush?: boolean;
  children: ReactNode;
};

/** Sidebar + main area; the top strip doubles as the window's title bar. */
export function AppShell({ sidebar, flush, children }: Props) {
  return (
    <div className="ui-shell">
      <nav aria-label="Primary" className="ui-sidebar">
        <div data-tauri-drag-region className="ui-brand"><span className="ui-brand-mark">Δ</span><span className="ui-label">MeetContext</span></div>
        {sidebar}
      </nav>
      <div className={`ui-shell-main ${flush ? "ui-shell-flush" : ""}`}>
        <div data-tauri-drag-region className="ui-titlebar">{OWN_TITLE_BAR && <WindowControls />}</div>
        <main className={flush ? "ui-main-flush" : "ui-main"}>{children}</main>
      </div>
    </div>
  );
}
