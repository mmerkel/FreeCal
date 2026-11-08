// The import flow: picking a file, the preview, running and undoing an
// import, run against the fake core with a stand-in grid (see
// StubGrid.svelte). What an import does to the Events is the core's, tested
// at the core interface; these tests check the calls and what the window shows.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import App from './App.svelte';
import type { Calendar, ImportSummary } from './core/types';
import { calendar, event, firstLaunch, scriptWindow } from './test/answers';
import { journalTransactionId } from './test/ids';
import { failure, type FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

/** A first launch with `calendars`. */
function withCalendars(...calendars: Calendar[]) {
  const core = firstLaunch();
  core.answer('listCalendars', calendars);
  return core;
}

/** The preview of a file with `newEvents` Events, none of them in `into` yet. */
function preview(into: Calendar, fields: Partial<ImportSummary> = {}): ImportSummary {
  return { calendarId: into.id, newEvents: 2, existingEvents: 0, pinned: [], ...fields };
}

async function showApp(core: FakeCore) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
  return userEvent.setup();
}

async function chooseImport(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Import…' }));
}

/** Lets the user pick `file`, opens the import dialog for it and waits for `summary`. */
async function openImport(
  user: ReturnType<typeof userEvent.setup>,
  core: FakeCore,
  summary: ImportSummary,
  file = 'work.ics',
) {
  core.answer('pickImportFile', file);
  core.answer('previewImport', summary);
  await chooseImport(user);
  const dialog = await screen.findByRole('dialog', { name: `Import “⁨${file}⁩”` });
  await within(dialog).findByText(/new event/);
  return dialog;
}

function grid() {
  return screen.getByRole('list', { name: 'Grid' });
}

test('Import… asks for a file in the system’s file dialog, with translated words', async () => {
  const core = withCalendars(calendar({ name: 'Home' }));
  const user = await showApp(core);

  await chooseImport(user);

  expect(core.callsOf('pickImportFile')).toEqual([
    ['Import events', 'Calendar files (.ics, .zip)'],
  ]);
  // The user cancelled the file dialog: nothing else happens.
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.callsOf('previewImport')).toEqual([]);
});

test('the import dialog offers only writable Calendars and previews the first shown one', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(
    calendar({ name: 'Hidden', shown: false }),
    calendar({ name: 'Holidays', readOnly: true }),
    home,
  );
  const user = await showApp(core);

  const dialog = await openImport(user, core, preview(home));

  const choice = within(dialog).getByRole('combobox', { name: 'Into calendar' });
  expect(
    within(choice)
      .getAllByRole('option')
      .map((option) => option.textContent),
  ).toEqual(['Hidden', 'Home']);
  expect(choice).toHaveDisplayValue('Home');
  expect(core.callsOf('previewImport')).toEqual([[home.id]]);
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();
  // Nothing exists yet, so there is nothing to choose.
  expect(within(dialog).queryByRole('radio')).not.toBeInTheDocument();
});

test('with Events already in the Calendar, the dialog asks once what to do with them', async () => {
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work' });
  const core = withCalendars(home, work);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();

  core.answer('previewImport', preview(work, { newEvents: 1, existingEvents: 1 }));
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
  expect(core.callsOf('runImport')).toEqual([[work.id, 'skip']]);
});

test('the preview names the zones floating times were pinned to', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);

  const dialog = await openImport(
    user,
    core,
    preview(home, {
      pinned: [
        { zone: 'Europe/Berlin', events: 2 },
        { zone: 'UTC', events: 1 },
      ],
    }),
  );

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
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  core.answer('pickImportFile', 'empty.ics');
  core.answer('previewImport', preview(home, { newEvents: 0 }));
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Import “⁨empty.ics⁩”' });
  expect(await within(dialog).findByText('The file holds no events.')).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Import' })).toBeDisabled();
});

test('the import runs under the modal “Working…” dialog, and the grid follows “Occurrences changed”', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));
  const ran = core.pending('runImport');

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const working = await screen.findByRole('dialog', { name: 'Working…' });
  expect(working).toHaveAttribute('open');
  expect(within(working).getByText('Importing “⁨work.ics⁩”…')).toBeInTheDocument();
  expect(within(working).queryByRole('button')).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  expect(screen.getByRole('dialog', { name: 'Working…' })).toBeInTheDocument();

  ran.resolve();

  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.callsOf('runImport')).toEqual([[home.id, 'update']]);
  expect(core.callsOf('discardImport')).toEqual([]);
  const standup = event({
    calendarId: home.id,
    title: 'Standup',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
  });
  scriptWindow(core, { calendars: [home], events: [standup] });
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });
  expect(await within(grid()).findByRole('button', { name: 'Standup' })).toBeInTheDocument();
});

test('after an import, Undo undoes its Journal Transaction under “Working…”', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home, { newEvents: 1, existingEvents: 1 }));
  core.answer('runImport', {
    journalTransaction: journalTransactionId(7),
    imported: 1,
    updated: 1,
    skipped: 0,
  });

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const toast = await screen.findByRole('status');
  expect(toast).toHaveTextContent('Imported “⁨work.ics⁩”: 1 added, 1 updated, 0 skipped.');
  const undone = core.pending('undoJournalTransaction');

  await user.click(within(toast).getByRole('button', { name: 'Undo' }));

  const working = await screen.findByRole('dialog', { name: 'Working…' });
  expect(within(working).getByText('Undoing the import of “⁨work.ics⁩”…')).toBeInTheDocument();
  expect(core.callsOf('undoJournalTransaction')).toEqual([[journalTransactionId(7)]]);
  undone.resolve();
  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent('The import of “⁨work.ics⁩” was undone.'),
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(within(screen.getByRole('status')).queryByRole('button', { name: 'Undo' })).toBeNull();

  await user.click(within(screen.getByRole('status')).getByRole('button', { name: 'Dismiss' }));
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});

test('an import that changed nothing offers no Undo', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  const dialog = await openImport(
    user,
    core,
    preview(home, { newEvents: 0, existingEvents: 1 }),
    'one.ics',
  );
  core.answer('runImport', { journalTransaction: null, imported: 0, updated: 0, skipped: 1 });

  await user.click(within(dialog).getByRole('radio', { name: 'Skip them' }));
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  const toast = await screen.findByRole('status');
  expect(toast).toHaveTextContent('Imported “⁨one.ics⁩”: 0 added, 0 updated, 1 skipped.');
  expect(within(toast).queryByRole('button', { name: 'Undo' })).not.toBeInTheDocument();
});

test('a failed import stays in the dialog and says why', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));
  core.answer('runImport', failure({ code: 'calendarReadOnly' }));

  await user.click(within(dialog).getByRole('button', { name: 'Import' }));

  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The import failed: the calendar is read-only',
  );
  expect(screen.queryByRole('dialog', { name: 'Working…' })).not.toBeInTheDocument();
  expect(core.callsOf('discardImport')).toEqual([]);
});

test('Cancel lets go of the picked file', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));

  await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(core.callsOf('discardImport')).toEqual([[]]);
  expect(core.callsOf('runImport')).toEqual([]);
});

test('a file that can’t be read gives a message naming it', async () => {
  const core = withCalendars(calendar({ name: 'Home' }));
  core.answer('pickImportFile', failure({ code: 'importTooLarge', file: 'takeout.zip' }));
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Can’t import this file' });
  expect(dialog).toHaveTextContent(
    'The file couldn’t be opened: “⁨takeout.zip⁩” is too large to import',
  );
});

test('a preview that fails says why and can’t be imported', async () => {
  const core = withCalendars(calendar({ name: 'Home' }));
  core.answer('pickImportFile', 'work.ics');
  core.answer('previewImport', failure({ code: 'importNotCalendar', file: 'work.ics' }));
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = await screen.findByRole('dialog', { name: 'Import “⁨work.ics⁩”' });
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    'The file couldn’t be read: “⁨work.ics⁩” isn’t a calendar file FreeCal can read',
  );
  expect(within(dialog).getByRole('button', { name: 'Import' })).toBeDisabled();
});

test('without a writable Calendar, Import… says where to create one first', async () => {
  const core = withCalendars(calendar({ name: 'Holidays', readOnly: true }));
  const user = await showApp(core);

  await chooseImport(user);

  const dialog = screen.getByRole('dialog', { name: 'No calendar to import into' });
  expect(dialog).toHaveTextContent('Create a calendar first with the + next to “Local”');
  expect(core.callsOf('pickImportFile')).toEqual([]);
});

test('a preview for a Calendar chosen earlier never overwrites the latest one', async () => {
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work' });
  const core = withCalendars(home, work);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));
  const slow = core.pending('previewImport');
  const choice = within(dialog).getByRole('combobox', { name: 'Into calendar' });

  await user.selectOptions(choice, 'Work');
  await user.selectOptions(choice, 'Home');
  await within(dialog).findByText('2 new events');
  slow.resolve(preview(work, { newEvents: 1, existingEvents: 1 }));

  await new Promise((resolve) => setTimeout(resolve));
  expect(within(dialog).getByText('2 new events')).toBeInTheDocument();
  expect(within(dialog).queryByRole('radio')).not.toBeInTheDocument();
});

test('the hostile file name and zone are shown only as text', async () => {
  const home = calendar({ name: 'Home' });
  const core = withCalendars(home);
  const user = await showApp(core);
  core.answer('runImport', {
    journalTransaction: journalTransactionId(7),
    imported: 1,
    updated: 0,
    skipped: 0,
  });

  const dialog = await openImport(
    user,
    core,
    preview(home, { newEvents: 1, pinned: [{ zone: hostile.pinnedZone, events: 1 }] }),
    hostile.fileName,
  );

  expect(within(dialog).getByRole('heading')).toHaveTextContent(hostile.expected.importTitle);
  expect(within(dialog).getByText(hostile.expected.importPinned)).toBeInTheDocument();
  await user.click(within(dialog).getByRole('button', { name: 'Import' }));
  expect(await screen.findByRole('status')).toHaveTextContent(hostile.expected.importDone);
  expect(document.querySelector('img, script:not([src])')).toBeNull();
});

test('a Calendar that becomes Read-only while the dialog is open is no longer chosen', async () => {
  const home = calendar({ name: 'Home' });
  const work = calendar({ name: 'Work' });
  const core = withCalendars(home, work);
  const user = await showApp(core);
  const dialog = await openImport(user, core, preview(home));
  const choice = within(dialog).getByRole('combobox', { name: 'Into calendar' });
  expect(choice).toHaveDisplayValue('Home');

  core.answer('listCalendars', [{ ...home, readOnly: true }, work]);
  core.answer('previewImport', preview(work));
  core.emit({ kind: 'calendarsChanged' });

  await waitFor(() => expect(choice).toHaveDisplayValue('Work'));
  await user.click(await within(dialog).findByRole('button', { name: 'Import' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(core.callsOf('runImport')).toEqual([[work.id, 'update']]);
});
