import { expect, test } from 'vitest';
import { FakeDarkQuery } from '../test/darkQuery';
import { followAppearance } from './colourScheme';

function follow(appearance: 'system' | 'light' | 'dark', systemDark: boolean) {
  const root = document.createElement('html');
  const query = new FakeDarkQuery(systemDark);
  const stop = followAppearance(appearance, root, query as unknown as MediaQueryList);
  return { root, query, stop };
}

test('System follows the system’s light or dark style until stopped', () => {
  const { root, query, stop } = follow('system', false);
  expect(root.dataset.colorScheme).toBe('light');

  query.flip(true);
  expect(root.dataset.colorScheme).toBe('dark');

  stop();
  query.flip(false);
  expect(root.dataset.colorScheme).toBe('dark');
});

test('Light and Dark stay as chosen whatever the system does', () => {
  const dark = follow('dark', false);
  expect(dark.root.dataset.colorScheme).toBe('dark');
  dark.query.flip(true);
  dark.query.flip(false);
  expect(dark.root.dataset.colorScheme).toBe('dark');

  const light = follow('light', true);
  expect(light.root.dataset.colorScheme).toBe('light');
  light.query.flip(false);
  light.query.flip(true);
  expect(light.root.dataset.colorScheme).toBe('light');
});
