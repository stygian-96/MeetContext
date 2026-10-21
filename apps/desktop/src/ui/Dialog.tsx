import { useEffect, useRef, type ReactNode } from "react";

type Props = { title: string; tag?: string; onClose: () => void; footer: ReactNode; children: ReactNode };

/** Modal dialog on the native <dialog>: focus trap and Esc come free. */
export function Dialog({ title, tag, onClose, footer, children }: Props) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => ref.current?.showModal(), []);
  return (
    <dialog ref={ref} className="ui-dialog" aria-labelledby="ui-dialog-title" onClose={onClose}>
      <div className="ui-dialog-head">
        <h2 id="ui-dialog-title">{title}</h2>
        {tag && <span className="ui-mono ui-muted" style={{ fontSize: 11 }}>{tag}</span>}
        <button className="ui-dialog-close" aria-label="Close" onClick={onClose}>×</button>
      </div>
      <div className="ui-dialog-body">{children}</div>
      <div className="ui-dialog-foot">{footer}</div>
    </dialog>
  );
}
