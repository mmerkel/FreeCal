// Quitting with Unsent Changes (spec: "Quitting"), against the fake core
// with a stand-in grid: Quit first tries to send them under "Working…",
// then quits, or asks with the Calendars still waiting and why.
import { screen, within } from '@testing-library/svelte';
import type userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import type { CalendarSyncStatus, SyncStatus, SyncStatuses } from './core/types';
import { caldavAccount, calendar, firstLaunch, localAccount } from './test/answers';
import { settled } from './test/fakeCore';
import { showApp } from './test/showApp';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

const account = caldavAccount('ada');
const team = calendar({ accountId: account.id, name: 'Team' });
const family = calendar({ accountId: account.id, name: 'Family' });

/**
 * The Sync Statuses with the Account in `accountStatus`, Team with `teamUnsent`
 * Unsent Changes and in `teamStatus`, and Family with `familyUnsent`.
 */
function statuses(
  teamUnsent: number,
  familyUnsent = 0,
  accountStatus: SyncStatus = { kind: 'synced' },
  teamStatus: SyncStatus = { kind: 'synced' },
): SyncStatuses {
  const calendarStatus = (
    calendarId: CalendarSyncStatus['calendarId'],
    unsentChanges: number,
    status: SyncStatus,
  ): CalendarSyncStatus => ({
    calendarId,
    report: { status, lastSync: null, unsentChanges },
    firstSyncUnfinished: false,
  });
  return {
    overall: { state: { kind: 'synced' }, lastSync: null },
    accounts: [
      {
        accountId: account.id,
        report: { status: accountStatus, lastSync: null, unsentChanges: teamUnsent + familyUnsent },
      },
    ],
    calendars: [
      calendarStatus(team.id, teamUnsent, teamStatus),
      calendarStatus(family.id, familyUnsent, { kind: 'synced' }),
    ],
  };
}

/** The window with the CalDAV Account's Calendars, whose Sync Statuses are `before`. */
async function showWith(before: SyncStatuses) {
  const core = firstLaunch();
  core.answer('listAccounts', [localAccount, account]);
  core.answer('listCalendars', [team, family]);
  core.answer('syncStatuses', before);
  return showApp(core);
}

async function chooseQuit(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.click(
    within(screen.getByRole('menu', { name: 'Main menu' })).getByRole('menuitem', {
      name: 'Quit',
    }),
  );
}

test('Quit with nothing unsent quits at once', async () => {
  const { core, user } = await showWith(statuses(0));

  await chooseQuit(user);

  expect(core.callsOf('sendBeforeQuit')).toEqual([]);
  expect(core.callsOf('quit')).toEqual([[]]);
});

test('Quit with unsent changes first sends them under "Working…", then quits', async () => {
  const { core, user } = await showWith(statuses(1, 1));
  const sending = core.pending('sendBeforeQuit');

  await chooseQuit(user);

  const working = screen.getByRole('dialog', { name: 'Working…' });
  expect(working).toHaveTextContent('Sending 2 changes…');
  expect(core.callsOf('quit')).toEqual([]);
  sending.resolve(statuses(0));
  await settled();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.callsOf('quit')).toEqual([[]]);
});

test('changes still unsent are listed with why, with Cancel focused', async () => {
  const { core, user } = await showWith(statuses(2, 1));
  core.answer(
    'sendBeforeQuit',
    statuses(2, 1, { kind: 'synced' }, { kind: 'syncError', error: { code: 'writesRefused' } }),
  );

  await chooseQuit(user);

  const asking = await screen.findByRole('alertdialog', {
    name: 'Quit with changes not yet sent?',
  });
  expect(asking).toHaveTextContent('3 changes haven’t reached the server yet:');
  const listed = within(asking)
    .getAllByRole('listitem')
    .map((item) => item.textContent);
  expect(listed).toEqual([
    '“⁨Team⁩”: the server doesn’t allow changes to this calendar',
    '“⁨Family⁩”: not sent yet',
  ]);
  expect(asking).toHaveTextContent('They’ll be sent the next time FreeCal starts.');
  expect(within(asking).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  expect(core.callsOf('quit')).toEqual([]);
});

test('an Offline Account’s changes say so', async () => {
  const offline = statuses(1, 0, { kind: 'offline' });
  const { core, user } = await showWith(offline);
  core.answer('sendBeforeQuit', offline);

  await chooseQuit(user);

  const asking = await screen.findByRole('alertdialog');
  expect(asking).toHaveTextContent('1 change hasn’t reached the server yet:');
  expect(within(asking).getByRole('listitem')).toHaveTextContent('“⁨Team⁩”: offline');
});

test('a hostile Calendar name in the quit dialog stays text', async () => {
  const hostileCalendar = { ...team, name: hostile.calendarName };
  const offline = statuses(1, 0, { kind: 'offline' });
  const core = firstLaunch();
  core.answer('listAccounts', [localAccount, account]);
  core.answer('listCalendars', [hostileCalendar, family]);
  core.answer('syncStatuses', offline);
  core.answer('sendBeforeQuit', offline);
  const { user } = await showApp(core);

  await chooseQuit(user);

  const line = within(await screen.findByRole('alertdialog')).getByRole('listitem');
  expect(line.textContent).toBe(hostile.expected.quitCalendar);
  expect(line.querySelector('img')).toBeNull();
});

test('Cancel keeps FreeCal running, and Quit anyway quits', async () => {
  const { core, user } = await showWith(statuses(1));
  core.answer('sendBeforeQuit', statuses(1));
  await chooseQuit(user);

  await user.click(await screen.findByRole('button', { name: 'Cancel' }));

  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(core.callsOf('quit')).toEqual([]);

  await chooseQuit(user);
  await user.click(await screen.findByRole('button', { name: 'Quit anyway' }));

  expect(core.callsOf('sendBeforeQuit')).toHaveLength(2);
  expect(core.callsOf('quit')).toEqual([[]]);
});

test('closing the window with unsent changes quits as Quit does', async () => {
  const { core } = await showWith(statuses(1));
  core.answer('sendBeforeQuit', statuses(1));

  core.askToQuit();

  expect(await screen.findByRole('alertdialog')).toHaveTextContent(
    '1 change hasn’t reached the server yet:',
  );
  expect(core.callsOf('sendBeforeQuit')).toEqual([[]]);
});
