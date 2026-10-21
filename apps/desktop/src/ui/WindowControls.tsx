import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

// Only Windows uses our frameless window (tauri.windows.conf.json); macOS keeps its own controls.
export const OWN_TITLE_BAR = "__TAURI_INTERNALS__" in window && navigator.userAgent.includes("Windows");

/** Minimise, maximise/restore and close, drawn in the app like Claude or Cursor do. */
export function WindowControls() {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    win.isMaximized().then(setMaximized);
    const unlisten = win.onResized(() => win.isMaximized().then(setMaximized));
    return () => void unlisten.then((f) => f());
  }, []);

  return (
    <div className="ui-window-controls">
      <button aria-label="Minimise" onClick={() => win.minimize()}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" stroke="currentColor" /></svg>
      </button>
      <button aria-label={maximized ? "Restore" : "Maximise"} onClick={() => win.toggleMaximize()}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          {maximized ? (
            <path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" fill="none" stroke="currentColor" />
          ) : (
            <rect x=".5" y=".5" width="9" height="9" fill="none" stroke="currentColor" />
          )}
        </svg>
      </button>
      <button aria-label="Close" className="ui-window-close" onClick={() => win.close()}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" /></svg>
      </button>
    </div>
  );
}
