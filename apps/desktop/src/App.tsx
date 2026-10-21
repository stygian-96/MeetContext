import { useEffect, useState } from "react";
import { commands, type Project } from "./bindings";
import { Captures } from "./screens/Captures";
import { NewProject } from "./screens/NewProject";
import { Overlay } from "./screens/Overlay";
import { ProjectPage } from "./screens/ProjectPage";
import { AppShell, NavHeading, NavItem } from "./ui";
import { CaptureIcon, FolderIcon, PlusIcon } from "./ui/icons";

type Route = { screen: "captures" } | { screen: "project"; id: number; edit?: boolean };

export default function App() {
  return new URLSearchParams(location.search).get("view") === "overlay" ? <Overlay /> : <Main />;
}

function Main() {
  const [route, setRoute] = useState<Route>({ screen: "captures" });
  const [projects, setProjects] = useState<Project[]>([]);
  const [creating, setCreating] = useState(false);

  const reloadProjects = () => commands.projectList().then((r) => r.status === "ok" && setProjects(r.data));
  useEffect(() => void reloadProjects(), []);

  const sidebar = (
    <>
      <NavItem icon={<CaptureIcon />} label="Captures" current={route.screen === "captures"} onClick={() => setRoute({ screen: "captures" })} />
      <NavHeading>Projects</NavHeading>
      {projects.map((p) => (
        <NavItem
          key={p.id}
          icon={<FolderIcon />}
          label={p.name}
          badge={p.version === 0 ? "draft" : undefined}
          current={route.screen === "project" && route.id === p.id}
          onClick={() => setRoute({ screen: "project", id: p.id })}
        />
      ))}
      <NavItem icon={<PlusIcon />} label="New project" dashed onClick={() => setCreating(true)} />
    </>
  );

  return (
    <AppShell sidebar={sidebar} flush={route.screen === "project"}>
      {route.screen === "captures" ? (
        <Captures />
      ) : (
        <ProjectPage key={route.id} id={route.id} startEditing={route.edit} onChanged={reloadProjects} />
      )}
      {creating && (
        <NewProject
          onClose={() => setCreating(false)}
          onCreated={(id) => {
            setCreating(false);
            reloadProjects();
            setRoute({ screen: "project", id, edit: true });
          }}
        />
      )}
    </AppShell>
  );
}
