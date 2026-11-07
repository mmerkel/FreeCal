// The status indicator and the sidebar's Sync Status markers, run against
// the fake core with a stand-in grid: they follow "Sync status changed".
import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import type { SyncStatus, SyncStatusReport } from './core/types';
import { SYNCING_DELAY_MS } from './lib/syncDisplay.svelte';
import { FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

afterEach(() => vi.useRealTimers());

const noon = { at: '2026-10-03T12:00:00', outcome: { kind: 'synced' } } as const;

function report(status: SyncStatus, unsentChanges = 0): SyncStatusReport {
  return { status, lastSync: noon, unsentChanges };
}

/** A CalDAV Account with the Calendar "Team", both synced at noon. */
function withServer() {
  const core = new FakeCore();
  const account = core.addCalDavAccountNow('ada', [
    { url: 'https://dav.example.org/team/', name: 'Team', colour: '#33aa66', readOnly: false },
  ]);
  const team = core.addServerCalendar(account.id, 'https://dav.example.org/team/');
  core.statuses = {
    overall: { state: { kind: 'synced' }, lastSync: noon.at },
    accounts: [{ accountId: account.id, report: report({ kind: 'synced' }) }],
    calendars: [{ calendarId: team.id, report: report({ kind: 'synced' }) }],
  };
  return { core, account: account.id, team: team.id };
}

async function showApp(core: FakeCore) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
}

function indicator() {
  return screen.getByRole('group', { name: 'Sync status' });
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
  await showApp(new FakeCore());

  expect(indicator()).toHaveTextContent('Synced');
  expect(indicator()).not.toHaveTextContent('12:00');
});

test('the indicator shows the overall status and when the last sync finished', async () => {
  const { core } = withServer();
  await showApp(core);

  expect(indicator()).toHaveTextContent('Synced 12:00');
});

test('the indicator and the Account row follow "Sync status changed" to Offline and back', async () => {
  const { core, account } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'offline' }, lastSync: '2026-10-03T12:05:00' });
  core.reportAccount(account, {
    status: { kind: 'offline' },
    lastSync: { at: '2026-10-03T12:05:00', outcome: { kind: 'offline' } },
    unsentChanges: 0,
  });

  await vi.waitFor(() => expect(indicator()).toHaveTextContent('Offline · last sync 12:05'));
  expect(accountSection().getByText('Offline')).toBeInTheDocument();
  // Shown once on the Account, not on its Calendars.
  expect(team()).not.toHaveAccessibleDescription(/Offline/);

  core.setOverall({ state: { kind: 'synced' }, lastSync: '2026-10-03T12:07:00' });
  core.reportAccount(account, report({ kind: 'synced' }));

  await vi.waitFor(() => expect(indicator()).toHaveTextContent('Synced 12:07'));
  expect(accountSection().queryByText('Offline')).toBeNull();
});

test('a Sync Error shows on the Calendar where it happened, with its message', async () => {
  const { core, team: id } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'syncError' }, lastSync: noon.at });
  core.reportCalendar(
    id,
    report({ kind: 'syncError', error: { code: 'serverRefused', status: 403, message: 'No' } }),
  );

  await vi.waitFor(() => expect(indicator()).toHaveTextContent('Sync error'));
  expect(team()).toHaveAccessibleDescription(/Sync error: .*403/);
});

test('the sidebar shows a Calendar’s unsent changes when there are any', async () => {
  const { core, team: id } = withServer();
  await showApp(core);

  core.reportCalendar(id, report({ kind: 'synced' }, 3));

  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/3 changes not yet sent/));

  core.reportCalendar(id, report({ kind: 'synced' }, 0));

  await vi.waitFor(() => expect(team()).not.toHaveAccessibleDescription(/not yet sent/));
});

test('a sync FreeCal started shows as Syncing only after about a second', async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const { core, account } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'syncing', byUser: false }, lastSync: noon.at });
  core.reportAccount(account, report({ kind: 'syncing', byUser: false, firstSync: false }));
  await vi.waitFor(() =>
    expect(core.calls.filter((call) => call === 'syncStatuses')).toHaveLength(2),
  );
  await vi.advanceTimersByTimeAsync(SYNCING_DELAY_MS / 2);

  expect(indicator()).toHaveTextContent('Synced 12:00');
  expect(accountSection().queryByText('Syncing…')).toBeNull();

  await vi.advanceTimersByTimeAsync(SYNCING_DELAY_MS / 2);

  expect(indicator()).toHaveTextContent('Syncing…');
  expect(accountSection().getByText('Syncing…')).toBeInTheDocument();
});

test('a sync the user started shows as Syncing at once', async () => {
  const { core, account } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'syncing', byUser: true }, lastSync: noon.at });
  core.reportAccount(account, report({ kind: 'syncing', byUser: true, firstSync: false }));

  await vi.waitFor(() => expect(indicator()).toHaveTextContent('Syncing…'));
  expect(accountSection().getByText('Syncing…')).toBeInTheDocument();
});

test('a Calendar on its first sync says it is loading for the first time', async () => {
  const { core, team: id } = withServer();
  await showApp(core);

  core.reportCalendar(id, {
    status: { kind: 'syncing', byUser: true, firstSync: true },
    lastSync: null,
    unsentChanges: 0,
  });

  await vi.waitFor(() => expect(team()).toHaveAccessibleDescription(/Loading for the first time…/));
});

test('“Sync now” in the main menu syncs every server Account', async () => {
  const user = userEvent.setup();
  const { core } = withServer();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  const menu = screen.getByRole('menu', { name: 'Main menu' });
  await user.click(within(menu).getByRole('menuitem', { name: 'Sync now' }));

  expect(core.syncedNow).toEqual([null]);
});

test('R syncs every server Account, but not while typing', async () => {
  const user = userEvent.setup();
  const { core } = withServer();
  await showApp(core);

  await user.keyboard('r');
  expect(core.syncedNow).toEqual([null]);

  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.keyboard('r');
  expect(core.syncedNow).toEqual([null]);
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

  expect(core.syncedNow).toEqual([account]);
});
