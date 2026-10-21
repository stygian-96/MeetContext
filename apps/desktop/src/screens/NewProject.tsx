import { useState } from "react";
import { commands } from "../bindings";
import { Alert, Button, Dialog, Field } from "../ui";

/** Names a project; its PRD is written next, in the editor. */
export function NewProject({ onClose, onCreated }: { onClose: () => void; onCreated: (id: number) => void }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState("");

  async function create() {
    const r = await commands.projectCreate(name.trim(), description.trim());
    r.status === "ok" ? onCreated(r.data) : setError(r.error.message);
  }

  const footer = (
    <>
      <span style={{ flex: 1 }} />
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="primary" disabled={!name.trim()} onClick={create}>Create and write PRD</Button>
    </>
  );

  return (
    <Dialog title="New project" onClose={onClose} footer={footer}>
      {error && <Alert>{error}</Alert>}
      <Field label="Project name">
        <input autoFocus value={name} placeholder="Checkout" onChange={(e) => setName(e.target.value)} />
      </Field>
      <Field label="What is it about? (helps the AI send meeting decisions here)">
        <textarea rows={3} value={description} placeholder="One-page checkout for web and iOS: cart, shipping, payment." onChange={(e) => setDescription(e.target.value)} />
      </Field>
    </Dialog>
  );
}
