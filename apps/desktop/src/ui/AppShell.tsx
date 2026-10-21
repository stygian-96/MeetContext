import type { ReactNode } from "react";
import { OWN_TITLE_BAR, WindowControls } from "./WindowControls";

/** Sidebar + main area; the top strip doubles as the window's title bar. Phase 2 adds projects. */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="ui-shell">
      <nav aria-label="Primary" className="ui-sidebar">
        <div data-tauri-drag-region className="ui-brand"><span className="ui-brand-mark">Δ</span><span className="ui-label">MeetContext</span></div>
        <a href="#" className="ui-nav-item" aria-current="page" title="Captures">
          <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
            <rect x="2" y="3" width="12" height="9" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.3" />
            <circle cx="8" cy="7.5" r="2" fill="none" stroke="currentColor" strokeWidth="1.3" />
            <path d="M5 14h6" stroke="currentColor" strokeWidth="1.3" />
          </svg>
          <span className="ui-label">Captures</span>
        </a>
      </nav>
      <div className="ui-shell-main">
        <div data-tauri-drag-region className="ui-titlebar">{OWN_TITLE_BAR && <WindowControls />}</div>
        <main className="ui-main">{children}</main>
      </div>
    </div>
  );
}
