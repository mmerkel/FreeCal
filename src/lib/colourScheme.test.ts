import { expect, test } from 'vitest';
import { followSystemColourScheme } from './colourScheme';

/** A stand-in for `matchMedia('(prefers-color-scheme: dark)')` that a test can flip. */
class FakeDarkQuery extends EventTarget {
  constructor(public matches: boolean) {
    super();
  }

  flip(matches: boolean) {
    this.matches = matches;
    this.dispatchEvent(new Event('change'));
  }
}

test('the root follows the system’s light or dark style until stopped', () => {
  const root = document.createElement('html');
  const query = new FakeDarkQuery(false);

  const stop = followSystemColourScheme(root, query as unknown as MediaQueryList);
  expect(root.dataset.colorScheme).toBe('light');

  query.flip(true);
  expect(root.dataset.colorScheme).toBe('dark');

  stop();
  query.flip(false);
  expect(root.dataset.colorScheme).toBe('dark');
});
