import { expect } from 'vitest';

/** Checks that a menu is placed against `button` by CSS anchor positioning. */
export function expectAnchoredTo(menu: HTMLElement, button: HTMLElement) {
  const anchor = menu.style.getPropertyValue('position-anchor');
  expect(anchor).toMatch(/^--/);
  expect(button.style.getPropertyValue('anchor-name')).toBe(anchor);
}
