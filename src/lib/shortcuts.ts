/**
 * FreeCal's keyboard shortcuts, in one table that the key handling and the
 * hints in tooltips both read. The letters are Google Calendar's.
 */
import { t } from '../i18n';

export type ShortcutAction =
  | 'create'
  | 'day'
  | 'week'
  | 'month'
  | 'next'
  | 'previous'
  | 'today'
  | 'sync'
  | 'settings'
  | 'sidebar'
  | 'undo'
  | 'redo';

interface Shortcut {
  /** `KeyboardEvent.key`. */
  key: string;
  /**
   * Pressed together with Ctrl. Such a shortcut also acts while typing,
   * unless a text field has it too. Its letter counts in either case.
   */
  ctrl?: boolean;
  /** Pressed together with Shift too; only for a shortcut with Ctrl. */
  shift?: boolean;
}

/** The shortcuts a text field has too, which don't act while typing. */
const TEXT_FIELD_ACTIONS: readonly ShortcutAction[] = ['undo', 'redo'];

/** Each action's keys; hints show the first one. */
const SHORTCUTS: Record<ShortcutAction, readonly Shortcut[]> = {
  create: [{ key: 'c' }],
  day: [{ key: 'd' }, { key: '1' }],
  week: [{ key: 'w' }, { key: '2' }],
  month: [{ key: 'm' }, { key: '3' }],
  next: [{ key: 'j' }, { key: 'n' }],
  previous: [{ key: 'k' }, { key: 'p' }],
  today: [{ key: 't' }],
  // Google Calendar's "Refresh".
  sync: [{ key: 'r' }],
  settings: [{ key: ',', ctrl: true }],
  sidebar: [{ key: 'F9' }],
  undo: [{ key: 'z', ctrl: true }],
  redo: [
    { key: 'z', ctrl: true, shift: true },
    { key: 'y', ctrl: true },
  ],
};

/**
 * The shortcut `key` presses, if any, and whether it acts while typing.
 * Alt, Meta and held-down keys never count.
 */
export function shortcutFor(
  key: KeyboardEvent,
): { action: ShortcutAction; whileTyping: boolean } | undefined {
  if (key.altKey || key.metaKey || key.repeat) return undefined;
  for (const [action, shortcuts] of Object.entries(SHORTCUTS) as [ShortcutAction, Shortcut[]][]) {
    const found = shortcuts.find((shortcut) =>
      shortcut.ctrl
        ? key.ctrlKey &&
          key.key.toLowerCase() === shortcut.key &&
          Boolean(shortcut.shift) === key.shiftKey
        : !key.ctrlKey && key.key === shortcut.key,
    );
    if (found) {
      return { action, whileTyping: Boolean(found.ctrl) && !TEXT_FIELD_ACTIONS.includes(action) };
    }
  }
  return undefined;
}

/** The keys of an action as a hint shows them, such as `D` or `Ctrl+,`. */
export function shortcutKeys(action: ShortcutAction): string {
  const [first] = SHORTCUTS[action];
  const key = first.key.toUpperCase();
  if (first.shift) return t('shortcut.ctrlShift', { key });
  return first.ctrl ? t('shortcut.ctrl', { key }) : key;
}

/** A tooltip naming an action and its keys. */
export function shortcutHint(label: string, action: ShortcutAction): string {
  return t('shortcut.hint', { action: label, keys: shortcutKeys(action) });
}
