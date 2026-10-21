import { useEffect, useState } from "react";
import { commands, events } from "../bindings";
import { formatDuration } from "../format";
import { Button } from "../ui";

/** The floating capture bar; drag it by any empty space. */
export function Overlay() {
  const [elapsed, setElapsed] = useState(0);
  const [stopping, setStopping] = useState(false);

  useEffect(() => {
    // The window is transparent so only the pill shows.
    document.documentElement.style.background = document.body.style.background = "transparent";
    const unlisten = events.captureTick.listen((e) => setElapsed(e.payload.elapsed_ms));
    return () => void unlisten.then((f) => f());
  }, []);

  return (
    <div data-tauri-drag-region style={{ height: "100vh", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div
        data-tauri-drag-region
        style={{ display: "flex", alignItems: "center", gap: 14, background: "var(--surface)", border: "1px solid var(--line-strong)", borderRadius: 999, padding: "6px 6px 6px 16px", fontSize: 13, whiteSpace: "nowrap", boxShadow: "0 2px 8px rgba(0,0,0,.2)" }}
      >
        <span data-tauri-drag-region className="ui-rec-dot" />
        <strong data-tauri-drag-region style={{ fontWeight: 600, color: "var(--record-ink)" }}>Capturing</strong>
        <span data-tauri-drag-region className="ui-mono">{formatDuration(elapsed)}</span>
        <Button variant="primary" pill disabled={stopping} onClick={() => (setStopping(true), commands.captureStop())}>
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><rect x="1" y="1" width="8" height="8" rx="1.5" fill="currentColor" /></svg>
          Stop capture
        </Button>
      </div>
    </div>
  );
}
