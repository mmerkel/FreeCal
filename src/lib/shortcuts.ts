/**
 * FreeCal's keyboard shortcuts, in one table that the key handling and the
 * hints in tooltips both read. The letters are Google Calendar's.
 */
import { t } from '../i18n';

export type ShortcutAction =
  'create' | 'day' | 'week' | 'month' | 'next' | 'previous' | 'today' | 'settings' | 'sidebar';

interface Shortcut {
  /** `KeyboardEvent.key`. */
  key: string;
  /** Pressed together with Ctrl. Such a shortcut also acts while typing. */
  ctrl?: boolean;
}

/** Each action's keys; hints show the first one. */
const SHORTCUTS: Record<ShortcutAction, readonly Shortcut[]> = {
  create: [{ key: 'c' }],
  day: [{ key: 'd' }, { key: '1' }],
  week: [{ key: 'w' }, { key: '2' }],
  month: [{ key: 'm' }, { key: '3' }],
  next: [{ key: 'j' }, { key: 'n' }],
  previous: [{ key: 'k' }, { key: 'p' }],
  today: [{ key: 't' }],
  settings: [{ key: ',', ctrl: true }],
  sidebar: [{ key: 'F9' }],
};

/** The shortcut `key` presses, if any. Alt, Meta and held-down keys never count. */
export function shortcutFor(
  key: KeyboardEvent,
): { action: ShortcutAction; ctrl: boolean } | undefined {
  if (key.altKey || key.metaKey || key.repeat) return undefined;
  for (const [action, shortcuts] of Object.entries(SHORTCUTS) as [ShortcutAction, Shortcut[]][]) {
    const found = shortcuts.find(
      (shortcut) => shortcut.key === key.key && Boolean(shortcut.ctrl) === key.ctrlKey,
    );
    if (found) return { action, ctrl: Boolean(found.ctrl) };
  }
  return undefined;
}

/** The keys of an action as a hint shows them, such as `D` or `Ctrl+,`. */
export function shortcutKeys(action: ShortcutAction): string {
  const [first] = SHORTCUTS[action];
  const key = first.key.toUpperCase();
  return first.ctrl ? t('shortcut.ctrl', { key }) : key;
}

/** A tooltip naming an action and its keys. */
export function shortcutHint(label: string, action: ShortcutAction): string {
  return t('shortcut.hint', { action: label, keys: shortcutKeys(action) });
}
