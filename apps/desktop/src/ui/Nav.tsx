import type { ReactNode } from "react";

type ItemProps = { icon?: ReactNode; label: string; current?: boolean; badge?: ReactNode; dashed?: boolean; onClick: () => void };

/** A sidebar link; the label hides when the sidebar collapses to a rail. `dashed` is for "add" actions. */
export function NavItem({ icon, label, current, badge, dashed, onClick }: ItemProps) {
  return (
    <button className={`ui-nav-item ${dashed ? "ui-nav-dashed" : ""}`} aria-current={current ? "page" : undefined} title={label} onClick={onClick}>
      {icon}
      <span className="ui-label ui-nav-text">{label}</span>
      {badge && <span className="ui-label ui-nav-badge">{badge}</span>}
    </button>
  );
}

/** Small mono caps heading between nav groups. */
export function NavHeading({ children }: { children: ReactNode }) {
  return <div className="ui-label ui-nav-heading">{children}</div>;
}
