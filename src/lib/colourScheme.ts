/**
 * Keeps `data-color-scheme` on `root` in line with the system's light or dark
 * style. The tokens in `app.css` and FullCalendar's palettes switch on this
 * one attribute. Returns a function that stops following.
 */
export function followSystemColourScheme(
  root: HTMLElement = document.documentElement,
  darkQuery: MediaQueryList = matchMedia('(prefers-color-scheme: dark)'),
): () => void {
  const apply = () => {
    root.dataset.colorScheme = darkQuery.matches ? 'dark' : 'light';
  };
  apply();
  darkQuery.addEventListener('change', apply);
  return () => darkQuery.removeEventListener('change', apply);
}
