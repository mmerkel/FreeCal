import type { Appearance } from '../core/types';

/**
 * Sets `data-color-scheme` on `root` to the chosen appearance; with
 * `system`, keeps it in line with the system's light or dark style. The
 * tokens in `app.css` and FullCalendar's palettes switch on this one
 * attribute. Returns a function that stops following.
 */
export function followAppearance(
  appearance: Appearance,
  root: HTMLElement = document.documentElement,
  darkQuery: MediaQueryList = matchMedia('(prefers-color-scheme: dark)'),
): () => void {
  if (appearance !== 'system') {
    root.dataset.colorScheme = appearance;
    return () => {};
  }
  const apply = () => {
    root.dataset.colorScheme = darkQuery.matches ? 'dark' : 'light';
  };
  apply();
  darkQuery.addEventListener('change', apply);
  return () => darkQuery.removeEventListener('change', apply);
}
