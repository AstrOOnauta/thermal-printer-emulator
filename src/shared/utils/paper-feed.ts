/**
 * How long new paper takes to come out. A thermal printer feeds about 1200 dots per
 * second, and one dot is one CSS pixel here; very short or very long receipts are clamped
 * so the motion stays readable. The CSS reveal (`.paper-print`) and the list's scroll use
 * the same duration and curve, which keeps the printed edge on the window's bottom edge.
 */
const DOTS_PER_MS = 1.2;
export const FEED_MIN_MS = 300;
export const FEED_MAX_MS = 1400;

export function feedDuration(pixels: number): number {
  const duration = pixels / DOTS_PER_MS;
  return Math.round(Math.min(FEED_MAX_MS, Math.max(FEED_MIN_MS, duration)));
}

/** Same curve as `--ease-out-cubic` in globals.css: cubic-bezier(0.33, 1, 0.68, 1). */
export function easeOutCubic(progress: number): number {
  return 1 - (1 - progress) ** 3;
}

export function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}
