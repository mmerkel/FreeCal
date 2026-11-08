import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, onTestFinished, test, vi } from 'vitest';
import App from '../App.svelte';
import { expectAnchoredTo } from '../test/anchored';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar } from '../core/types';
import { caldavAccount, calendar, firstLaunch, localAccount } from '../test/answers';
import { failure, type RecordingCore } from '../test/recordingCore';

// The sidebar's Calendars, run against the recording fake core. A test that
// changes them checks the call, then scripts the Calendars as the core would
// have them and emits "Calendars changed".

/** A first launch with `calendars` in the Local Account. */
function withCalendars(...calendars: Calendar[]) {
  const core = firstLaunch();
  core.answer('listCalendars', calendars);
  return core;
}

/** Scripts `calendars` and emits "Calendars changed", as the core does after a change. */
function calendarsChanged(core: RecordingCore, ...calendars: Calendar[]) {
  core.answer('listCalendars', calendars);
  core.emit({ kind: 'calendarsChanged' });
}

async function showApp(core: RecordingCore) {
  render(App, { core });
  return screen.findByRole('list', { name: 'Local' });
}

/** Opens a Calendar's menu with its ⋮ button. */
async function openMenu(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(screen.getByRole('button', { name: `Options for ${name}` }));
  return screen.getByRole('menu', { name: `Options for ${name}` });
}

test('a CalDAV Account is named by its hostile username as text', async () => {
  const core = firstLaunch();
  core.answer('listAccounts', [localAccount, caldavAccount(hostile.username)]);

  await showApp(core);

  expect(screen.getByRole('list', { name: hostile.expected.accountName })).toBeInTheDocument();
  expect(document.querySelector('img')).toBeNull();
});

test('shows each Calendar under its Account as a switch', async () => {
  const core = withCalendars(calendar({ name: 'Home' }), calendar({ name: 'Work', shown: false }));

  const local = await showApp(core);

  expect(within(local).getByRole('switch', { name: 'Home' })).toBeChecked();
  expect(within(local).getByRole('switch', { name: 'Work' })).not.toBeChecked();
});

test('an Account’s full name is in its heading’s tooltip', async () => {
  const core = withCalendars();

  await showApp(core);

  expect(screen.getByRole('heading', { name: 'Local' })).toHaveAttribute('title', 'Local');
});

test('with no Calendars, the Local Account shows a hint until one is created', async () => {
  const user = userEvent.setup();
  const core = withCalendars();
  await showApp(core);
  const hint = 'No calendars yet. Use + to create one.';

  expect(screen.getByText(hint)).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Enter}');
  calendarsChanged(core, calendar({ name: 'Home' }));

  await screen.findByRole('switch', { name: 'Home' });
  expect(screen.queryByText(hint)).not.toBeInTheDocument();
});

test('a Calendar’s tooltip says whether it is shown and what a click does', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work', shown: false });
  const core = withCalendars(home, work);
  await showApp(core);

  expect(screen.getByRole('switch', { name: 'Home' })).toHaveAttribute(
    'title',
    'Home: shown, click to hide',
  );
  expect(screen.getByRole('switch', { name: 'Work' })).toHaveAttribute(
    'title',
    'Work: hidden, click to show',
  );

  await user.click(screen.getByRole('switch', { name: 'Home' }));
  calendarsChanged(core, { ...home, shown: false }, work);

  await waitFor(() =>
    expect(screen.getByRole('switch', { name: 'Home' })).toHaveAttribute(
      'title',
      'Home: hidden, click to show',
    ),
  );
});

test('a Read-only Calendar’s row shows a lock, which describes it', async () => {
  const core = withCalendars(
    calendar({ name: 'Home' }),
    calendar({ name: 'Holidays', readOnly: true }),
  );

  const local = await showApp(core);

  const holidays = within(local).getByRole('switch', { name: 'Holidays' });
  expect(holidays).toHaveAccessibleDescription('Read-only Holidays: shown, click to hide');
  expect(holidays.querySelector('[title="Read-only"] svg')).not.toBeNull();
  const home = within(local).getByRole('switch', { name: 'Home' });
  expect(home.querySelector('[title="Read-only"]')).toBeNull();
  expect(home).toHaveAccessibleDescription('Home: shown, click to hide');
});

test('clicking a Calendar’s row hides and shows it', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  await showApp(core);

  await user.click(screen.getByRole('switch', { name: 'Home' }));
  expect(core.callsOf('setCalendarShown')).toEqual([[home.id, false]]);
  calendarsChanged(core, { ...home, shown: false });
  await waitFor(() => expect(screen.getByRole('switch', { name: 'Home' })).not.toBeChecked());

  await user.click(screen.getByRole('switch', { name: 'Home' }));
  expect(core.callsOf('setCalendarShown')).toEqual([
    [home.id, false],
    [home.id, true],
  ]);
  calendarsChanged(core, home);
  await waitFor(() => expect(screen.getByRole('switch', { name: 'Home' })).toBeChecked());
});

test('a Calendar can be hidden with the keyboard', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  screen.getByRole('switch', { name: 'Home' }).focus();
  await user.keyboard(' ');

  expect(core.callsOf('setCalendarShown')).toEqual([[expect.anything(), false]]);
});

test('creates a Calendar in the Local Account', async () => {
  const user = userEvent.setup();
  const core = withCalendars();
  const local = await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Enter}');

  expect(core.callsOf('createCalendar')).toEqual([
    [localAccount.id, 'Home', expect.stringMatching(/^#[0-9a-f]{6}$/)],
  ]);
  expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument();
  calendarsChanged(core, calendar({ name: 'Home' }));
  expect(await within(local).findByRole('switch', { name: 'Home' })).toBeChecked();
});

test('a new Calendar takes the next colour in turn that no Calendar uses yet', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home', colour: '#ad1457' });
  const core = withCalendars(home);
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Work{Enter}');
  calendarsChanged(core, home, calendar({ name: 'Work', colour: '#3f51b5' }));
  await screen.findByRole('switch', { name: 'Work' });
  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Birthdays{Enter}');

  expect(core.callsOf('createCalendar').map(([, , colour]) => colour)).toEqual([
    '#3f51b5',
    '#c0ca33',
  ]);
});

test('a Calendar’s menu offers 24 colours', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');

  expect(within(menu).getAllByRole('menuitemradio')).toHaveLength(24);
});

test('cancelling a new Calendar creates nothing', async () => {
  const user = userEvent.setup();
  const core = withCalendars();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Escape}');

  expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument();
  expect(core.callsOf('createCalendar')).toEqual([]);
});

test('renames a Calendar from its menu', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  const name = screen.getByRole('textbox', { name: 'Name' });
  expect(name).toHaveValue('Home');
  await user.clear(name);
  await user.type(name, 'Family{Enter}');

  expect(core.callsOf('renameCalendar')).toEqual([[home.id, 'Family']]);
  calendarsChanged(core, { ...home, name: 'Family' });
  expect(await screen.findByRole('switch', { name: 'Family' })).toBeInTheDocument();
});

test('Escape leaves a Calendar’s name unchanged', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), ' and away{Escape}');

  expect(screen.getByRole('switch', { name: 'Home' })).toBeInTheDocument();
  expect(core.callsOf('renameCalendar')).toEqual([]);
});

test('recolours a Calendar from the swatches in its menu', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home', colour: '#039be5' });
  const core = withCalendars(home);
  await showApp(core);

  let menu = await openMenu(user, 'Home');
  expect(within(menu).getByRole('menuitemradio', { name: 'Light blue' })).toBeChecked();
  await user.click(within(menu).getByRole('menuitemradio', { name: 'Green' }));

  expect(core.callsOf('recolourCalendar')).toEqual([[home.id, '#0b8043']]);
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  calendarsChanged(core, { ...home, colour: '#0b8043' });
  await waitFor(() =>
    expect(screen.getByRole('switch', { name: 'Home' }).querySelector('.calendar-dot')).toHaveStyle(
      { '--colour': '#0b8043' },
    ),
  );
  menu = await openMenu(user, 'Home');
  expect(within(menu).getByRole('menuitemradio', { name: 'Green' })).toBeChecked();
  expect(within(menu).getByRole('menuitemradio', { name: 'Light blue' })).not.toBeChecked();
});

test('right-click on a Calendar’s row opens its menu', async () => {
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const notPrevented = await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));

  expect(notPrevented).toBe(false);
  expect(screen.getByRole('menu', { name: 'Options for Home' })).toBeInTheDocument();
});

test.each(['{Shift>}{F10}{/Shift}', '{ContextMenu}'])(
  '%s on a focused Calendar row opens its menu',
  async (keys) => {
    const user = userEvent.setup();
    await showApp(withCalendars(calendar({ name: 'Home' })));

    screen.getByRole('switch', { name: 'Home' }).focus();
    await user.keyboard(keys);

    const menu = screen.getByRole('menu', { name: 'Options for Home' });
    expect(menu).toContainElement(document.activeElement as HTMLElement);
    expectAnchoredTo(menu, screen.getByRole('button', { name: 'Options for Home' }));
  },
);

test('the menu is used with arrow keys and closed with Escape', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  const firstSwatch = within(menu).getAllByRole('menuitemradio')[0];
  expect(firstSwatch).toHaveFocus();
  await user.keyboard('{End}');
  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toHaveFocus();
  await user.keyboard('{ArrowDown}');
  expect(firstSwatch).toHaveFocus();
  await user.keyboard('{ArrowUp}');
  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toHaveFocus();

  await user.keyboard('{Escape}');

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Options for Home' })).toHaveFocus();
});

test('a click elsewhere closes the menu', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(document.body);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('the ⋮ button closes the menu it opened', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(screen.getByRole('button', { name: 'Options for Home' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('a menu opened by right-click closes on a click elsewhere', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);
  (document.activeElement as HTMLElement | null)?.blur();

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));
  expect(screen.getByRole('menu')).toBeInTheDocument();
  await user.click(document.body);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('a click on another Calendar while a menu is open only closes the menu', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }), calendar({ name: 'Work' }));
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(screen.getByRole('switch', { name: 'Work' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(core.callsOf('setCalendarShown')).toEqual([]);
  expect(screen.getByRole('switch', { name: 'Work' })).toBeChecked();
});

test('a click on its own row only closes a menu opened by right-click', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));
  expect(screen.getByRole('menu')).toBeInTheDocument();
  await user.click(screen.getByRole('switch', { name: 'Home' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(core.callsOf('setCalendarShown')).toEqual([]);
});

test('a Calendar’s menu is shown as a popover anchored to its ⋮ button', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);
  const shown = vi.spyOn(HTMLElement.prototype, 'showPopover');
  onTestFinished(() => shown.mockRestore());

  const menu = await openMenu(user, 'Home');

  expect(menu).toHaveAttribute('popover', 'manual');
  expect(shown.mock.contexts).toEqual([menu]);
  expectAnchoredTo(menu, screen.getByRole('button', { name: 'Options for Home' }));
});

test('a menu opened by right-click on a row is anchored to its ⋮ button', async () => {
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));

  expectAnchoredTo(
    screen.getByRole('menu'),
    screen.getByRole('button', { name: 'Options for Home' }),
  );
});

test('an open menu keeps its focus when the Calendars change', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  await showApp(core);
  const shown = vi.spyOn(HTMLElement.prototype, 'showPopover');
  onTestFinished(() => shown.mockRestore());

  const menu = await openMenu(user, 'Home');
  await user.keyboard('{End}');
  calendarsChanged(core, home, calendar({ name: 'Work' }));
  await screen.findByRole('switch', { name: 'Work' });

  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toHaveFocus();
  expect(shown).toHaveBeenCalledTimes(1);
});

test('a scroll outside the open menu closes it, and focus returns to its ⋮', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  const local = await showApp(core);

  await openMenu(user, 'Home');
  await fireEvent.scroll(local);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Options for Home' })).toHaveFocus();
});

test('a scroll inside the open menu leaves it open', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await fireEvent.scroll(menu);

  expect(menu).toBeInTheDocument();
});

test('“Show only this” shows one Calendar and hides all others', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work', shown: false });
  const birthdays = calendar({ name: 'Birthdays' });
  const core = withCalendars(home, work, birthdays);
  await showApp(core);

  const menu = await openMenu(user, 'Work');
  await user.click(within(menu).getByRole('menuitem', { name: 'Show only this' }));

  expect(core.callsOf('showOnlyCalendar')).toEqual([[work.id]]);
  calendarsChanged(
    core,
    { ...home, shown: false },
    { ...work, shown: true },
    { ...birthdays, shown: false },
  );
  await waitFor(() => expect(screen.getByRole('switch', { name: 'Work' })).toBeChecked());
  expect(screen.getByRole('switch', { name: 'Home' })).not.toBeChecked();
  expect(screen.getByRole('switch', { name: 'Birthdays' })).not.toBeChecked();
});

test('deletes a Calendar only after confirming in a dialog', async () => {
  const user = userEvent.setup();
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work' });
  const core = withCalendars(home, work);
  await showApp(core);

  let menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  const confirmation = screen.getByRole('alertdialog', { name: 'Delete “Home”?' });
  expect(within(confirmation).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(core.callsOf('deleteCalendar')).toEqual([]);
  expect(screen.getByRole('button', { name: 'Options for Home' })).toHaveFocus();

  menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }));

  expect(core.callsOf('deleteCalendar')).toEqual([[home.id]]);
  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  calendarsChanged(core, work);
  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Home' })).not.toBeInTheDocument(),
  );
  expect(screen.getByRole('switch', { name: 'Work' })).toBeInTheDocument();
});

test('Escape cancels deleting a Calendar', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(core.callsOf('deleteCalendar')).toEqual([]);
});

test('reads the Calendars again on “Calendars changed”', async () => {
  const core = withCalendars();
  await showApp(core);

  calendarsChanged(core, calendar({ name: 'Added elsewhere' }));

  expect(await screen.findByRole('switch', { name: 'Added elsewhere' })).toBeInTheDocument();
});

test('a failed change is reported', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  core.answer('setCalendarShown', failure({ code: 'localStoreFailed' }));
  await showApp(core);

  await user.click(screen.getByRole('switch', { name: 'Home' }));

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'FreeCal’s database couldn’t be read or written',
  );
});

test('a hostile Calendar name is shown as text in the row, the menu and the dialog', async () => {
  const user = userEvent.setup();
  const hostile = '<img src=x onerror="window.pwned=1"><script>window.pwned=1</script>';
  const core = withCalendars(calendar({ name: hostile }));

  const local = await showApp(core);
  expect(within(local).getByRole('switch', { name: hostile })).toBeInTheDocument();
  const menu = await openMenu(user, hostile);
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));

  expect(screen.getByRole('alertdialog', { name: `Delete “${hostile}”?` })).toBeInTheDocument();
  expect(document.querySelector('img, script')).toBeNull();
});

test('cancelling clears the report of a failed change', async () => {
  const user = userEvent.setup();
  const core = withCalendars(calendar({ name: 'Home' }));
  core.answer('renameCalendar', failure({ code: 'emptyCalendarName' }));
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), '{Enter}');
  expect(await screen.findByRole('alert')).toHaveTextContent('needs a name');
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
