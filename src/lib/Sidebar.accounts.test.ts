// Server Accounts in the sidebar: adding one, choosing its Calendars and
// removing them, run against the fake core. What the server offers
// and what the core keeps are scripted; a test that changes them checks the
// call, then scripts the result and emits the Signals the core would send.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';
import App from '../App.svelte';
import hostile from '../../fixtures/hostile-event.json';
import type { Account, AvailableCalendar, Calendar, SyncStatusReport } from '../core/types';
import { caldavAccount, calendar, firstLaunch, localAccount } from '../test/answers';
import { failure, type FakeCore } from '../test/fakeCore';

const work: AvailableCalendar = {
  url: 'https://dav.example.org/alice/work/',
  name: 'Work',
  colour: '#00897b',
  readOnly: false,
};
const home: AvailableCalendar = {
  url: 'https://dav.example.org/alice/home/',
  name: 'Home',
  colour: null,
  readOnly: false,
};
const birthdays: AvailableCalendar = {
  url: 'https://dav.example.org/alice/birthdays/',
  name: 'Birthdays',
  colour: '#8e24aa',
  readOnly: false,
};

/** The sidebar's name for a CalDAV Account of `alice`. */
const ALICE = '⁨alice⁩ (CalDAV)';
const alice = caldavAccount('alice');

/** `available` added to FreeCal in `account`, read-only like every server Calendar until 1.0. */
function added(available: AvailableCalendar, account: Account = alice): Calendar {
  return calendar({
    accountId: account.id,
    name: available.name,
    colour: available.colour ?? '#808080',
    readOnly: true,
  });
}

/** A first launch with `accounts` besides the Local Account, and `calendars`. */
function withAccounts(accounts: Account[], calendars: Calendar[] = []) {
  const core = firstLaunch();
  core.answer('listAccounts', [localAccount, ...accounts]);
  core.answer('listCalendars', calendars);
  return core;
}

/** Scripts the Accounts and Calendars and emits "Calendars changed", as the core does after a change. */
function calendarsChanged(core: FakeCore, accounts: Account[], calendars: Calendar[]) {
  core.answer('listAccounts', [localAccount, ...accounts]);
  core.answer('listCalendars', calendars);
  core.emit({ kind: 'calendarsChanged' });
}

async function showApp(core: FakeCore) {
  render(App, { core });
  return screen.findByRole('list', { name: 'Local' });
}

/** Fills in and sends the "Add account…" dialog. */
async function addAccount(user: ReturnType<typeof userEvent.setup>, username = 'alice') {
  await user.click(screen.getByRole('button', { name: 'Add account…' }));
  const dialog = screen.getByRole('dialog', { name: 'Add a CalDAV account' });
  await user.type(
    within(dialog).getByRole('textbox', { name: 'Server address' }),
    'https://dav.example.org/',
  );
  await user.type(within(dialog).getByRole('textbox', { name: 'Username' }), username);
  await user.type(within(dialog).getByLabelText('Password'), 'secret');
  await user.click(within(dialog).getByRole('button', { name: 'Add account' }));
  return dialog;
}

/** Opens "Manage calendars…" from an Account's ⋮ menu. */
async function manage(user: ReturnType<typeof userEvent.setup>, account: string) {
  await user.click(screen.getByRole('button', { name: `Options for ${account}` }));
  const menu = screen.getByRole('menu', { name: `Options for ${account}` });
  await user.click(within(menu).getByRole('menuitem', { name: 'Manage calendars…' }));
  return screen.getByRole('dialog', { name: `Calendars of ${account}` });
}

test('adds a CalDAV Account, then the Calendars the user chooses, which load for the first time', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  core.answer('addCalDavAccount', { account: alice, available: [work, home] });
  await showApp(core);

  await addAccount(user);

  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(core.callsOf('addCalDavAccount')).toEqual([
    ['https://dav.example.org/', 'alice', 'secret'],
  ]);
  calendarsChanged(core, [alice], []);
  expect(within(chooser).getByRole('checkbox', { name: 'Work' })).toBeChecked();
  expect(within(chooser).getByRole('checkbox', { name: 'Home' })).toBeChecked();
  await user.click(within(chooser).getByRole('checkbox', { name: 'Home' }));
  const firstSync = core.pending('addCalendars');
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.callsOf('addCalendars')).toEqual([[alice.id, [work.url]]]);
  const workCalendar = added(work);
  const syncing = (status: SyncStatusReport['status'], firstSyncUnfinished: boolean) => {
    const report = { status, lastSync: null, unsentChanges: 0 };
    core.answer('syncStatuses', {
      overall: { state: { kind: 'synced' }, lastSync: null },
      accounts: [],
      calendars: [{ calendarId: workCalendar.id, report, firstSyncUnfinished }],
    });
    core.emit({
      kind: 'calendarSyncStatusChanged',
      calendarId: workCalendar.id,
      report,
      firstSyncUnfinished,
    });
  };
  calendarsChanged(core, [alice], [workCalendar]);
  syncing({ kind: 'syncing', byUser: true }, true);
  const list = await screen.findByRole('list', { name: ALICE });
  await waitFor(() =>
    expect(within(list).getByRole('switch', { name: 'Work' })).toHaveAccessibleDescription(
      'Loading for the first time… Read-only Work: shown, click to hide',
    ),
  );

  syncing({ kind: 'synced' }, false);
  firstSync.resolve([workCalendar]);

  await waitFor(() =>
    expect(screen.queryByText('Loading for the first time…')).not.toBeInTheDocument(),
  );
  expect(within(list).getByRole('switch', { name: 'Work' })).toHaveAccessibleDescription(
    'Read-only Work: shown, click to hide',
  );
});

test('a server that offers no Calendars says so after adding its Account', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  core.answer('addCalDavAccount', { account: alice, available: [] });
  await showApp(core);

  await addAccount(user);

  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(within(chooser).getByText('The server offers no calendars.')).toBeInTheDocument();
  calendarsChanged(core, [alice], []);
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));
  expect(core.callsOf('addCalendars')).toEqual([]);
  expect(
    within(await screen.findByRole('list', { name: ALICE })).getByText(
      'No calendars added yet. Choose some with “Manage calendars…”.',
    ),
  ).toBeInTheDocument();
});

test('a failure adding the Account is shown in words, and the dialog keeps what was typed', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  core.answer('addCalDavAccount', failure({ code: 'wrongCredentials' }));
  await showApp(core);

  const dialog = await addAccount(user);

  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The account couldn’t be added: the server didn’t accept the username and password',
  );
  expect(within(dialog).getByRole('textbox', { name: 'Username' })).toHaveValue('alice');
});

test('a server’s refusal is shown with its hostile text only as text', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  core.answer(
    'addCalDavAccount',
    failure({ code: 'serverRefused', status: 500, message: hostile.serverMessage }),
  );
  await showApp(core);

  const dialog = await addAccount(user);

  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    `The account couldn’t be added: ${hostile.expected.serverRefused}`,
    { normalizeWhitespace: false },
  );
  expect(document.querySelector('img')).toBeNull();
});

test('while the server is asked, the dialog says so and can’t add the Account twice or be cancelled', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  const answer = core.pending('addCalDavAccount');
  await showApp(core);

  const dialog = await addAccount(user);

  expect(within(dialog).getByText('Connecting to the server…')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Add account' })).toBeDisabled();
  expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeDisabled();
  await user.keyboard('{Escape}');
  expect(dialog).toBeInTheDocument();
  answer.resolve({ account: alice, available: [] });
  expect(await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` })).toBeInTheDocument();
  expect(core.callsOf('addCalDavAccount')).toHaveLength(1);
});

test('cancelling “Add account…” adds nothing', async () => {
  const user = userEvent.setup();
  const core = firstLaunch();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Add account…' }));
  await user.click(screen.getByRole('button', { name: 'Cancel' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.callsOf('addCalDavAccount')).toEqual([]);
});

test('“Manage calendars…” lists the Account’s Calendars checked and the server’s Available Calendars unchecked', async () => {
  const user = userEvent.setup();
  const workCalendar = added(work);
  const core = withAccounts([alice], [workCalendar]);
  core.answer('availableCalendars', [home, birthdays]);
  await showApp(core);

  const dialog = await manage(user, ALICE);

  expect(core.callsOf('availableCalendars')).toEqual([[alice.id]]);
  expect(await within(dialog).findByRole('checkbox', { name: 'Work' })).toBeChecked();
  expect(within(dialog).getByRole('checkbox', { name: 'Home' })).not.toBeChecked();
  expect(within(dialog).getByRole('checkbox', { name: 'Birthdays' })).not.toBeChecked();
  await user.click(within(dialog).getByRole('checkbox', { name: 'Birthdays' }));
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() => expect(core.callsOf('addCalendars')).toEqual([[alice.id, [birthdays.url]]]));
  expect(core.callsOf('removeCalendar')).toEqual([]);
  calendarsChanged(core, [alice], [workCalendar, added(birthdays)]);
  const list = screen.getByRole('list', { name: ALICE });
  expect(await within(list).findByRole('switch', { name: 'Birthdays' })).toBeInTheDocument();
});

test('“Manage calendars…” says while it asks the server, and without it still removes Calendars', async () => {
  const user = userEvent.setup();
  const workCalendar = added(work);
  const core = withAccounts([alice], [workCalendar]);
  const available = core.pending('availableCalendars');
  await showApp(core);

  const dialog = await manage(user, ALICE);

  expect(within(dialog).getByText('Asking the server for its calendars…')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Save' })).toBeDisabled();
  available.fail({ code: 'serverUnreachable' });
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The server’s calendars couldn’t be read: the server couldn’t be reached',
  );
  await user.click(within(dialog).getByRole('checkbox', { name: 'Work' }));
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() => expect(core.callsOf('removeCalendar')).toEqual([[workCalendar.id]]));
  calendarsChanged(core, [alice], []);
  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Work' })).not.toBeInTheDocument(),
  );
});

test('unchecking a Calendar in “Manage calendars…” says what happens, and Save removes it from FreeCal only', async () => {
  const user = userEvent.setup();
  const workCalendar = added(work);
  const homeCalendar = added(home);
  const core = withAccounts([alice], [workCalendar, homeCalendar]);
  core.answer('availableCalendars', []);
  await showApp(core);

  const dialog = await manage(user, ALICE);
  await user.click(await within(dialog).findByRole('checkbox', { name: 'Home' }));

  expect(within(dialog).getByText(/1 calendar will be removed from FreeCal/)).toHaveTextContent(
    '1 calendar will be removed from FreeCal, with its events on this computer. It stays on the server.',
  );
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.callsOf('removeCalendar')).toEqual([[homeCalendar.id]]);
  expect(core.callsOf('addCalendars')).toEqual([]);
});

test('Calendars that couldn’t be added or loaded are reported in the sidebar', async () => {
  const user = userEvent.setup();
  const core = withAccounts([alice]);
  core.answer('availableCalendars', [work]);
  core.answer('addCalendars', failure({ code: 'serverUnreachable' }));
  await showApp(core);

  const dialog = await manage(user, ALICE);
  await user.click(await within(dialog).findByRole('checkbox', { name: 'Work' }));
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Not every calendar could be added and loaded: the server couldn’t be reached',
  );
});

test('a server Calendar’s menu offers “Remove from FreeCal” instead of Delete, after confirming', async () => {
  const user = userEvent.setup();
  const workCalendar = added(work);
  const core = withAccounts([alice], [workCalendar]);
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Options for Work' }));
  const menu = screen.getByRole('menu', { name: 'Options for Work' });
  expect(within(menu).queryByRole('menuitem', { name: 'Delete' })).not.toBeInTheDocument();
  await user.click(within(menu).getByRole('menuitem', { name: 'Remove from FreeCal' }));
  const confirmation = screen.getByRole('alertdialog', {
    name: 'Remove “⁨Work⁩” from FreeCal?',
  });
  expect(confirmation).toHaveTextContent('It stays on the server');
  await user.click(within(confirmation).getByRole('button', { name: 'Remove' }));

  expect(core.callsOf('removeCalendar')).toEqual([[workCalendar.id]]);
  expect(core.callsOf('deleteCalendar')).toEqual([]);
  calendarsChanged(core, [alice], []);
  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Work' })).not.toBeInTheDocument(),
  );
});

test('a Local Calendar’s menu still offers Delete', async () => {
  const user = userEvent.setup();
  await showApp(withAccounts([], [calendar({ name: 'Notes' })]));

  await user.click(screen.getByRole('button', { name: 'Options for Notes' }));

  const menu = screen.getByRole('menu', { name: 'Options for Notes' });
  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toBeInTheDocument();
  expect(
    within(menu).queryByRole('menuitem', { name: 'Remove from FreeCal' }),
  ).not.toBeInTheDocument();
});

test('removes a whole Account only after confirming', async () => {
  const user = userEvent.setup();
  const core = withAccounts([alice], [added(work)]);
  await showApp(core);
  const remove = async () => {
    await user.click(screen.getByRole('button', { name: `Options for ${ALICE}` }));
    const menu = screen.getByRole('menu', { name: `Options for ${ALICE}` });
    await user.click(within(menu).getByRole('menuitem', { name: 'Remove account…' }));
    return screen.getByRole('alertdialog', { name: `Remove ${ALICE}?` });
  };

  let confirmation = await remove();
  expect(within(confirmation).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
  expect(core.callsOf('removeAccount')).toEqual([]);

  confirmation = await remove();
  expect(confirmation).toHaveTextContent('Nothing changes on the server.');
  await user.click(within(confirmation).getByRole('button', { name: 'Remove account' }));

  expect(core.callsOf('removeAccount')).toEqual([[alice.id]]);
  calendarsChanged(core, [], []);
  await waitFor(() => expect(screen.queryByRole('list', { name: ALICE })).not.toBeInTheDocument());
  expect(screen.getByRole('list', { name: 'Local' })).toBeInTheDocument();
});

test('the Local Account has no Account menu', async () => {
  await showApp(firstLaunch());

  expect(screen.queryByRole('button', { name: 'Options for Local' })).not.toBeInTheDocument();
});

test('hostile Calendar names from the server are text in the chooser, the sidebar and the remove dialog', async () => {
  const user = userEvent.setup();
  const hostileCalendar: AvailableCalendar = {
    url: 'https://dav.example.org/alice/x/',
    name: hostile.calendarName,
    colour: null,
    readOnly: false,
  };
  const core = firstLaunch();
  core.answer('addCalDavAccount', { account: alice, available: [hostileCalendar] });
  await showApp(core);

  await addAccount(user);
  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(within(chooser).getByRole('checkbox', { name: hostile.calendarName })).toBeChecked();
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));
  calendarsChanged(core, [alice], [added(hostileCalendar)]);
  const list = await screen.findByRole('list', { name: ALICE });
  await within(list).findByRole('switch', { name: hostile.calendarName });
  await user.click(screen.getByRole('button', { name: `Options for ${hostile.calendarName}` }));
  await user.click(screen.getByRole('menuitem', { name: 'Remove from FreeCal' }));

  expect(
    screen.getByRole('alertdialog', { name: hostile.expected.removeCalendarTitle }),
  ).toBeInTheDocument();
  expect(document.querySelector('img, script')).toBeNull();
});

test('a hostile username is text in the Account’s menu, “Manage calendars…” and the remove dialog', async () => {
  const user = userEvent.setup();
  await showApp(withAccounts([caldavAccount(hostile.username)]));
  const name = hostile.expected.accountName;

  const dialog = await manage(user, name);
  await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await user.click(screen.getByRole('button', { name: `Options for ${name}` }));
  await user.click(screen.getByRole('menuitem', { name: 'Remove account…' }));

  expect(screen.getByRole('alertdialog', { name: `Remove ${name}?` })).toBeInTheDocument();
  expect(document.querySelector('img')).toBeNull();
});
