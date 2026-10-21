import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** A PRD rendered in the serif reading style (Markdown with tables and task lists). */
export function Prose({ markdown }: { markdown: string }) {
  return (
    <div className="ui-prose">
      <Markdown remarkPlugins={[remarkGfm]}>{markdown}</Markdown>
    </div>
  );
}
