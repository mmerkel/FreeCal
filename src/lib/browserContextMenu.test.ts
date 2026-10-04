import { expect, test } from 'vitest';
import { turnOffBrowserContextMenu } from './browserContextMenu';

function rightClick(element: Element): boolean {
  return element.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
}

test('the browser’s context menu is off everywhere, text fields included, until turned back on', () => {
  const page = document.createElement('div');
  const text = document.createElement('p');
  const field = document.createElement('input');
  page.append(text, field);
  document.body.append(page);

  const turnBackOn = turnOffBrowserContextMenu(page);
  expect(rightClick(text)).toBe(false);
  expect(rightClick(field)).toBe(false);

  turnBackOn();
  expect(rightClick(text)).toBe(true);
  page.remove();
});

test('FreeCal’s own right-click handlers still run', () => {
  const page = document.createElement('div');
  const row = document.createElement('button');
  page.append(row);
  let opened = false;
  row.addEventListener('contextmenu', () => (opened = true));

  const turnBackOn = turnOffBrowserContextMenu(page);
  rightClick(row);

  expect(opened).toBe(true);
  turnBackOn();
});

test('a handler that stops a right-click from spreading can’t bring the browser’s menu back', () => {
  const page = document.createElement('div');
  const row = document.createElement('button');
  page.append(row);
  row.addEventListener('contextmenu', (clicked) => clicked.stopPropagation());

  const turnBackOn = turnOffBrowserContextMenu(page);

  expect(rightClick(row)).toBe(false);
  turnBackOn();
});
