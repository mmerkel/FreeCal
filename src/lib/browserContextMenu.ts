/**
 * Turns off the webview's own context menu on right-click, everywhere in
 * `root`, text fields included, so that FreeCal looks the same wherever it
 * is clicked. It listens in the capture phase, so no handler can stop it by
 * stopping the right-click from spreading. FreeCal's own right-click handlers
 * still run. Returns a function that turns the menu back on.
 */
export function turnOffBrowserContextMenu(root: EventTarget = window): () => void {
  const prevent = (clicked: Event) => clicked.preventDefault();
  root.addEventListener('contextmenu', prevent, true);
  return () => root.removeEventListener('contextmenu', prevent, true);
}
