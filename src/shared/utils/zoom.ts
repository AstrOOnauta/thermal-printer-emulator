/** Paper zoom steps in percent, as `settings::ZOOM_STEPS` in Rust. */
export const ZOOM_STEPS = [75, 100, 125, 150, 200] as const;

/** The next step up or down from `zoom`, clamped at the ends. */
export function stepZoom(zoom: number, direction: 1 | -1): number {
  const index = ZOOM_STEPS.findIndex((step) => step >= zoom);
  const current = index === -1 ? ZOOM_STEPS.length - 1 : index;
  const next = Math.min(
    ZOOM_STEPS.length - 1,
    Math.max(0, current + direction),
  );
  return ZOOM_STEPS[next] ?? 100;
}
