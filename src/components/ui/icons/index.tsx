/** Line icons drawn for this app: 24×24, `currentColor`, 1.75 stroke. */

const STROKE = {
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.75,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const;

/** Settings: a gear with 8 teeth (generated geometry, symmetric). */
export function GearIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-[18px]" aria-hidden {...STROKE}>
      <path d="M10.07 5.27L10.33 2.55A9.6 9.6 0 0 1 13.67 2.55L13.93 5.27A7 7 0 0 1 15.39 5.88L17.51 4.14A9.6 9.6 0 0 1 19.86 6.49L18.12 8.61A7 7 0 0 1 18.73 10.07L21.45 10.33A9.6 9.6 0 0 1 21.45 13.67L18.73 13.93A7 7 0 0 1 18.12 15.39L19.86 17.51A9.6 9.6 0 0 1 17.51 19.86L15.39 18.12A7 7 0 0 1 13.93 18.73L13.67 21.45A9.6 9.6 0 0 1 10.33 21.45L10.07 18.73A7 7 0 0 1 8.61 18.12L6.49 19.86A9.6 9.6 0 0 1 4.14 17.51L5.88 15.39A7 7 0 0 1 5.27 13.93L2.55 13.67A9.6 9.6 0 0 1 2.55 10.33L5.27 10.07A7 7 0 0 1 5.88 8.61L4.14 6.49A9.6 9.6 0 0 1 6.49 4.14L8.61 5.88A7 7 0 0 1 10.07 5.27Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

/** Clear: a trash can. */
export function TrashIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-[18px]" aria-hidden {...STROKE}>
      <path d="M4 7h16M9 7V4.5h6V7M6.5 7l.9 12.1a1.6 1.6 0 0 0 1.6 1.4h6a1.6 1.6 0 0 0 1.6-1.4L17.5 7M10 11v6M14 11v6" />
    </svg>
  );
}

export function CloseIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-[18px]" aria-hidden {...STROKE}>
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
  );
}
