import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';
import App from '../App.svelte';
import { FakeCore } from '../test/fakeCore';

async function showApp(core: FakeCore) {
  render(App, { core });
  return screen.findByRole('list', { name: 'Local' });
}

test('subscribes to Signals before reading anything', async () => {
  const core = new FakeCore();

  await showApp(core);

  expect(core.calls[0]).toBe('subscribe');
  expect(core.calls).toContain('listCalendars');
});

test('shows each Calendar under its Account', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work', shown: false });

  const local = await showApp(core);

  expect(within(local).getByRole('checkbox', { name: 'Home' })).toBeChecked();
  expect(within(local).getByRole('checkbox', { name: 'Work' })).not.toBeChecked();
});

test('hides and shows a Calendar', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.click(screen.getByRole('checkbox', { name: 'Home' }));
  await waitFor(() => expect(screen.getByRole('checkbox', { name: 'Home' })).not.toBeChecked());
  expect(core.calendars.find((calendar) => calendar.id === home.id)?.shown).toBe(false);

  await user.click(screen.getByRole('checkbox', { name: 'Home' }));
  await waitFor(() => expect(screen.getByRole('checkbox', { name: 'Home' })).toBeChecked());
});

test('creates a Calendar in the Local Account', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const local = await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Home{Enter}');

  expect(await within(local).findByRole('checkbox', { name: 'Home' })).toBeChecked();
  expect(core.calendars).toMatchObject([{ accountId: 1, name: 'Home' }]);
  expect(core.calendars[0].colour).toMatch(/^#[0-9a-f]{6}$/);
  expect(screen.queryByRole('textbox', { name: 'Name' })).not.toBeInTheDocument();
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

test('renames a Calendar', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Rename Home' }));
  const name = screen.getByRole('textbox', { name: 'Name' });
  expect(name).toHaveValue('Home');
  await user.clear(name);
  await user.type(name, 'Family{Enter}');

  expect(await screen.findByRole('checkbox', { name: 'Family' })).toBeInTheDocument();
  expect(core.calendars[0].name).toBe('Family');
});

test('Escape leaves a Calendar’s name unchanged', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Rename Home' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), ' and away{Escape}');

  expect(screen.getByRole('checkbox', { name: 'Home' })).toBeInTheDocument();
  expect(core.calls).not.toContain('renameCalendar');
});

test('recolours a Calendar', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home', colour: '#3366cc' });
  await showApp(core);

  const colour = screen.getByLabelText('Colour of Home');
  expect(colour).toHaveValue('#3366cc');
  await fireEvent.change(colour, { target: { value: '#ff8800' } });

  await waitFor(() => expect(core.calendars[0].colour).toBe('#ff8800'));
  await waitFor(() => expect(screen.getByLabelText('Colour of Home')).toHaveValue('#ff8800'));
});

test('deletes a Calendar only after confirmation', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.addCalendar({ name: 'Work' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Delete Home' }));
  const confirmation = screen.getByRole('alertdialog', {
    name: 'Delete “Home” and all its Events?',
  });
  await user.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
  expect(core.calls).not.toContain('deleteCalendar');
  expect(screen.getByRole('checkbox', { name: 'Home' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Delete Home' }));
  await user.click(
    within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Delete' }),
  );

  await waitFor(() =>
    expect(screen.queryByRole('checkbox', { name: 'Home' })).not.toBeInTheDocument(),
  );
  expect(screen.getByRole('checkbox', { name: 'Work' })).toBeInTheDocument();
});

test('reads the Calendars again on “Calendars changed”', async () => {
  const core = new FakeCore();
  await showApp(core);

  core.addCalendar({ name: 'Added elsewhere' });
  core.emit({ kind: 'calendarsChanged' });

  expect(await screen.findByRole('checkbox', { name: 'Added elsewhere' })).toBeInTheDocument();
});

test('a failed change is reported', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.setCalendarShown = async () => {
    throw 'the Local Store failed: disk full';
  };
  await showApp(core);

  await user.click(screen.getByRole('checkbox', { name: 'Home' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('disk full');
});

test('a hostile Calendar name is shown as text', async () => {
  const hostile = '<img src=x onerror="window.pwned=1"><script>window.pwned=1</script>';
  const core = new FakeCore();
  core.addCalendar({ name: hostile });

  const local = await showApp(core);

  expect(within(local).getByRole('checkbox', { name: hostile })).toBeInTheDocument();
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

  await user.click(screen.getByRole('button', { name: 'Rename Home' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), '{Enter}');
  expect(await screen.findByRole('alert')).toHaveTextContent('needs a name');
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
