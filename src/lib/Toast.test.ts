// The toast: a confirmation goes after 5 seconds, not while the pointer is
// over it or it has the focus; one reporting a problem stays.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import Toast from './Toast.svelte';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

function show(stays = false) {
  const ondismiss = vi.fn();
  const run = vi.fn();
  render(Toast, { message: 'Event deleted', action: { label: 'Undo', run }, stays, ondismiss });
  return ondismiss;
}

test('a confirmation goes after 5 seconds', () => {
  const ondismiss = show();

  vi.advanceTimersByTime(4999);
  expect(ondismiss).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(ondismiss).toHaveBeenCalledOnce();
});

test('a toast reporting a problem stays', () => {
  const ondismiss = show(true);

  vi.advanceTimersByTime(60_000);

  expect(ondismiss).not.toHaveBeenCalled();
});

test('it stays while the pointer is over it, and goes 5 seconds after it left', async () => {
  const ondismiss = show();
  const toast = screen.getByRole('status');

  vi.advanceTimersByTime(4000);
  await fireEvent.mouseEnter(toast);
  vi.advanceTimersByTime(10_000);
  expect(ondismiss).not.toHaveBeenCalled();

  await fireEvent.mouseLeave(toast);
  vi.advanceTimersByTime(4999);
  expect(ondismiss).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(ondismiss).toHaveBeenCalledOnce();
});

test('it stays while it has the focus', async () => {
  const ondismiss = show();
  const undo = screen.getByRole('button', { name: 'Undo' });

  undo.focus();
  await tick();
  vi.advanceTimersByTime(10_000);
  expect(ondismiss).not.toHaveBeenCalled();

  undo.blur();
  await tick();
  vi.advanceTimersByTime(5000);
  expect(ondismiss).toHaveBeenCalledOnce();
});

test('focus moving within it keeps it', async () => {
  const ondismiss = show();
  const undo = screen.getByRole('button', { name: 'Undo' });

  undo.focus();
  await tick();
  screen.getByRole('button', { name: 'Dismiss' }).focus();
  await tick();
  vi.advanceTimersByTime(10_000);

  expect(ondismiss).not.toHaveBeenCalled();
});
