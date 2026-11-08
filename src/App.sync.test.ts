// The status indicator and the sidebar's Sync Status markers, run against
// the recording fake core with a stand-in grid: they follow "Sync status
// changed", which the tests emit with the Sync Statuses they script.
import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import type {
  AccountId,
  CalendarId,
  SyncStatus,
  SyncStatusReport,
  SyncStatuses,
} from './core/types';
import { SYNCING_DELAY_MS } from './lib/syncDisplay.svelte';
import { caldavAccount, calendar, firstLaunch, localAccount } from './test/answers';
import type { RecordingCore } from './test/recordingCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

// Three minutes after the last sync, at noon in Berlin.
beforeEach(() => vi.setSystemTime('2026-10-03T10:03:00Z'));
afterEach(() => vi.useRealTimers());

/** A sync finished at noon in Berlin. */
const noon = { at: '2026-10-03T10:00:00Z', outcome: { kind: 'synced' } } as const;

function report(status: SyncStatus, unsentChanges = 0): SyncStatusReport {
  return { status, lastSync: noon, unsentChanges };
}

/**
 * A CalDAV Account with the Calendar "Team", both synced at noon, and the
 * Signals the core sends as their Sync Status changes.
 */
function withServer() {
  const core = firstLaunch();
  const account = caldavAccount('ada');
  const team = calendar({ accountId: account.id, name: 'Team', colour: '#33aa66', readOnly: true });
  let statuses: SyncStatuses = {
    overall: { state: { kind: 'synced' }, lastSync: noon.at },
    accounts: [{ accountId: account.id, report: report({ kind: 'synced' }) }],
    calendars: [
      { calendarId: team.id, report: report({ kind: 'synced' }), firstSyncUnfinished: false },
    ],
  };
  core.answer('listAccounts', [localAccount, account]);
  core.answer('listCalendars', [team]);
  core.answer('syncStatuses', statuses);
  /** Scripts the overall status of the next "Sync status changed". */
  const setOverall = (overall: SyncStatuses['overall']) => {
    statuses = { ...statuses, overall };
    core.answer('syncStatuses', statuses);
  };
  /** Scripts a new Sync Status Report of the Account and emits "Sync status changed". */
  const reportAccount = (id: AccountId, accountReport: SyncStatusReport) => {
    statuses = {
      ...statuses,
      accounts: [
        ...statuses.accounts.filter((found) => found.accountId !== id),
        { accountId: id, report: accountReport },
      ],
    };
    core.answer('syncStatuses', statuses);
    core.emit({ kind: 'accountSyncStatusChanged', accountId: id, report: accountReport });
  };
  /**
   * Scripts a new Sync Status Report of the Calendar, with whether its First
   * Sync is unfinished, and emits "Sync status changed".
   */
  const reportCalendar = (
    id: CalendarId,
    calendarReport: SyncStatusReport,
    firstSyncUnfinished = false,
  ) => {
    statuses = {
      ...statuses,
      calendars: [
        ...statuses.calendars.filter((found) => found.calendarId !== id),
        { calendarId: id, report: calendarReport, firstSyncUnfinished },
      ],
    };
    core.answer('syncStatuses', statuses);
    core.emit({
      kind: 'calendarSyncStatusChanged',
      calendarId: id,
      report: calendarReport,
      firstSyncUnfinished,
    });
  };
  return {
    core,
    account: account.id,
    team: team.id,
    setOverall,
    reportAccount,
    reportCalendar,
  };
}

async function showApp(core: RecordingCore) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
}

function indicator() {
  return screen.getByRole('group', { name: 'Sync status' });
}

/**
 * What the status indicator says: as the toolbar icon's name and tooltip,
 * and as the line at the bottom of the sidebar, which must agree.
 */
function statusText(): string {
  const icon = within(indicator()).getByRole('img');
  const text = icon.getAttribute('aria-label') ?? '';
  expect(indicator()).toHaveAttribute('title', text);
  expect(sidebarStatusText()).toBe(text);
  return text;
}

/** The status text at the bottom of the sidebar, if it shows. */
function sidebarStatusText(): string | undefined {
  return document.querySelector('.overall-text')?.textContent?.trim();
}

/** The sidebar's section of the Account "ada (CalDAV)". */
function accountSection() {
  return within(
    screen.getByRole('heading', { name: '\u2068ada\u2069 (CalDAV)' }).closest('section')!,
  );
}

function team() {
  return screen.getByRole('switch', { name: 'Team' });
}

test('with no server Accounts the indicator says synced, with no time', async () => {
  await showApp(firstLaunch());

  expect(statusText()).toBe('Synced');
});

test('the indicator shows the overall status and how long ago the last sync finished', async () => {
  const { core } = withServer();
  await showApp(core);

  expect(statusText()).toBe('Synced 3 minutes ago');
});

test('under a minute the indicator shows no time, then the time ago in minutes', async () => {
  vi.useFakeTimers();
  vi.setSystemTime('2026-10-03T10:00:05Z');
  const { core } = withServer();
  render(App, { core });
  await vi.waitFor(() => screen.getByRole('list', { name: 'Grid' }));

  expect(statusText()).toBe('Synced');

  await vi.advanceTimersByTimeAsync(54_000);
  expect(statusText()).toBe('Synced');

  await vi.advanceTimersByTimeAsync(1000);
  expect(statusText()).toBe('Synced 1 minute ago');

  await vi.advanceTimersByTimeAsync(29_000);
  expect(statusText()).toBe('Synced 1 minute ago');

  await vi.advanceTimersByTimeAsync(1000);
  expect(statusText()).toBe('Synced 2 minutes ago');
});

test('while the sidebar is hidden, only the toolbar icon’s tooltip says the status', async () => {
  const { core } = withServer();
  core.answer('sidebar', { shown: false, width: null });
  await showApp(core);

  expect(indicator()).toHaveAttribute('title', 'Synced 3 minutes ago');
  expect(within(indicator()).getByRole('img')).toHaveAccessibleName('Synced 3 minutes ago');
  // Its line in the sidebar is hidden with the sidebar.
  const line = document.querySelector('.overall-text')!;
  expect(line.parentElement!.closest('[aria-hidden="true"]')).not.toBeNull();
});

test('the indicator and the Account row follow "Sync status changed" to Offline and back', async () => {
  const { core, account, setOverall, reportAccount } = withServer();
  await showApp(core);

  setOverall({ state: { kind: 'offline' }, lastSync: '2026-10-03T10:01:00Z' });
  reportAccount(account, {
    status: { kind: 'offline' },
    lastSync: { at: '2026-10-03T10:01:00Z', outcome: { kind: 'offline' } },
    unsentChanges: 0,
  });

  await vi.waitFor(() => expect(statusText()).toBe('Offline · last sync 2 minutes ago'));
  expect(accountSection().getByText('Offline')).toBeInTheDocument();
  // Shown once on the Account, not on its Calendars.
  expect(team()).not.toHaveAccessibleDescription(/Offline/);

  setOverall({ state: { kind: 'synced' }, lastSync: '2026-10-03T10:02:00Z' });
  reportAccount(account, report({ kind: 'synced' }));

  await vi.waitFor(() => expect(statusText()).toBe('Synced 1 minute ago'));
  expect(accountSection().queryByText('Offline')).toBeNull();
});

test('a Sync Error shows on the Calendar where it happened, with its message', async () => {
  const { core, team: id, setOverall, reportCalendar } = withServer();
  await showApp(core);

  setOverall({ state: { kind: 'syncError' }, lastSync: noon.at });
  reportCalendar(
    id,
    report({ kind: 'syncError', error: { code: 'serverRefused', status: 403, message: 'No' } }),
  );

  await vi.waitFor(() => expect(statusText()).toBe('Sync error · last sync 3 minutes ago'));
  expect(team()).toHaveAccessibleDescription(/Sync error: .*403/);
});

test('the sidebar shows a Calendar’s unsent changes when there are any', async () => {
  const { core, team: id, reportCalendar } = withServer();
  await showApp(core);

  reportCalendar(id, report({ kind: 'synced' }, 3));

  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/3 changes not yet sent/));

  reportCalendar(id, report({ kind: 'synced' }, 0));

  await vi.waitFor(() => expect(team()).not.toHaveAccessibleDescription(/not yet sent/));
});

test('a sync FreeCal started shows as Syncing only after about a second', async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const { core, account, setOverall, reportAccount } = withServer();
  await showApp(core);

  setOverall({ state: { kind: 'syncing', byUser: false }, lastSync: noon.at });
  reportAccount(account, report({ kind: 'syncing', byUser: false }));
  await vi.waitFor(() => expect(core.callsOf('syncStatuses')).toHaveLength(2));
  await vi.advanceTimersByTimeAsync(SYNCING_DELAY_MS / 2);

  expect(statusText()).toBe('Synced 3 minutes ago');
  expect(accountSection().queryByText('Syncing…')).toBeNull();

  await vi.advanceTimersByTimeAsync(SYNCING_DELAY_MS / 2);

  expect(statusText()).toBe('Syncing… · last sync 3 minutes ago');
  expect(accountSection().getByText('Syncing…')).toBeInTheDocument();
});

test('a sync the user started shows as Syncing at once', async () => {
  const { core, account, setOverall, reportAccount } = withServer();
  await showApp(core);

  setOverall({ state: { kind: 'syncing', byUser: true }, lastSync: noon.at });
  reportAccount(account, report({ kind: 'syncing', byUser: true }));

  await vi.waitFor(() => expect(statusText()).toBe('Syncing… · last sync 3 minutes ago'));
  expect(accountSection().getByText('Syncing…')).toBeInTheDocument();
});

test('a Calendar whose First Sync is unfinished says so, whatever its Sync Status', async () => {
  const { core, team: id, reportCalendar } = withServer();
  await showApp(core);
  const unfinished = (status: SyncStatus, lastSync: SyncStatusReport['lastSync']) =>
    reportCalendar(id, { status, lastSync, unsentChanges: 0 }, true);
  const refused = { code: 'serverRefused', status: 404, message: 'Not Found' } as const;

  unfinished({ kind: 'syncing', byUser: true }, null);
  await vi.waitFor(() =>
    expect(team()).toHaveAccessibleDescription(/^Loading for the first time… /),
  );

  // Waiting for the next sync, as after a relaunch or a sync that didn't reach it.
  unfinished({ kind: 'synced' }, null);
  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/^Not loaded yet /));

  unfinished(
    { kind: 'syncError', error: refused },
    { at: noon.at, outcome: { kind: 'syncError', error: refused } },
  );
  await vi.waitFor(() =>
    expect(team()).toHaveAccessibleDescription(/^Sync error: .*404.* · not fully loaded /),
  );

  // Once it finishes, it says nothing.
  reportCalendar(id, report({ kind: 'synced' }));
  await vi.waitFor(() => expect(team()).not.toHaveAccessibleDescription(/loaded/));
});

test('the overall status doesn’t show a Calendar whose First Sync is unfinished', async () => {
  const { core, team: id, reportCalendar } = withServer();
  await showApp(core);

  reportCalendar(id, report({ kind: 'synced' }), true);

  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/^Not loaded yet /));
  expect(statusText()).toBe('Synced 3 minutes ago');
});

test('a Calendar whose First Sync is unfinished says “Not loaded yet” while its Account is Offline', async () => {
  const { core, account, team: id, reportAccount, reportCalendar } = withServer();
  await showApp(core);
  const offline = { at: '2026-10-03T10:01:00Z', outcome: { kind: 'offline' } } as const;

  reportAccount(account, { status: { kind: 'offline' }, lastSync: offline, unsentChanges: 0 });
  reportCalendar(id, { status: { kind: 'synced' }, lastSync: null, unsentChanges: 0 }, true);

  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/^Not loaded yet /));
  expect(accountSection().getByText('Offline')).toBeInTheDocument();
});

test('“Sync now” in the main menu syncs every server Account', async () => {
  const user = userEvent.setup();
  const { core } = withServer();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  const menu = screen.getByRole('menu', { name: 'Main menu' });
  await user.click(within(menu).getByRole('menuitem', { name: 'Sync now' }));

  expect(core.callsOf('syncNow')).toEqual([[null]]);
});

test('R syncs every server Account, but not while typing', async () => {
  const user = userEvent.setup();
  const { core } = withServer();
  await showApp(core);

  await user.keyboard('r');
  expect(core.callsOf('syncNow')).toEqual([[null]]);

  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.keyboard('r');
  expect(core.callsOf('syncNow')).toEqual([[null]]);
});

test('“Sync now” in an Account’s menu syncs that Account', async () => {
  const user = userEvent.setup();
  const { core, account } = withServer();
  await showApp(core);

  await user.click(
    accountSection().getByRole('button', { name: 'Options for \u2068ada\u2069 (CalDAV)' }),
  );
  const menu = screen.getByRole('menu', { name: 'Options for \u2068ada\u2069 (CalDAV)' });
  await user.click(within(menu).getByRole('menuitem', { name: 'Sync now' }));

  expect(core.callsOf('syncNow')).toEqual([[account]]);
});
