import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { SyncStatuses } from '../core/types';
import { accountId, calendarId } from '../test/ids';
import { overallSyncText, SYNCING_DELAY_MS, SyncingDelay, timeAgo } from './syncDisplay.svelte';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

const lastSync = { at: '2026-10-03T10:00:00Z', outcome: { kind: 'synced' } } as const;

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

/** How long ago an instant `seconds` before noon UTC was, seen at noon. */
function ago(seconds: number): string {
  const now = Temporal.Instant.from('2026-10-03T12:00:00Z');
  return timeAgo(now.subtract({ milliseconds: seconds * 1000 }), now);
}

test('the time ago is in the largest whole unit, rounded half up', () => {
  expect(ago(0)).toBe('0 seconds ago');
  expect(ago(1)).toBe('1 second ago');
  expect(ago(42)).toBe('42 seconds ago');
  expect(ago(60)).toBe('1 minute ago');
  expect(ago(89)).toBe('1 minute ago');
  expect(ago(90)).toBe('2 minutes ago');
  expect(ago(3 * 60)).toBe('3 minutes ago');
  expect(ago(60 * 60 + 25 * 60)).toBe('1 hour ago');
  expect(ago(60 * 60 + 31 * 60)).toBe('2 hours ago');
  expect(ago(36 * 60 * 60)).toBe('2 days ago');
  expect(ago(400 * 24 * 60 * 60)).toBe('400 days ago');
});

test('a time that rounds up to the next unit shows in that unit', () => {
  expect(ago(59.6)).toBe('1 minute ago');
  expect(ago(59 * 60 + 40)).toBe('1 hour ago');
  expect(ago(23 * 60 * 60 + 40 * 60)).toBe('1 day ago');
});

test('a last sync after now, from a clock that went back, is 0 seconds ago', () => {
  expect(ago(-30)).toBe('0 seconds ago');
});

test('the time ago counts instants, so a DST change neither adds nor loses an hour', () => {
  // 02:30 CEST, then 02:30 CET an hour later, as Berlin's clocks went back.
  const before = Temporal.Instant.from('2026-10-25T02:30:00+02:00');
  const after = Temporal.Instant.from('2026-10-25T02:30:00+01:00');

  expect(timeAgo(before, after)).toBe('1 hour ago');
});

test('the status text says Synced with the time ago, other states the last sync', () => {
  const now = Temporal.Instant.from('2026-10-03T10:03:00Z');
  const lastSync = '2026-10-03T10:00:00Z';

  expect(overallSyncText({ state: { kind: 'synced' }, lastSync }, now)).toBe(
    'Synced 3 minutes ago',
  );
  expect(overallSyncText({ state: { kind: 'offline' }, lastSync }, now)).toBe(
    'Offline · last sync 3 minutes ago',
  );
  expect(overallSyncText({ state: { kind: 'syncing', byUser: true }, lastSync }, now)).toBe(
    'Syncing… · last sync 3 minutes ago',
  );
  expect(overallSyncText({ state: { kind: 'syncError' }, lastSync }, now)).toBe(
    'Sync error · last sync 3 minutes ago',
  );
  expect(overallSyncText({ state: { kind: 'conflict' }, lastSync }, now)).toBe(
    'Conflict · last sync 3 minutes ago',
  );
});

test('without a last sync the status text is the state alone', () => {
  const now = Temporal.Instant.from('2026-10-03T10:03:00Z');

  expect(overallSyncText({ state: { kind: 'synced' }, lastSync: null }, now)).toBe('Synced');
  expect(overallSyncText({ state: { kind: 'offline' }, lastSync: null }, now)).toBe('Offline');
});
