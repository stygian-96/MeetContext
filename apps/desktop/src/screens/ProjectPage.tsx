import { useEffect, useState } from "react";
import { commands, type Prd } from "../bindings";
import { Alert, Button, Prose, Tabs } from "../ui";
import { PrdEditor } from "./PrdEditor";
import "./Project.css";

type Props = { id: number; startEditing?: boolean; onChanged: () => void };

/** A project's PRD in the reading view; Edit switches to the Markdown editor. */
export function ProjectPage({ id, startEditing, onChanged }: Props) {
  const [prd, setPrd] = useState<Prd>();
  const [editing, setEditing] = useState(!!startEditing);
  const [error, setError] = useState("");

  useEffect(() => {
    commands.prdGet(id).then((r) => (r.status === "ok" ? setPrd(r.data) : setError(r.error.message)));
  }, [id]);

  if (error) return <div style={{ padding: 40 }}><Alert>{error}</Alert></div>;
  if (!prd) return null;
  if (editing) {
    return (
      <PrdEditor
        prd={prd}
        onCancel={() => setEditing(false)}
        onSaved={(p) => {
          setPrd(p);
          setEditing(false);
          onChanged();
        }}
      />
    );
  }

  const { project, markdown } = prd;
  return (
    <>
      <header className="prd-header">
        <div className="prd-header-row">
          <div className="prd-crumbs"><span>Projects</span><span>/</span><strong>{project.name}</strong></div>
          <Button size="small" onClick={() => setEditing(true)}>Edit PRD</Button>
        </div>
        <h1 className="prd-title">{project.name}</h1>
        <Tabs label="PRD views" tabs={[{ id: "document", label: "Document" }]} current="document" onChange={() => {}} />
      </header>
      <div className="prd-body">
        <article className="prd-article">
          {markdown.trim() ? (
            <Prose markdown={markdown} />
          ) : (
            <div className="prd-empty">
              <span>This project has no PRD yet.</span>
              <Button variant="primary" onClick={() => setEditing(true)}>Write the PRD</Button>
            </div>
          )}
        </article>
        <aside className="prd-aside" aria-label="About this PRD">
          <h2>About</h2>
          <span>{project.description || <span className="ui-muted">No description. Add one so the AI knows which meetings belong here.</span>}</span>
          <span className="ui-muted">
            {project.version === 0 ? "Not saved yet" : `Version ${project.version} · saved ${new Date(project.updated_at).toLocaleString()}`}
          </span>
          <span className="ui-muted">Written here · no connected source</span>
        </aside>
      </div>
    </>
  );
}
