import { useEffect, useState } from "react";
import { commands, events, type Capture } from "../bindings";
import { formatDuration } from "../format";
import { Button, Pill, type PillTone } from "../ui";
import { NewCapture } from "./NewCapture";
import "./Captures.css";

const STATUS: Record<string, [PillTone, string]> = {
  recording: ["record", "Recording"],
  ready: ["good", "Recorded"],
  failed: ["record", "Failed"],
  interrupted: ["warn", "Interrupted"],
};

function when(ms: number) {
  const d = new Date(ms);
  const day = d.toDateString() === new Date().toDateString() ? "Today" : d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  return `${day} · ${d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
}

export function Captures() {
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [creating, setCreating] = useState(false);

  const reload = () => commands.captureList().then((r) => r.status === "ok" && setCaptures(r.data));

  useEffect(() => {
    reload();
    const unlisten = events.captureState.listen(reload);
    return () => void unlisten.then((f) => f());
  }, []);

  async function remove(id: number) {
    if ((await commands.captureDelete(id)).status === "ok") reload();
  }

  return (
    <>
      <header style={{ display: "flex", flexWrap: "wrap", gap: 16, alignItems: "flex-end", justifyContent: "space-between" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <h1>Captures</h1>
          <span className="ui-muted" style={{ fontSize: 13.5 }}>Every recorded meeting.</span>
        </div>
        <Button variant="primary" onClick={() => setCreating(true)}>
          <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#f08080" }} />
          New capture
        </Button>
      </header>

      <section aria-label="Captures" className="ui-card captures-list">
        <div>
          <div className="captures-row captures-head">
            <span>Meeting</span><span>Length</span><span className="captures-captured">Captured</span><span>Status</span><span />
          </div>
          {captures.length === 0 && <div className="ui-muted" style={{ padding: "32px 20px", fontSize: 13.5 }}>No captures yet. Start one with New capture.</div>}
          {captures.map((c) => {
            const [tone, label] = STATUS[c.state] ?? ["neutral", c.state];
            return (
              <div key={c.id} className="captures-row">
                <span className="captures-name">
                  <strong title={c.name}>{c.name}</strong>
                  <span className="ui-muted" style={{ fontSize: 12.5 }}>{when(c.started_at)}</span>
                </span>
                <span className="ui-mono" style={{ fontSize: 13 }}>{c.duration_ms == null ? "—" : formatDuration(c.duration_ms)}</span>
                <span className="ui-muted captures-captured">
                  <span>Audio</span>
                  <span>{c.chunks} chunks · {(c.bytes / 1e6).toFixed(1)} MB</span>
                </span>
                <span style={{ display: "flex", flexDirection: "column", gap: 4, alignItems: "flex-start" }}>
                  <Pill tone={tone}>{label}</Pill>
                  {c.error && <span className="ui-muted" style={{ fontSize: 12 }}>{c.error}</span>}
                </span>
                <span>
                  <Button size="small" disabled={c.state === "recording"} onClick={() => remove(c.id)}>Delete</Button>
                </span>
              </div>
            );
          })}
        </div>
      </section>

      {creating && <NewCapture onClose={() => setCreating(false)} />}
    </>
  );
}
