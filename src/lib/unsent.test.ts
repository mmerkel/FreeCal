import { expect, test } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar, SyncStatus, SyncStatuses, UnsentChange } from '../core/types';
import { accountId, calendarId } from '../test/ids';
import { notYetSyncedText } from './unsent';

const work: Calendar = {
  id: calendarId(2),
  accountId: accountId(3),
  name: 'Work',
  colour: '#3366cc',
  shown: true,
  readOnly: false,
  serverReadOnly: false,
};

function statuses(account: SyncStatus, calendar: SyncStatus): SyncStatuses {
  const report = (status: SyncStatus) => ({ status, lastSync: null, unsentChanges: 1 });
  return {
    overall: { state: { kind: 'synced' }, lastSync: null },
    accounts: [{ accountId: work.accountId, report: report(account) }],
    calendars: [{ calendarId: work.id, report: report(calendar), firstSyncUnfinished: false }],
  };
}

const synced: SyncStatus = { kind: 'synced' };
const since = '2026-10-03T10:00:00Z';
const changedOnServer: UnsentChange = { since, sendFailure: { kind: 'serverChanged' } };
const neverTried: UnsentChange = { since, sendFailure: null };

test('an Unsent Change never tried, with nothing in its way, gives no reason', () => {
  expect(notYetSyncedText(neverTried, statuses(synced, synced), work)).toBe('Not yet synced');
});

test('a stored Send Failure is the reason when the Account and Calendar give none', () => {
  expect(notYetSyncedText(changedOnServer, statuses(synced, synced), work)).toBe(
    'Not yet synced: the event was changed on the server',
  );
  const failed: UnsentChange = {
    since,
    sendFailure: { kind: 'failed', error: { code: 'serverRefused', status: 500, message: 'Oops' } },
  };
  expect(notYetSyncedText(failed, statuses(synced, synced), work)).toContain('Oops');
});

test('the Account’s reason comes first, then the Calendar’s, then the Send Failure', () => {
  const refused: SyncStatus = { kind: 'syncError', error: { code: 'writesRefused' } };

  expect(notYetSyncedText(changedOnServer, statuses({ kind: 'offline' }, refused), work)).toBe(
    'Not yet synced: offline',
  );
  expect(notYetSyncedText(changedOnServer, statuses(synced, refused), work)).toBe(
    'Not yet synced: the server doesn’t allow changes to this calendar',
  );
});

test('a hostile server message in a Send Failure stays text', () => {
  const refused: UnsentChange = {
    since,
    sendFailure: {
      kind: 'failed',
      error: { code: 'serverRefused', status: 500, message: hostile.serverMessage },
    },
  };

  expect(notYetSyncedText(refused, statuses(synced, synced), work)).toBe(
    hostile.expected.notYetSynced,
  );
});

test('an unfinished First Sync holds the changes back', () => {
  const loading = statuses(synced, synced);
  loading.calendars[0].firstSyncUnfinished = true;

  expect(notYetSyncedText(changedOnServer, loading, work)).toBe(
    'Not yet synced: loading for the first time',
  );
});

test('without Sync Statuses yet, only the Send Failure gives a reason', () => {
  expect(notYetSyncedText(neverTried, undefined, work)).toBe('Not yet synced');
});
