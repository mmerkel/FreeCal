import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, expect, test } from 'vitest';
import { tauriCore } from './tauriCore';

afterEach(() => clearMocks());

test('today turns the shell’s ISO date into a PlainDate', async () => {
  mockIPC((command) => (command === 'today' ? '2026-10-04' : undefined));

  const today = await tauriCore.today();

  expect(today).toBeInstanceOf(Temporal.PlainDate);
  expect(today.equals(Temporal.PlainDate.from('2026-10-04'))).toBe(true);
});

test('listAccounts returns the shell’s Accounts', async () => {
  mockIPC((command) => (command === 'list_accounts' ? [{ id: 1, provider: 'local' }] : undefined));

  expect(await tauriCore.listAccounts()).toEqual([{ id: 1, provider: 'local' }]);
});

test('subscribe receives the Signals the shell forwards until unsubscribed', async () => {
  mockIPC(() => undefined, { shouldMockEvents: true });
  const received: unknown[] = [];

  const unsubscribe = await tauriCore.subscribe((signal) => received.push(signal));
  await emit('freecal://signal', { kind: 'first' });
  unsubscribe();
  await emit('freecal://signal', { kind: 'second' });
  await emit('some-other-event', { kind: 'other' });

  expect(received).toEqual([{ kind: 'first' }]);
});
