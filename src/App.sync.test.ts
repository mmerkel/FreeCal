// The status indicator and the sidebar's Sync Status markers, run against
// the fake core with a stand-in grid: they follow "Sync status changed".
import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import type { SyncStatus, SyncStatusReport } from './core/types';
import { SYNCING_DELAY_MS } from './lib/syncDisplay.svelte';
import { FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

// Three minutes after the last sync, at noon in Berlin.
beforeEach(() => vi.setSystemTime('2026-10-03T10:03:00Z'));
afterEach(() => vi.useRealTimers());

/** A sync finished at noon in Berlin. */
const noon = { at: '2026-10-03T10:00:00Z', outcome: { kind: 'synced' } } as const;

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
  await showApp(new FakeCore());

  expect(statusText()).toBe('Synced');
});

test('the indicator shows the overall status and how long ago the last sync finished', async () => {
  const { core } = withServer();
  await showApp(core);

  expect(statusText()).toBe('Synced 3 minutes ago');
});

test('under a minute the time ago counts up every second, then goes on in minutes', async () => {
  vi.useFakeTimers();
  vi.setSystemTime('2026-10-03T10:00:05Z');
  const { core } = withServer();
  render(App, { core });
  await vi.waitFor(() => screen.getByRole('list', { name: 'Grid' }));

  expect(statusText()).toBe('Synced 5 seconds ago');

  await vi.advanceTimersByTimeAsync(1000);
  expect(statusText()).toBe('Synced 6 seconds ago');

  await vi.advanceTimersByTimeAsync(83_000);
  expect(statusText()).toBe('Synced 1 minute ago');

  await vi.advanceTimersByTimeAsync(1000);
  expect(statusText()).toBe('Synced 2 minutes ago');
});

test('while the sidebar is hidden, only the toolbar icon’s tooltip says the status', async () => {
  const { core } = withServer();
  core.storedSidebar = { shown: false, width: null };
  await showApp(core);

  expect(indicator()).toHaveAttribute('title', 'Synced 3 minutes ago');
  expect(within(indicator()).getByRole('img')).toHaveAccessibleName('Synced 3 minutes ago');
  // Its line in the sidebar is hidden with the sidebar.
  const line = document.querySelector('.overall-text')!;
  expect(line.parentElement!.closest('[aria-hidden="true"]')).not.toBeNull();
});

test('the indicator and the Account row follow "Sync status changed" to Offline and back', async () => {
  const { core, account } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'offline' }, lastSync: '2026-10-03T10:01:00Z' });
  core.reportAccount(account, {
    status: { kind: 'offline' },
    lastSync: { at: '2026-10-03T10:01:00Z', outcome: { kind: 'offline' } },
    unsentChanges: 0,
  });

  await vi.waitFor(() => expect(statusText()).toBe('Offline · last sync 2 minutes ago'));
  expect(accountSection().getByText('Offline')).toBeInTheDocument();
  // Shown once on the Account, not on its Calendars.
  expect(team()).not.toHaveAccessibleDescription(/Offline/);

  core.setOverall({ state: { kind: 'synced' }, lastSync: '2026-10-03T10:02:00Z' });
  core.reportAccount(account, report({ kind: 'synced' }));

  await vi.waitFor(() => expect(statusText()).toBe('Synced 1 minute ago'));
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

  await vi.waitFor(() => expect(statusText()).toBe('Sync error · last sync 3 minutes ago'));
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

  expect(statusText()).toBe('Synced 3 minutes ago');
  expect(accountSection().queryByText('Syncing…')).toBeNull();

  await vi.advanceTimersByTimeAsync(SYNCING_DELAY_MS / 2);

  expect(statusText()).toBe('Syncing… · last sync 3 minutes ago');
  expect(accountSection().getByText('Syncing…')).toBeInTheDocument();
});

test('a sync the user started shows as Syncing at once', async () => {
  const { core, account } = withServer();
  await showApp(core);

  core.setOverall({ state: { kind: 'syncing', byUser: true }, lastSync: noon.at });
  core.reportAccount(account, report({ kind: 'syncing', byUser: true, firstSync: false }));

  await vi.waitFor(() => expect(statusText()).toBe('Syncing… · last sync 3 minutes ago'));
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
