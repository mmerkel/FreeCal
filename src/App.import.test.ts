// The import flow: picking a file, the preview, running and undoing an
// import, run against the fake core with a stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import App from './App.svelte';
import type { When } from './core/types';
import { FakeCore, type FakeImportFile } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

const monday: When = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' };

const workFile: FakeImportFile = {
  name: 'work.ics',
  events: [
    { uid: 'standup', title: 'Standup', when: monday },
    { uid: 'review', title: 'Review', when: monday },
  ],
};

async function showApp(core: FakeCore) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
  return userEvent.setup();
}

async function chooseImport(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Import…' }));
}

/** Opens the import dialog for `file` and waits for its preview. */
async function openImport(
  user: ReturnType<typeof userEvent.setup>,
  core: FakeCore,
  file = workFile,
) {
  core.importFile = file;
  await chooseImport(user);
  const dialog = await screen.findByRole('dialog', { name: `Import “\u2068${file.name}\u2069”` });
  await within(dialog).findByText(/new event/);
  return dialog;
}

function grid() {
  return screen.getByRole('list', { name: 'Grid' });
}

test('Import… asks for a file in the system’s file dialog, with translated words', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);

  await chooseImport(user);

  expect(core.fileDialogs).toEqual([
    { title: 'Import events', filter: 'Calendar files (.ics, .zip)' },
  ]);
  // The user cancelled the file dialog: nothing else happens.
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.calls).not.toContain('previewImport');
});

test('the import dialog offers only writable Calendars and previews the first shown one', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Hidden', shown: false });
  core.addCalendar({ name: 'Holidays', readOnly: true });
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);

  const dialog = await openImport(user, core);

  const calendar = within(dialog).getByRole('combobox', { name: 'Into calendar' });
  expect(
    within(calendar)
      .getAllByRole('option')
      .map((option) => option.textContent),
  ).toEqual(['Hidden', 'Home']);
  expect(calendar).toHaveDisplayValue('Home');
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();
  // Nothing exists yet, so there is nothing to choose.
  expect(within(dialog).queryByRole('radio')).not.toBeInTheDocument();
});

test('with Events already in the Calendar, the dialog asks once what to do with them', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const work = core.addCalendar({ name: 'Work' });
  core.addEvent({ calendarId: work.id, title: 'Old standup', when: monday, uid: 'standup' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();

  await user.selectOptions(within(dialog).getByRole('combobox', { name: 'Into calendar' }), 'Work');

  expect(await within(dialog).findByText('1 new event')).toBeInTheDocument();
  expect(within(dialog).getByText('1 event is already in this calendar')).toBeInTheDocument();
  const choice = within(dialog).getByRole('group', { name: 'Events already in the calendar' });
  expect(
    within(choice)
      .getAllByRole('radio')
      .map((radio) => radio.parentElement!.textContent!.trim()),
  ).toEqual(['Update them', 'Skip them', 'Import them as copies']);
  expect(within(choice).getByRole('radio', { name: 'Update them' })).toBeChecked();

  await user.click(within(choice).getByRole('radio', { name: 'Skip them' }));
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.events.filter((event) => event.calendarId === work.id).map((e) => e.title)).toEqual([
    'Old standup',
    'Review',
  ]);
  expect(core.events.some((event) => event.calendarId === home.id)).toBe(false);
});

test('the preview names the zones floating times were pinned to', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);

  const dialog = await openImport(user, core, {
    ...workFile,
    pinned: [
      { zone: 'Europe/Berlin', events: 2 },
      { zone: 'UTC', events: 1 },
    ],
  });

  expect(
    within(dialog).getByText(
      '2 events have times without a time zone; they will use Europe/Berlin.',
    ),
  ).toBeInTheDocument();
  expect(
    within(dialog).getByText('1 event has times without a time zone; it will use UTC.'),
  ).toBeInTheDocument();
});

test('a file without Events says so and can’t be imported', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.importFile = { name: 'empty.ics', events: [] };
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Import “\u2068empty.ics\u2069”' });
  expect(await within(dialog).findByText('The file holds no events.')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Import' })).toBeDisabled();
});

test('the import runs under the modal “Working…” dialog, which can’t be closed', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  let finish!: () => void;
  const run = core.runImport.bind(core);
  vi.spyOn(core, 'runImport').mockImplementation(async (...args) => {
    await new Promise<void>((resolve) => (finish = resolve));
    return run(...args);
  });

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const working = await screen.findByRole('dialog', { name: 'Working…' });
  expect(working).toHaveAttribute('open');
  expect(within(working).getByText('Importing “\u2068work.ics\u2069”…')).toBeInTheDocument();
  expect(within(working).queryByRole('button')).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  expect(screen.getByRole('dialog', { name: 'Working…' })).toBeInTheDocument();

  finish();

  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.calls).not.toContain('discardImport');
  expect(await within(grid()).findByRole('button', { name: 'Standup' })).toBeInTheDocument();
});

test('after an import, Undo undoes it in one step', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Old standup', when: monday, uid: 'standup' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const toast = await screen.findByRole('status');
  expect(toast).toHaveTextContent(
    'Imported “\u2068work.ics\u2069”: 1 added, 1 updated, 0 skipped.',
  );
  expect(
    within(grid())
      .getAllByRole('button')
      .map((button) => button.textContent!.trim()),
  ).toEqual(['Standup', 'Review']);

  await user.click(within(toast).getByRole('button', { name: 'Undo' }));

  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent(
      'The import of “\u2068work.ics\u2069” was undone.',
    ),
  );
  expect(within(screen.getByRole('status')).queryByRole('button', { name: 'Undo' })).toBeNull();
  await waitFor(() =>
    expect(
      within(grid())
        .getAllByRole('button')
        .map((b) => b.textContent!.trim()),
    ).toEqual(['Old standup']),
  );

  await user.click(within(screen.getByRole('status')).getByRole('button', { name: 'Dismiss' }));
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});

test('undoing runs under “Working…” too, and a refused undo says why and can be tried again', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));
  const toast = await screen.findByRole('status');
  home.readOnly = true;
  core.calendars[0].readOnly = true;
  let finish!: () => void;
  const undo = core.undoImport.bind(core);
  vi.spyOn(core, 'undoImport').mockImplementationOnce(async (id) => {
    await new Promise<void>((resolve) => (finish = resolve));
    return undo(id);
  });

  await user.click(within(toast).getByRole('button', { name: 'Undo' }));

  const working = await screen.findByRole('dialog', { name: 'Working…' });
  expect(
    within(working).getByText('Undoing the import of “\u2068work.ics\u2069”…'),
  ).toBeInTheDocument();
  finish();
  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent(
      'The import of “\u2068work.ics\u2069” couldn’t be undone: the calendar is read-only',
    ),
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

  core.calendars[0].readOnly = false;
  await user.click(within(screen.getByRole('status')).getByRole('button', { name: 'Undo' }));
  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent(
      'The import of “\u2068work.ics\u2069” was undone.',
    ),
  );
});

test('an import that changed nothing offers no Undo', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Standup', when: monday, uid: 'standup' });
  const user = await showApp(core);
  const dialog = await openImport(user, core, { name: 'one.ics', events: [workFile.events[0]] });

  await user.click(within(dialog).getByRole('radio', { name: 'Skip them' }));
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const toast = await screen.findByRole('status');
  expect(toast).toHaveTextContent('Imported “\u2068one.ics\u2069”: 0 added, 0 updated, 1 skipped.');
  expect(within(toast).queryByRole('button', { name: 'Undo' })).not.toBeInTheDocument();
});

test('a failed import stays in the dialog and says why', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  core.calendars[0].readOnly = true;

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The import failed: the calendar is read-only',
  );
  expect(screen.queryByRole('dialog', { name: 'Working…' })).not.toBeInTheDocument();
  expect(core.pickedImport).toBeDefined();
});

test('Cancel lets go of the picked file', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);

  await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.calls).toContain('discardImport');
  expect(core.pickedImport).toBeUndefined();
  expect(core.calls).not.toContain('runImport');
});

test('a file that can’t be read gives a message naming it', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.importFile = { code: 'importTooLarge', file: 'takeout.zip' };
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Can’t import this file' });
  expect(dialog).toHaveTextContent(
    'The file couldn’t be opened: “\u2068takeout.zip\u2069” is too large to import',
  );
});

test('a preview that fails says why and can’t be imported', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.importFile = workFile;
  vi.spyOn(core, 'previewImport').mockRejectedValue({
    code: 'importNotCalendar',
    file: 'work.ics',
  });
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Import “\u2068work.ics\u2069”' });
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The file couldn’t be read: “\u2068work.ics\u2069” isn’t a calendar file FreeCal can read',
  );
  expect(within(dialog).getByRole('button', { name: 'Import' })).toBeDisabled();
});

test('without a writable Calendar, Import… says where to create one first', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Holidays', readOnly: true });
  core.importFile = workFile;
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = screen.getByRole('dialog', { name: 'No calendar to import into' });
  expect(dialog).toHaveTextContent('Create a calendar first with the + next to “Local”');
  expect(core.calls).not.toContain('pickImportFile');
});

test('a preview for a Calendar chosen earlier never overwrites the latest one', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const work = core.addCalendar({ name: 'Work' });
  core.addEvent({ calendarId: work.id, title: 'Old standup', when: monday, uid: 'standup' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  const preview = core.previewImport.bind(core);
  let finishSlow!: () => void;
  vi.spyOn(core, 'previewImport').mockImplementationOnce(async (calendarId) => {
    await new Promise<void>((resolve) => (finishSlow = resolve));
    return preview(calendarId);
  });
  const calendar = within(dialog).getByRole('combobox', { name: 'Into calendar' });

  await user.selectOptions(calendar, 'Work');
  await user.selectOptions(calendar, 'Home');
  await within(dialog).findByText('2 new events');
  finishSlow();

  await new Promise((resolve) => setTimeout(resolve));
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();
  expect(within(dialog).queryByRole('radio')).not.toBeInTheDocument();
});

test('the hostile file name and zone are shown only as text', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const user = await showApp(core);

  const dialog = await openImport(user, core, {
    name: hostile.fileName,
    events: [{ uid: 'hostile', title: hostile.title, when: monday }],
    pinned: [{ zone: hostile.pinnedZone, events: 1 }],
  });

  expect(within(dialog).getByRole('heading')).toHaveTextContent(hostile.expected.importTitle);
  expect(within(dialog).getByText(hostile.expected.importPinned)).toBeInTheDocument();
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));
  expect(await screen.findByRole('status')).toHaveTextContent(hostile.expected.importDone);
  expect(document.querySelector('img, script:not([src])')).toBeNull();
});

test('a file picked while something else was opened meanwhile is let go', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  core.importFile = workFile;
  let pick!: (name: string) => void;
  vi.spyOn(core, 'pickImportFile').mockImplementation(async () => {
    core.pickedImport = workFile;
    return new Promise<string>((resolve) => (pick = resolve));
  });
  const user = await showApp(core);
  await chooseImport(user);

  await user.keyboard('{Control>},{/Control}');
  pick('work.ics');

  await waitFor(() => expect(core.pickedImport).toBeUndefined());
  expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();
  expect(screen.queryByRole('dialog', { name: /Import/ })).not.toBeInTheDocument();
});

test('a Calendar that becomes Read-only while the dialog is open is no longer chosen', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const work = core.addCalendar({ name: 'Work' });
  const user = await showApp(core);
  const dialog = await openImport(user, core);
  const calendar = within(dialog).getByRole('combobox', { name: 'Into calendar' });
  expect(calendar).toHaveDisplayValue('Home');

  core.calendars.find((found) => found.id === home.id)!.readOnly = true;
  core.emit({ kind: 'calendarsChanged' });

  await waitFor(() => expect(calendar).toHaveDisplayValue('Work'));
  await user.click(await within(dialog).findByRole('button', { name: 'Import' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.events.every((event) => event.calendarId === work.id)).toBe(true);
});
