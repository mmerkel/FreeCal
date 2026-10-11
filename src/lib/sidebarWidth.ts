/**
 * The limits of the sidebar's width, in `rem`. The maximum depends on the
 * window; a narrow window never takes the sidebar below the minimum.
 */
export const DEFAULT_WIDTH = 14;
export const MIN_WIDTH = 10;
/** The arrow keys change the width by this much. */
export const WIDTH_STEP = 1;
/** The share of the window the sidebar may take at most. */
const MAX_SHARE = 0.4;

/** The widest the sidebar may be in a window `windowWidth` rem wide. */
export function maxWidth(windowWidth: number): number {
  return Math.max(MIN_WIDTH, windowWidth * MAX_SHARE);
}

/** `width` within the limits; `null`, or a stored value that isn't a number, is the default. */
export function shownWidth(width: number | null, windowWidth: number): number {
  const wanted = width !== null && Number.isFinite(width) ? width : DEFAULT_WIDTH;
  return Math.min(Math.max(wanted, MIN_WIDTH), maxWidth(windowWidth));
}
