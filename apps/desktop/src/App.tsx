import { useEffect, useState } from "react";
import { commands } from "./bindings";
import "./App.css";

// Phase 0: proves UI -> core calls and that settings persist across restarts.
export default function App() {
  const [name, setName] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => {
    commands.settingsGet("display_name").then((r) => r.status === "ok" && setName(r.data ?? ""));
  }, []);

  async function save() {
    const r = await commands.settingsSet("display_name", name);
    setStatus(r.status === "ok" ? "Saved" : r.error.message);
  }

  return (
    <main>
      <h1>MeetContext</h1>
      <label>
        Display name <input value={name} onChange={(e) => (setName(e.target.value), setStatus(""))} />
      </label>
      <button onClick={save}>Save</button> <span className="muted">{status}</span>
    </main>
  );
}
