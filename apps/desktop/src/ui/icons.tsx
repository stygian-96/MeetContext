/** Stroke icons from the design canvas; they take the text colour. */
export function CaptureIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <rect x="2" y="3" width="12" height="9" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.3" />
      <circle cx="8" cy="7.5" r="2" fill="none" stroke="currentColor" strokeWidth="1.3" />
      <path d="M5 14h6" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}

export function FolderIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 14 14" aria-hidden="true">
      <path d="M1.5 3.5h4l1.2 1.5h5.8v6.5h-11z" fill="none" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

export function PlusIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M8 3v10M3 8h10" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}
