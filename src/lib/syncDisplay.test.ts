import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { SyncStatuses } from '../core/types';
import { accountId, calendarId } from '../test/ids';
import { SYNCING_DELAY_MS, SyncingDelay } from './syncDisplay';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

const lastSync = { at: '2026-10-03T12:00:00', outcome: { kind: 'synced' } } as const;

function statuses(byUser: boolean | undefined): SyncStatuses {
  const status =
    byUser === undefined
      ? ({ kind: 'synced' } as const)
      : ({ kind: 'syncing', byUser, firstSync: false } as const);
  return {
    overall: {
      state: byUser === undefined ? { kind: 'synced' } : { kind: 'syncing', byUser },
      lastSync: lastSync.at,
    },
    accounts: [{ accountId: accountId(2), report: { status, lastSync, unsentChanges: 0 } }],
    calendars: [
      {
        calendarId: calendarId(3),
        report: { status: { kind: 'offline' }, lastSync: null, unsentChanges: 0 },
      },
    ],
  };
}

test('a sync FreeCal started shows as Syncing only once it has run for about a second', () => {
  const delay = new SyncingDelay();

  let shown = delay.shown(statuses(false));
  expect(shown.overall.state).toEqual({ kind: 'synced' });
  expect(shown.accounts[0].report.status).toEqual({ kind: 'synced' });
  expect(shown.calendars[0].report.status).toEqual({ kind: 'offline' });

  vi.advanceTimersByTime(SYNCING_DELAY_MS - 1);
  shown = delay.shown(statuses(false));
  expect(shown.overall.state).toEqual({ kind: 'synced' });

  vi.advanceTimersByTime(1);
  shown = delay.shown(statuses(false));
  expect(shown.overall.state).toEqual({ kind: 'syncing', byUser: false });
  expect(shown.accounts[0].report.status.kind).toBe('syncing');
});

test('a sync the user started shows as Syncing at once', () => {
  const delay = new SyncingDelay();

  const shown = delay.shown(statuses(true));

  expect(shown.overall.state).toEqual({ kind: 'syncing', byUser: true });
  expect(shown.accounts[0].report.status.kind).toBe('syncing');
});

test('a sync that ends starts its delay again next time', () => {
  const delay = new SyncingDelay();
  delay.shown(statuses(false));
  vi.advanceTimersByTime(SYNCING_DELAY_MS);
  delay.shown(statuses(undefined));

  const shown = delay.shown(statuses(false));

  expect(shown.overall.state).toEqual({ kind: 'synced' });
});

test('while Syncing waits, a row shows how its last sync ended', () => {
  const delay = new SyncingDelay();
  const failed = statuses(false);
  failed.accounts[0].report.lastSync = { at: lastSync.at, outcome: { kind: 'offline' } };

  const shown = delay.shown(failed);

  expect(shown.accounts[0].report.status).toEqual({ kind: 'offline' });
});

test('it tells when a waiting Syncing is due, so that the window shows it then', () => {
  const delay = new SyncingDelay();
  const due = vi.fn();
  delay.ondue = due;
  delay.shown(statuses(false));

  vi.advanceTimersByTime(SYNCING_DELAY_MS);

  expect(due).toHaveBeenCalled();
});
