/** Hover/active/disabled feedback for anything clickable. */
export const INTERACTIVE =
  'cursor-pointer transition-opacity hover:opacity-80 active:opacity-60 disabled:cursor-not-allowed disabled:opacity-40';

/** A bordered secondary button. */
export const BUTTON = `rounded-md border border-border px-3 py-1.5 text-sm text-ink ${INTERACTIVE}`;

/** Text inputs and selects. */
export const FIELD =
  'rounded-md border border-field-border bg-transparent px-3 py-1.5 text-sm text-ink outline-none focus:border-accent';
