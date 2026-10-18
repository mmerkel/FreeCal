import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';
import App from '../App.svelte';
import hostile from '../../fixtures/hostile-event.json';
import type { AddedAccount, AvailableCalendar } from '../core/types';
import { FakeCore } from '../test/fakeCore';

const work: AvailableCalendar = {
  url: 'https://dav.example.org/alice/work/',
  name: 'Work',
  colour: '#00897b',
};
const home: AvailableCalendar = {
  url: 'https://dav.example.org/alice/home/',
  name: 'Home',
  colour: null,
};
const birthdays: AvailableCalendar = {
  url: 'https://dav.example.org/alice/birthdays/',
  name: 'Birthdays',
  colour: '#8e24aa',
};

/** The sidebar's name for a CalDAV Account of `alice`. */
const ALICE = '⁨alice⁩ (CalDAV)';

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
  const core = new FakeCore();
  core.caldavServer = [work, home];
  await showApp(core);

  await addAccount(user);

  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(within(chooser).getByRole('checkbox', { name: 'Work' })).toBeChecked();
  expect(within(chooser).getByRole('checkbox', { name: 'Home' })).toBeChecked();
  expect(core.keyring.get(core.accounts[1].id)).toBe('secret');
  await user.click(within(chooser).getByRole('checkbox', { name: 'Home' }));
  const release = core.holdFirstSyncs();
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  const alice = await screen.findByRole('list', { name: ALICE });
  const row = await within(alice).findByRole('switch', { name: 'Work' });
  expect(row).toHaveAccessibleDescription('Loading for the first time…');
  expect(within(alice).queryByRole('switch', { name: 'Home' })).not.toBeInTheDocument();
  expect([...core.calendarUrls.values()]).toEqual([work.url]);

  release();

  await waitFor(() =>
    expect(screen.queryByText('Loading for the first time…')).not.toBeInTheDocument(),
  );
  expect(within(alice).getByRole('switch', { name: 'Work' })).toHaveAccessibleDescription(
    'Work: shown, click to hide',
  );
});

test('a server that offers no Calendars says so after adding its Account', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  await showApp(core);

  await addAccount(user);

  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(within(chooser).getByText('The server offers no calendars.')).toBeInTheDocument();
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));
  expect(core.calls).not.toContain('addCalendars');
  expect(
    within(screen.getByRole('list', { name: ALICE })).getByText(
      'No calendars added yet. Choose some with “Manage calendars…”.',
    ),
  ).toBeInTheDocument();
});

test('a failure adding the Account is shown in words, and the dialog keeps what was typed', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.caldavServer = { code: 'wrongCredentials' };
  await showApp(core);

  const dialog = await addAccount(user);

  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The account couldn’t be added: the server didn’t accept the username and password',
  );
  expect(within(dialog).getByRole('textbox', { name: 'Username' })).toHaveValue('alice');
  expect(core.accounts).toHaveLength(1);
});

test('a server’s refusal is shown with its hostile text only as text', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.caldavServer = { code: 'serverRefused', status: 500, message: hostile.serverMessage };
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
  const core = new FakeCore();
  let answer!: (added: AddedAccount) => void;
  core.addCalDavAccount = () => new Promise((resolve) => (answer = resolve));
  await showApp(core);

  const dialog = await addAccount(user);

  expect(within(dialog).getByText('Connecting to the server…')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Add account' })).toBeDisabled();
  expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeDisabled();
  await user.keyboard('{Escape}');
  expect(dialog).toBeInTheDocument();
  answer({ account: core.addCalDavAccountNow('alice'), available: [] });
  expect(await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` })).toBeInTheDocument();
});

test('cancelling “Add account…” adds nothing', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Add account…' }));
  await user.click(screen.getByRole('button', { name: 'Cancel' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('addCalDavAccount');
});

test('“Manage calendars…” lists the Account’s Calendars checked and the server’s Available Calendars unchecked', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const alice = core.addCalDavAccountNow('alice', [work, home, birthdays]);
  core.addServerCalendar(alice.id, work.url);
  await showApp(core);

  const dialog = await manage(user, ALICE);

  expect(core.calls).toContain('availableCalendars');
  expect(await within(dialog).findByRole('checkbox', { name: 'Work' })).toBeChecked();
  expect(within(dialog).getByRole('checkbox', { name: 'Home' })).not.toBeChecked();
  expect(within(dialog).getByRole('checkbox', { name: 'Birthdays' })).not.toBeChecked();
  await user.click(within(dialog).getByRole('checkbox', { name: 'Birthdays' }));
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  const list = screen.getByRole('list', { name: ALICE });
  expect(await within(list).findByRole('switch', { name: 'Birthdays' })).toBeInTheDocument();
  expect([...core.calendarUrls.values()]).toEqual([work.url, birthdays.url]);
  expect(core.calls).not.toContain('removeCalendar');
});

test('“Manage calendars…” says while it asks the server, and without it still removes Calendars', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const alice = core.addCalDavAccountNow('alice', [work]);
  core.addServerCalendar(alice.id, work.url);
  let fail!: (error: unknown) => void;
  core.availableCalendars = () => new Promise((_, reject) => (fail = reject));
  await showApp(core);

  const dialog = await manage(user, ALICE);

  expect(within(dialog).getByText('Asking the server for its calendars…')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Save' })).toBeDisabled();
  fail({ code: 'serverUnreachable' });
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The server’s calendars couldn’t be read: the server couldn’t be reached',
  );
  await user.click(within(dialog).getByRole('checkbox', { name: 'Work' }));
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Work' })).not.toBeInTheDocument(),
  );
  expect(core.calls).toContain('removeCalendar');
});

test('unchecking a Calendar in “Manage calendars…” says what happens, and Save removes it from FreeCal only', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const alice = core.addCalDavAccountNow('alice', [work, home]);
  core.addServerCalendar(alice.id, work.url);
  core.addServerCalendar(alice.id, home.url);
  await showApp(core);

  const dialog = await manage(user, ALICE);
  await user.click(await within(dialog).findByRole('checkbox', { name: 'Home' }));

  expect(within(dialog).getByText(/1 calendar will be removed from FreeCal/)).toHaveTextContent(
    '1 calendar will be removed from FreeCal, with its events on this computer. It stays on the server.',
  );
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Home' })).not.toBeInTheDocument(),
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.calls).toContain('removeCalendar');
  expect(core.calls).not.toContain('addCalendars');
  expect(core.offered.get(alice.id)).toEqual([work, home]);
});

test('Calendars that couldn’t be added or loaded are reported in the sidebar', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalDavAccountNow('alice', [work]);
  core.addCalendars = async () => {
    throw { code: 'serverUnreachable' };
  };
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
  const core = new FakeCore();
  const alice = core.addCalDavAccountNow('alice', [work]);
  core.addServerCalendar(alice.id, work.url);
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

  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Work' })).not.toBeInTheDocument(),
  );
  expect(core.calls).toContain('removeCalendar');
  expect(core.calls).not.toContain('deleteCalendar');
});

test('a Local Calendar’s menu still offers Delete', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Notes' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Options for Notes' }));

  const menu = screen.getByRole('menu', { name: 'Options for Notes' });
  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toBeInTheDocument();
  expect(
    within(menu).queryByRole('menuitem', { name: 'Remove from FreeCal' }),
  ).not.toBeInTheDocument();
});

test('removes a whole Account only after confirming', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const alice = core.addCalDavAccountNow('alice', [work]);
  core.addServerCalendar(alice.id, work.url);
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
  expect(core.calls).not.toContain('removeAccount');

  confirmation = await remove();
  expect(confirmation).toHaveTextContent('Nothing changes on the server.');
  await user.click(within(confirmation).getByRole('button', { name: 'Remove account' }));

  await waitFor(() => expect(screen.queryByRole('list', { name: ALICE })).not.toBeInTheDocument());
  expect(core.keyring.has(alice.id)).toBe(false);
  expect(screen.getByRole('list', { name: 'Local' })).toBeInTheDocument();
});

test('the Local Account has no Account menu', async () => {
  const core = new FakeCore();
  await showApp(core);

  expect(screen.queryByRole('button', { name: 'Options for Local' })).not.toBeInTheDocument();
});

test('hostile Calendar names from the server are text in the chooser, the sidebar and the remove dialog', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.caldavServer = [
    { url: 'https://dav.example.org/alice/x/', name: hostile.calendarName, colour: null },
  ];
  await showApp(core);

  await addAccount(user);
  const chooser = await screen.findByRole('dialog', { name: `Calendars of ${ALICE}` });
  expect(within(chooser).getByRole('checkbox', { name: hostile.calendarName })).toBeChecked();
  await user.click(within(chooser).getByRole('button', { name: 'Save' }));
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
  const core = new FakeCore();
  core.addCalDavAccountNow(hostile.username);
  await showApp(core);
  const name = hostile.expected.accountName;

  const dialog = await manage(user, name);
  await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await user.click(screen.getByRole('button', { name: `Options for ${name}` }));
  await user.click(screen.getByRole('menuitem', { name: 'Remove account…' }));

  expect(screen.getByRole('alertdialog', { name: `Remove ${name}?` })).toBeInTheDocument();
  expect(document.querySelector('img')).toBeNull();
});
