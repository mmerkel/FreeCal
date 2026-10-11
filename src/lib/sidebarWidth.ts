/**
 * The limits of the sidebar's width, in `rem`. They don't depend on the
 * window: a narrow window never takes the sidebar below the minimum.
 */
const DEFAULT_WIDTH = 12;
const MIN_WIDTH = 8;
export const MAX_WIDTH = 20;
/** A drag that takes the sidebar's edge left of this hides the sidebar. */
const HIDE_BELOW = 4;
/** The arrow keys change the width by this much. */
export const WIDTH_STEP = 1;

/** `width` within the limits; `null`, or a stored value that isn't a number, is the default. */
export function shownWidth(width: number | null): number {
  const wanted = width !== null && Number.isFinite(width) ? width : DEFAULT_WIDTH;
  return Math.min(Math.max(wanted, MIN_WIDTH), MAX_WIDTH);
}

/**
 * The width a drag that takes the sidebar's edge `edge` rem from the window's
 * left edge shows it at; `undefined` when it hides the sidebar.
 */
export function draggedWidth(edge: number): number | undefined {
  return edge < HIDE_BELOW ? undefined : shownWidth(edge);
}
