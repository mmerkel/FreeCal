import { expect, test } from 'vitest';
import { shortcutFor, shortcutKeys } from './shortcuts';

function press(key: string, modifiers: KeyboardEventInit = {}) {
  return new KeyboardEvent('keydown', { key, ...modifiers });
}

test('a key finds its action, and Ctrl only finds the Ctrl shortcuts', () => {
  expect(shortcutFor(press('n'))).toEqual({ action: 'next', whileTyping: false });
  expect(shortcutFor(press(',', { ctrlKey: true }))).toEqual({
    action: 'settings',
    whileTyping: true,
  });
  expect(shortcutFor(press('n', { ctrlKey: true }))).toBeUndefined();
  expect(shortcutFor(press(','))).toBeUndefined();
  expect(shortcutFor(press('x'))).toBeUndefined();
});

test('Ctrl+Z undoes, Ctrl+Shift+Z and Ctrl+Y redo, with Caps Lock too, but not while typing', () => {
  const ctrl = { ctrlKey: true };
  expect(shortcutFor(press('z', ctrl))).toEqual({ action: 'undo', whileTyping: false });
  expect(shortcutFor(press('Z', ctrl))).toEqual({ action: 'undo', whileTyping: false });
  expect(shortcutFor(press('Z', { ...ctrl, shiftKey: true }))).toEqual({
    action: 'redo',
    whileTyping: false,
  });
  expect(shortcutFor(press('y', ctrl))).toEqual({ action: 'redo', whileTyping: false });
  expect(shortcutFor(press('z'))).toBeUndefined();
  expect(shortcutKeys('redo')).toBe('Ctrl+Shift+Z');
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
