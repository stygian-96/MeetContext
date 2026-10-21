import { useRef, useState, type KeyboardEvent } from "react";
import { commands, type Prd } from "../bindings";
import { Alert, Button, Field, Prose, Tabs } from "../ui";

type Props = { prd: Prd; onSaved: (prd: Prd) => void; onCancel: () => void };

const TABLE = "\n| Column | Column |\n| --- | --- |\n| Value | Value |\n";

/** Markdown editor with an insert toolbar and a live outline, laid out like the canvas's PRD editor. */
export function PrdEditor({ prd, onSaved, onCancel }: Props) {
  const { project } = prd;
  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description);
  const [markdown, setMarkdown] = useState(prd.markdown);
  const [view, setView] = useState<"write" | "preview">("write");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const area = useRef<HTMLTextAreaElement>(null);

  // execCommand keeps the browser's undo history, which setting the value directly would lose.
  function wrap(before: string, after = before, placeholder = "text") {
    const t = area.current!;
    t.focus();
    const selected = t.value.slice(t.selectionStart, t.selectionEnd) || placeholder;
    document.execCommand("insertText", false, before + selected + after);
  }

  function atLineStart(prefix: string) {
    const t = area.current!;
    t.focus();
    const start = t.value.lastIndexOf("\n", t.selectionStart - 1) + 1;
    t.setSelectionRange(start, start);
    document.execCommand("insertText", false, prefix);
  }

  function insert(text: string) {
    area.current!.focus();
    document.execCommand("insertText", false, text);
  }

  async function save() {
    setSaving(true);
    setError("");
    if (name.trim() !== project.name || description !== project.description) {
      const r = await commands.projectUpdate(project.id, name.trim() || project.name, description);
      if (r.status === "error") return (setError(r.error.message), setSaving(false));
    }
    const r = await commands.prdSave(project.id, markdown, project.version);
    setSaving(false);
    r.status === "ok" ? onSaved(r.data) : setError(r.error.message);
  }

  function onKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      save();
    }
  }

  const outline = markdown
    .split("\n")
    .map((line) => /^ {0,3}(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line))
    .filter((m) => m !== null)
    .map((m) => ({ level: m[1].length, text: m[2] }));

  return (
    <>
      <header className="prd-header" onKeyDown={onKeyDown}>
        <div className="prd-header-row">
          <div className="prd-crumbs"><span>Projects</span><span>/</span><strong>{project.name}</strong><span className="ui-mono" style={{ fontSize: 11, marginLeft: 6 }}>EDITING</span></div>
          <div style={{ display: "flex", gap: 8 }}>
            <Button size="small" onClick={onCancel}>Cancel</Button>
            <Button size="small" variant="primary" disabled={saving} onClick={save}>Save</Button>
          </div>
        </div>
        <input className="prd-title-input" aria-label="Project name" value={name} onChange={(e) => setName(e.target.value)} />
        <Tabs label="Editor view" tabs={[{ id: "write", label: "Write" }, { id: "preview", label: "Preview" }]} current={view} onChange={setView} />
      </header>
      <div className="prd-body" onKeyDown={onKeyDown}>
        <article className="prd-article">
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {error && <Alert>{error}</Alert>}
            {view === "write" ? (
              <>
                <div role="toolbar" aria-label="Insert" className="prd-toolbar">
                  <button onClick={() => atLineStart("## ")}>Heading</button>
                  <button aria-label="Bold" onClick={() => wrap("**")}><strong>B</strong></button>
                  <button aria-label="Italic" onClick={() => wrap("_")}><em>I</em></button>
                  <button onClick={() => wrap("[", "](https://)", "link text")}>Link</button>
                  <button onClick={() => atLineStart("- ")}>List</button>
                  <button onClick={() => insert(TABLE)}>Table</button>
                  <button onClick={() => insert("\n---\n")}>Divider</button>
                </div>
                <textarea
                  ref={area}
                  className="prd-textarea"
                  aria-label="PRD in Markdown"
                  value={markdown}
                  placeholder={"## Overview\nWhat are we building and why?\n\n## Scope\n..."}
                  onChange={(e) => setMarkdown(e.target.value)}
                />
              </>
            ) : markdown.trim() ? (
              <Prose markdown={markdown} />
            ) : (
              <span className="ui-muted">Nothing to preview yet.</span>
            )}
          </div>
        </article>
        <aside className="prd-aside" aria-label="Outline and details">
          <h2>Outline</h2>
          <div className="prd-outline">
            {outline.length === 0 && <span>Headings appear here.</span>}
            {outline.map((h, i) => (
              <span key={i} style={{ paddingLeft: (h.level - 1) * 12 }}>{h.text}</span>
            ))}
          </div>
          <h2 style={{ marginTop: 12 }}>About</h2>
          <Field label="What is this project about?">
            <textarea rows={4} value={description} onChange={(e) => setDescription(e.target.value)} />
          </Field>
          <span className="ui-muted">Ctrl+S saves.</span>
        </aside>
      </div>
    </>
  );
}
