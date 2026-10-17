import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, onTestFinished, test, vi } from 'vitest';
import App from '../App.svelte';
import { expectAnchoredTo } from '../test/anchored';
import { FakeCore } from '../test/fakeCore';

async function showApp(core: FakeCore) {
  render(App, { core });
  return screen.findByRole('list', { name: 'Local' });
}

/** Opens a Calendar's menu with its ⋮ button. */
async function openMenu(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(screen.getByRole('button', { name: `Options for ${name}` }));
  return screen.getByRole('menu', { name: `Options for ${name}` });
}

test('subscribes to Signals before reading anything a Signal could change', async () => {
  const core = new FakeCore();

  await showApp(core);

  // Whether FreeCal started comes first; no Signal changes that.
  expect(core.calls.slice(0, 2)).toEqual(['startupFailure', 'subscribe']);
  expect(core.calls).toContain('listCalendars');
});

test('shows each Calendar under its Account as a switch', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work', shown: false });

  const local = await showApp(core);

  expect(within(local).getByRole('switch', { name: 'Home' })).toBeChecked();
  expect(within(local).getByRole('switch', { name: 'Work' })).not.toBeChecked();
});

test('an Account’s full name is in its heading’s tooltip', async () => {
  const core = new FakeCore();

  await showApp(core);

  expect(screen.getByRole('heading', { name: 'Local' })).toHaveAttribute('title', 'Local');
});

test('with no Calendars, the Local Account shows a hint until one is created', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  await showApp(core);
  const hint = 'No calendars yet. Use + to create one.';

  expect(screen.getByText(hint)).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Enter}');

  await screen.findByRole('switch', { name: 'Home' });
  expect(screen.queryByText(hint)).not.toBeInTheDocument();
});

test('a Calendar’s tooltip says whether it is shown and what a click does', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work', shown: false });
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

  await waitFor(() =>
    expect(screen.getByRole('switch', { name: 'Home' })).toHaveAttribute(
      'title',
      'Home: hidden, click to show',
    ),
  );
});

test('clicking a Calendar’s row hides and shows it', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.click(screen.getByRole('switch', { name: 'Home' }));
  await waitFor(() => expect(screen.getByRole('switch', { name: 'Home' })).not.toBeChecked());
  expect(core.calendars.find((calendar) => calendar.id === home.id)?.shown).toBe(false);

  await user.click(screen.getByRole('switch', { name: 'Home' }));
  await waitFor(() => expect(screen.getByRole('switch', { name: 'Home' })).toBeChecked());
});

test('a Calendar can be hidden with the keyboard', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  screen.getByRole('switch', { name: 'Home' }).focus();
  await user.keyboard(' ');

  await waitFor(() => expect(screen.getByRole('switch', { name: 'Home' })).not.toBeChecked());
});

test('creates a Calendar in the Local Account', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const local = await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Enter}');

  expect(await within(local).findByRole('switch', { name: 'Home' })).toBeChecked();
  expect(core.calendars).toMatchObject([{ accountId: 1, name: 'Home' }]);
  expect(core.calendars[0].colour).toMatch(/^#[0-9a-f]{6}$/);
  expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument();
});

test('a new Calendar takes the next colour in turn that no Calendar uses yet', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home', colour: '#ad1457' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Work{Enter}');
  await screen.findByRole('switch', { name: 'Work' });
  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Birthdays{Enter}');
  await screen.findByRole('switch', { name: 'Birthdays' });

  expect(core.calendars.map((calendar) => calendar.colour)).toEqual([
    '#ad1457',
    '#3f51b5',
    '#c0ca33',
  ]);
});

test('a Calendar’s menu offers 24 colours', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const menu = await openMenu(user, 'Home');

  expect(within(menu).getAllByRole('menuitemradio')).toHaveLength(24);
});

test('cancelling a new Calendar creates nothing', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Escape}');

  expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument();
  expect(core.calls).not.toContain('createCalendar');
});

test('renames a Calendar from its menu', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  const name = screen.getByRole('textbox', { name: 'Name' });
  expect(name).toHaveValue('Home');
  await user.clear(name);
  await user.type(name, 'Family{Enter}');

  expect(await screen.findByRole('switch', { name: 'Family' })).toBeInTheDocument();
  expect(core.calendars[0].name).toBe('Family');
});

test('Escape leaves a Calendar’s name unchanged', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), ' and away{Escape}');

  expect(screen.getByRole('switch', { name: 'Home' })).toBeInTheDocument();
  expect(core.calls).not.toContain('renameCalendar');
});

test('recolours a Calendar from the swatches in its menu', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home', colour: '#039be5' });
  await showApp(core);

  let menu = await openMenu(user, 'Home');
  expect(within(menu).getByRole('menuitemradio', { name: 'Light blue' })).toBeChecked();
  await user.click(within(menu).getByRole('menuitemradio', { name: 'Green' }));

  await waitFor(() => expect(core.calendars[0].colour).toBe('#0b8043'));
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  menu = await openMenu(user, 'Home');
  expect(within(menu).getByRole('menuitemradio', { name: 'Green' })).toBeChecked();
  expect(within(menu).getByRole('menuitemradio', { name: 'Light blue' })).not.toBeChecked();
});

test('right-click on a Calendar’s row opens its menu', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const notPrevented = await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));

  expect(notPrevented).toBe(false);
  expect(screen.getByRole('menu', { name: 'Options for Home' })).toBeInTheDocument();
});

test.each(['{Shift>}{F10}{/Shift}', '{ContextMenu}'])(
  '%s on a focused Calendar row opens its menu',
  async (keys) => {
    const user = userEvent.setup();
    const core = new FakeCore();
    core.addCalendar({ name: 'Home' });
    await showApp(core);

    screen.getByRole('switch', { name: 'Home' }).focus();
    await user.keyboard(keys);

    const menu = screen.getByRole('menu', { name: 'Options for Home' });
    expect(menu).toContainElement(document.activeElement as HTMLElement);
    expectAnchoredTo(menu, screen.getByRole('button', { name: 'Options for Home' }));
  },
);

test('the menu is used with arrow keys and closed with Escape', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
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
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(document.body);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('the ⋮ button closes the menu it opened', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(screen.getByRole('button', { name: 'Options for Home' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('a menu opened by right-click closes on a click elsewhere', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);
  (document.activeElement as HTMLElement | null)?.blur();

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));
  expect(screen.getByRole('menu')).toBeInTheDocument();
  await user.click(document.body);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('a click on another Calendar while a menu is open only closes the menu', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work' });
  await showApp(core);

  await openMenu(user, 'Home');
  await user.click(screen.getByRole('switch', { name: 'Work' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('setCalendarShown');
  expect(screen.getByRole('switch', { name: 'Work' })).toBeChecked();
});

test('a click on its own row only closes a menu opened by right-click', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));
  expect(screen.getByRole('menu')).toBeInTheDocument();
  await user.click(screen.getByRole('switch', { name: 'Home' }));

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('setCalendarShown');
});

test('a Calendar’s menu is shown as a popover anchored to its ⋮ button', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);
  const shown = vi.spyOn(HTMLElement.prototype, 'showPopover');
  onTestFinished(() => shown.mockRestore());

  const menu = await openMenu(user, 'Home');

  expect(menu).toHaveAttribute('popover', 'manual');
  expect(shown.mock.contexts).toEqual([menu]);
  expectAnchoredTo(menu, screen.getByRole('button', { name: 'Options for Home' }));
});

test('a menu opened by right-click on a row is anchored to its ⋮ button', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await fireEvent.contextMenu(screen.getByRole('switch', { name: 'Home' }));

  expectAnchoredTo(
    screen.getByRole('menu'),
    screen.getByRole('button', { name: 'Options for Home' }),
  );
});

test('an open menu keeps its focus when the Calendars change', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);
  const shown = vi.spyOn(HTMLElement.prototype, 'showPopover');
  onTestFinished(() => shown.mockRestore());

  const menu = await openMenu(user, 'Home');
  await user.keyboard('{End}');
  core.addCalendar({ name: 'Work' });
  core.emit({ kind: 'calendarsChanged' });
  await screen.findByRole('switch', { name: 'Work' });

  expect(within(menu).getByRole('menuitem', { name: 'Delete' })).toHaveFocus();
  expect(shown).toHaveBeenCalledTimes(1);
});

test('a scroll outside the open menu closes it, and focus returns to its ⋮', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const local = await showApp(core);

  await openMenu(user, 'Home');
  await fireEvent.scroll(local);

  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Options for Home' })).toHaveFocus();
});

test('a scroll inside the open menu leaves it open', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await fireEvent.scroll(menu);

  expect(menu).toBeInTheDocument();
});

test('“Show only this” shows one Calendar and hides all others', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work', shown: false });
  core.addCalendar({ name: 'Birthdays' });
  await showApp(core);

  const menu = await openMenu(user, 'Work');
  await user.click(within(menu).getByRole('menuitem', { name: 'Show only this' }));

  await waitFor(() => expect(screen.getByRole('switch', { name: 'Work' })).toBeChecked());
  expect(screen.getByRole('switch', { name: 'Home' })).not.toBeChecked();
  expect(screen.getByRole('switch', { name: 'Birthdays' })).not.toBeChecked();
  expect(core.calls.filter((call) => call === 'showOnlyCalendar')).toHaveLength(1);
});

test('deletes a Calendar only after confirming in a dialog', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work' });
  await showApp(core);

  let menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  const confirmation = screen.getByRole('alertdialog', { name: 'Delete “Home”?' });
  expect(within(confirmation).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('deleteCalendar');
  expect(screen.getByRole('button', { name: 'Options for Home' })).toHaveFocus();

  menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }));

  await waitFor(() =>
    expect(screen.queryByRole('switch', { name: 'Home' })).not.toBeInTheDocument(),
  );
  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(screen.getByRole('switch', { name: 'Work' })).toBeInTheDocument();
});

test('Escape cancels deleting a Calendar', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('deleteCalendar');
});

test('reads the Calendars again on “Calendars changed”', async () => {
  const core = new FakeCore();
  await showApp(core);

  core.addCalendar({ name: 'Added elsewhere' });
  core.emit({ kind: 'calendarsChanged' });

  expect(await screen.findByRole('switch', { name: 'Added elsewhere' })).toBeInTheDocument();
});

test('a failed change is reported', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.setCalendarShown = async () => {
    throw 'the Local Store failed: disk full';
  };
  await showApp(core);

  await user.click(screen.getByRole('switch', { name: 'Home' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('disk full');
});

test('a hostile Calendar name is shown as text in the row, the menu and the dialog', async () => {
  const user = userEvent.setup();
  const hostile = '<img src=x onerror="window.pwned=1"><script>window.pwned=1</script>';
  const core = new FakeCore();
  core.addCalendar({ name: hostile });

  const local = await showApp(core);
  expect(within(local).getByRole('switch', { name: hostile })).toBeInTheDocument();
  const menu = await openMenu(user, hostile);
  await user.click(within(menu).getByRole('menuitem', { name: 'Delete' }));

  expect(screen.getByRole('alertdialog', { name: `Delete “${hostile}”?` })).toBeInTheDocument();
  expect(document.querySelector('img, script')).toBeNull();
});

test('cancelling clears the report of a failed change', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.renameCalendar = async () => {
    throw 'a Calendar needs a name';
  };
  await showApp(core);

  const menu = await openMenu(user, 'Home');
  await user.click(within(menu).getByRole('menuitem', { name: 'Rename' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), '{Enter}');
  expect(await screen.findByRole('alert')).toHaveTextContent('needs a name');
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
