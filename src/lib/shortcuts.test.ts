import { expect, test } from 'vitest';
import { shortcutFor, shortcutKeys } from './shortcuts';

function press(key: string, modifiers: KeyboardEventInit = {}) {
  return new KeyboardEvent('keydown', { key, ...modifiers });
}

test('a key finds its action, and Ctrl only finds the Ctrl shortcuts', () => {
  expect(shortcutFor(press('n'))).toEqual({ action: 'next', ctrl: false });
  expect(shortcutFor(press(',', { ctrlKey: true }))).toEqual({ action: 'settings', ctrl: true });
  expect(shortcutFor(press('n', { ctrlKey: true }))).toBeUndefined();
  expect(shortcutFor(press(','))).toBeUndefined();
  expect(shortcutFor(press('x'))).toBeUndefined();
});

test('Alt, Meta and a held-down key never press a shortcut', () => {
  expect(shortcutFor(press('w', { altKey: true }))).toBeUndefined();
  expect(shortcutFor(press('w', { metaKey: true }))).toBeUndefined();
  expect(shortcutFor(press('w', { repeat: true }))).toBeUndefined();
});

test('hints show an action’s first keys', () => {
  expect(shortcutKeys('next')).toBe('J');
  expect(shortcutKeys('settings')).toBe('Ctrl+,');
});
