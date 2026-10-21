import { useEffect, useState } from "react";
import { commands, events, type AppInfo, type JobUpdate } from "./bindings";
import "./App.css";

// Phase 0 dev panel: proves UI -> core calls, settings persistence and the job queue.
export default function App() {
  const [info, setInfo] = useState<AppInfo>();
  const [name, setName] = useState("");
  const [saved, setSaved] = useState("");
  const [failTimes, setFailTimes] = useState(2);
  const [sleepMs, setSleepMs] = useState(3000);
  const [updates, setUpdates] = useState<JobUpdate[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    commands.appInfo().then((r) => (r.status === "ok" ? setInfo(r.data) : setError(r.error.message)));
    commands.settingsGet("display_name").then((r) => r.status === "ok" && setName(r.data ?? ""));
    const unlisten = events.jobUpdated.listen((e) => setUpdates((u) => [e.payload, ...u].slice(0, 50)));
    return () => void unlisten.then((f) => f());
  }, []);

  async function save() {
    const r = await commands.settingsSet("display_name", name);
    r.status === "ok" ? setSaved("Saved") : setError(r.error.message);
  }

  async function queueJob() {
    const r = await commands.devEnqueueTestJob(failTimes, sleepMs);
    if (r.status === "error") setError(r.error.message);
  }

  return (
    <main>
      <h1>MeetContext</h1>
      {info && (
        <p className="muted">
          v{info.version} · {info.os} · {info.data_dir}
        </p>
      )}
      {error && <p className="error">{error}</p>}

      <section>
        <h2>Setting</h2>
        <label>
          Display name <input value={name} onChange={(e) => (setName(e.target.value), setSaved(""))} />
        </label>
        <button onClick={save}>Save</button> <span className="muted">{saved}</span>
      </section>

      <section>
        <h2>Test job</h2>
        <label>
          Fail first <input type="number" min={0} value={failTimes} onChange={(e) => setFailTimes(+e.target.value)} /> attempts
        </label>
        <label>
          Sleep <input type="number" min={0} step={500} value={sleepMs} onChange={(e) => setSleepMs(+e.target.value)} /> ms
        </label>
        <button onClick={queueJob}>Queue test job</button>
        <table>
          <thead>
            <tr><th>Job</th><th>Status</th><th>Attempt</th><th>Error</th></tr>
          </thead>
          <tbody>
            {updates.map((u, i) => (
              <tr key={i}>
                <td>#{u.id} {u.kind}</td>
                <td>{u.status}</td>
                <td>{u.attempts}</td>
                <td className="muted">{u.last_error}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}
