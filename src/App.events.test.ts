// The App's Event flows: creating, opening, editing and deleting Events, run
// against the fake core with a stand-in grid (see StubGrid.svelte).
// What a change does to the Events is the core's, tested at the core
// interface; these tests check the calls made, and that the window follows
// the "Occurrences changed" they emit as the core would.
import { screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import type { Event, Repeat, StructuredText, When } from './core/types';
import {
  caldavAccount,
  calendar,
  event,
  firstLaunch,
  occurrence,
  occurrencesChanged,
} from './test/answers';
import { journalTransactionId } from './test/ids';
import { failure, type FakeCore } from './test/fakeCore';
import { stubGrid } from './test/stubGrid';
import { showApp } from './test/showApp';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

afterEach(() => vi.useRealTimers());

const monday: When = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' };
const later: When = { kind: 'timed', start: '2026-10-06T14:00:00', end: '2026-10-06T15:30:00' };
const home = calendar({ name: 'Home' });
const locked = calendar({ name: 'Holidays', readOnly: true });

/** The user selects a slot in the grid, as FullCalendar reports it. */
async function selectSlot(when: When) {
  stubGrid.props!.onselectslot!(when);
  await tick();
}

/** Opens the Occurrence titled `title`, whose Event the core reads as `opened`. */
async function openOccurrence(
  user: ReturnType<typeof userEvent.setup>,
  core: FakeCore,
  opened: Event,
  title = opened.title,
) {
  core.answer('event', opened);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await user.click(await within(grid).findByRole('button', { name: title }));
  return screen.findByRole('dialog', { name: title });
}

test('selecting a slot opens the editor with that time, and saving creates the Event', async () => {
  const user = userEvent.setup();
  const core = firstLaunch({ calendars: [home] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-05');
  expect(within(editor).getByLabelText('Start time')).toHaveValue('09:00');
  expect(within(editor).getByLabelText('End time')).toHaveValue('10:30');
  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), 'Dentist{Enter}');

  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(core.callsOf('createEvent')).toEqual([
    [expect.objectContaining({ calendarId: home.id, title: 'Dentist', when: monday })],
  ]);
  occurrencesChanged(
    core,
    [home],
    [event({ calendarId: home.id, title: 'Dentist', when: monday })],
  );
  expect(await within(grid).findByRole('button', { name: 'Dentist' })).toBeInTheDocument();
});

test('a selection of whole days creates an all-day Event', async () => {
  const user = userEvent.setup();
  const core = firstLaunch({ calendars: [home] });
  await showApp(core);

  await selectSlot({ kind: 'allDay', start: '2026-10-07', end: '2026-10-09' });
  expect(screen.getByRole('checkbox', { name: 'All day' })).toBeChecked();
  expect(screen.getByLabelText('End date')).toHaveValue('2026-10-08');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(core.callsOf('createEvent')).toEqual([
      [
        expect.objectContaining({
          when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-09' },
        }),
      ],
    ]),
  );
});

test('the editor offers only writable Calendars, the first shown one first', async () => {
  const core = firstLaunch({
    calendars: [calendar({ name: 'Hidden', shown: false }), locked, home],
  });
  await showApp(core);

  await selectSlot(monday);

  const choice = screen.getByRole('combobox', { name: 'Calendar' });
  expect(
    within(choice)
      .getAllByRole('option')
      .map((option) => option.textContent),
  ).toEqual(['Hidden', 'Home']);
  expect(choice).toHaveDisplayValue('Home');
});

test('without a writable Calendar, creating points to the + next to the Local Account', async () => {
  const user = userEvent.setup();
  await showApp(firstLaunch({ calendars: [locked] }));

  await selectSlot(monday);
  const hint = screen.getByRole('dialog', { name: 'No calendar for new events' });
  expect(hint).toHaveTextContent(
    'Create a calendar first with the + next to “Local” in the sidebar.',
  );
  await user.click(within(hint).getByRole('button', { name: 'OK' }));
  await user.keyboard('c');

  expect(screen.getByRole('dialog', { name: 'No calendar for new events' })).toBeInTheDocument();
  expect(screen.queryByRole('dialog', { name: 'New event' })).toBeNull();
});

test('c creates an Event at the next full hour', async () => {
  vi.useFakeTimers({ now: new Date(2026, 9, 3, 14, 20), shouldAdvanceTime: true });
  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
  await showApp(firstLaunch({ calendars: [home] }));

  await user.keyboard('c');

  const editor = screen.getByRole('dialog', { name: 'New event' });
  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-03');
  expect(within(editor).getByLabelText('Start time')).toHaveValue('15:00');
  expect(within(editor).getByLabelText('End time')).toHaveValue('16:00');
  expect(within(editor).getByRole('textbox', { name: 'Title' })).toHaveValue('');
});

test('c does nothing while typing or with a modifier', async () => {
  const user = userEvent.setup();
  await showApp(firstLaunch({ calendars: [home] }));

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'calendar');
  await user.keyboard('{Escape}');
  await user.keyboard('{Control>}c{/Control}');

  expect(screen.queryByRole('dialog')).toBeNull();
});

test('an Event’s details show its title, time, location, description and Calendar', async () => {
  const user = userEvent.setup();
  const dentist = event({
    calendarId: home.id,
    title: 'Dentist',
    when: monday,
    location: 'Main Street 1',
    description: 'Bring the card',
  });
  const core = firstLaunch({ calendars: [home], events: [dentist] });
  await showApp(core);

  const details = await openOccurrence(user, core, dentist);

  expect(core.callsOf('event')).toEqual([[dentist.id]]);
  expect(details).toHaveTextContent('Monday, October 5, 2026');
  expect(details).toHaveTextContent('9:00');
  expect(details).toHaveTextContent('10:30');
  expect(details).toHaveTextContent('Main Street 1');
  expect(details).toHaveTextContent('Bring the card');
  expect(details).toHaveTextContent('Home');
  expect(
    within(screen.getByRole('list', { name: 'Grid' })).getByRole('button', { name: 'Dentist' }),
  ).toHaveAttribute('aria-current', 'true');
});

test('the details of an Event that had no time zone say which zone FreeCal gave it', async () => {
  const user = userEvent.setup();
  const floating = event({
    calendarId: home.id,
    title: 'Floating',
    when: monday,
    pinnedZone: 'Europe/Berlin',
  });
  const zoned = event({ calendarId: home.id, title: 'Zoned', when: monday });
  const core = firstLaunch({ calendars: [home], events: [floating, zoned] });
  await showApp(core);

  const floatingDetails = await openOccurrence(user, core, floating);
  expect(floatingDetails).toHaveTextContent(
    'The original has no time zone, so FreeCal uses Europe/Berlin.',
  );
  await user.click(within(floatingDetails).getByRole('button', { name: 'Close' }));

  const zonedDetails = await openOccurrence(user, core, zoned);
  expect(zonedDetails).not.toHaveTextContent('no time zone');
});

test('the details of an Event with an Unsent Change say it isn’t synced yet, and why', async () => {
  const user = userEvent.setup();
  const account = caldavAccount('ada');
  const team = calendar({ accountId: account.id, name: 'Team' });
  const waiting = event({
    calendarId: team.id,
    title: 'Waiting',
    when: monday,
    unsentChange: { since: '2026-10-03T10:00:00Z', sendFailure: { kind: 'serverChanged' } },
  });
  const sent = event({ calendarId: team.id, title: 'Sent', when: later });
  const core = firstLaunch({ accounts: [account], calendars: [team], events: [waiting, sent] });
  await showApp(core);

  const details = await openOccurrence(user, core, waiting);
  expect(details).toHaveTextContent('Not yet synced: the event was changed on the server');

  // The Account's reason comes first, and the details follow it.
  const offline = { status: { kind: 'offline' }, lastSync: null, unsentChanges: 1 } as const;
  core.answer('syncStatuses', {
    overall: { state: { kind: 'offline' }, lastSync: null },
    accounts: [{ accountId: account.id, report: offline }],
    calendars: [],
  });
  core.emit({ kind: 'accountSyncStatusChanged', accountId: account.id, report: offline });
  await waitFor(() => expect(details).toHaveTextContent('Not yet synced: offline'));
  await user.click(within(details).getByRole('button', { name: 'Close' }));

  expect(await openOccurrence(user, core, sent)).not.toHaveTextContent('Not yet synced');
});

test('an Event of a Read-only Calendar can’t be edited or deleted', async () => {
  const user = userEvent.setup();
  const fixed = event({ calendarId: locked.id, title: 'Fixed', when: monday });
  const core = firstLaunch({ calendars: [locked], events: [fixed] });
  await showApp(core);

  const details = await openOccurrence(user, core, fixed);

  expect(
    within(details)
      .getAllByRole('button')
      .map((button) => button.getAttribute('aria-label')),
  ).toEqual(['Close']);
  expect(within(details).getByText('Can’t be changed: its calendar is read-only.')).toBeVisible();
});

test('an Event with guests can’t be edited or deleted, and says where to edit it', async () => {
  const user = userEvent.setup();
  const meeting = event({ calendarId: home.id, title: 'Meeting', when: monday, hasGuests: true });
  const core = firstLaunch({ calendars: [home], events: [meeting] });
  await showApp(core);

  const details = await openOccurrence(user, core, meeting);

  expect(
    within(details)
      .getAllByRole('button')
      .map((button) => button.getAttribute('aria-label')),
  ).toEqual(['Close']);
  expect(
    within(details).getByText(
      'Has guests, so it can’t be changed in FreeCal. Edit it in Google or on its server for now.',
    ),
  ).toBeVisible();
  expect(within(details).queryByText(/calendar is read-only/)).toBeNull();
});

test('the details offer Edit, Delete and Close as icons with tooltips, ✕ focused', async () => {
  const user = userEvent.setup();
  const dentist = event({ calendarId: home.id, title: 'Dentist', when: monday });
  const core = firstLaunch({ calendars: [home], events: [dentist] });
  await showApp(core);

  const details = await openOccurrence(user, core, dentist);

  const buttons = within(details).getAllByRole('button');
  expect(buttons.map((button) => [button.getAttribute('aria-label'), button.title])).toEqual([
    ['Edit', 'Edit'],
    ['Delete', 'Delete'],
    ['Close', 'Close'],
  ]);
  expect(buttons.every((button) => button.querySelector('svg'))).toBe(true);
  expect(within(details).getByRole('button', { name: 'Close' })).toHaveFocus();
  expect(within(details).queryByText(/read-only/)).toBeNull();
});

test('✕ and a click on the backdrop close the details', async () => {
  const user = userEvent.setup();
  const dentist = event({ calendarId: home.id, title: 'Dentist', when: monday });
  const core = firstLaunch({ calendars: [home], events: [dentist] });
  await showApp(core);

  let details = await openOccurrence(user, core, dentist);
  await user.click(within(details).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).toBeNull();

  details = await openOccurrence(user, core, dentist);
  await user.click(within(details).getByText('Home'));
  expect(details).toBeInTheDocument();
  // The backdrop belongs to the dialog element itself; its content doesn't.
  await user.click(details);
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('a click on the editor’s backdrop doesn’t close it', async () => {
  const user = userEvent.setup();
  await showApp(firstLaunch({ calendars: [home] }));

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  await user.click(editor);

  expect(editor).toBeInTheDocument();
});

test('editing an Event changes it and returns to its details', async () => {
  const user = userEvent.setup();
  const lunch = event({ calendarId: home.id, title: 'Lunch', when: monday });
  const longLunch = { ...lunch, title: 'Long lunch' };
  const core = firstLaunch({ calendars: [home], events: [lunch] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  const details = await openOccurrence(user, core, lunch);
  core.answer('editEvent', { journalTransaction: journalTransactionId(7), value: longLunch });

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  const editor = screen.getByRole('dialog', { name: 'Edit event' });
  const title = within(editor).getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'Long lunch{Enter}');
  core.answer('event', longLunch);
  occurrencesChanged(core, [home], [longLunch]);

  expect(await screen.findByRole('dialog', { name: 'Long lunch' })).toBeInTheDocument();
  expect(core.callsOf('editEvent')).toEqual([
    [lunch.id, expect.objectContaining({ title: 'Long lunch', when: monday })],
  ]);
  expect(await within(grid).findByRole('button', { name: 'Long lunch' })).toBeInTheDocument();
});

test('cancelling an edit returns to the unchanged details', async () => {
  const user = userEvent.setup();
  const lunch = event({ calendarId: home.id, title: 'Lunch', when: monday });
  const core = firstLaunch({ calendars: [home], events: [lunch] });
  await showApp(core);
  const details = await openOccurrence(user, core, lunch);

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  await user.type(screen.getByRole('textbox', { name: 'Title' }), ' changed');
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Discard' }));

  expect(await screen.findByRole('dialog', { name: 'Lunch' })).toBeInTheDocument();
  expect(core.callsOf('editEvent')).toEqual([]);
});

test('deleting an Event asks first, then deletes it through the core', async () => {
  const user = userEvent.setup();
  const doomed = event({ calendarId: home.id, title: 'Doomed', when: monday });
  const kept = event({ calendarId: home.id, title: 'Kept', when: monday });
  const core = firstLaunch({ calendars: [home], events: [doomed, kept] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  const details = await openOccurrence(user, core, doomed);

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(ask).toHaveTextContent('“⁨Doomed⁩” will be deleted.');
  expect(within(ask).queryByRole('button', { name: 'Close' })).toBeNull();
  expect(core.callsOf('deleteEvent')).toEqual([]);
  await user.click(within(ask).getByRole('button', { name: 'Delete' }));

  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(core.callsOf('deleteEvent')).toEqual([[doomed.id]]);
  occurrencesChanged(core, [home], [kept]);
  await waitFor(() => expect(within(grid).queryByRole('button', { name: 'Doomed' })).toBeNull());
  expect(within(grid).getByRole('button', { name: 'Kept' })).toBeInTheDocument();
});

test('Cancel and Escape in the delete question keep the Event', async () => {
  const user = userEvent.setup();
  const kept = event({ calendarId: home.id, title: 'Kept', when: monday });
  const core = firstLaunch({ calendars: [home], events: [kept] });
  await showApp(core);
  const details = await openOccurrence(user, core, kept);

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(within(ask).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alertdialog')).toBeNull();
  expect(screen.getByRole('dialog', { name: 'Kept' })).toBeInTheDocument();
  expect(core.callsOf('deleteEvent')).toEqual([]);
});

test('“Occurrences changed” re-fetches the visible range and keeps the open Event', async () => {
  const user = userEvent.setup();
  const standup = event({ calendarId: home.id, title: 'Standup', when: monday });
  const moved = { ...standup, title: 'Standup (moved)' };
  const core = firstLaunch({ calendars: [home], events: [standup] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await openOccurrence(user, core, standup);

  // A change made elsewhere, such as a sync, that the core signals.
  core.answer('event', moved);
  occurrencesChanged(
    core,
    [home],
    [moved, event({ calendarId: home.id, title: 'Retro', when: monday })],
  );

  expect(await within(grid).findByRole('button', { name: 'Retro' })).toBeInTheDocument();
  expect(await screen.findByRole('dialog', { name: 'Standup (moved)' })).toBeInTheDocument();
  expect(within(grid).getByRole('button', { name: 'Standup (moved)' })).toHaveAttribute(
    'aria-current',
    'true',
  );
});

test('the open details close when their Event is deleted elsewhere', async () => {
  const user = userEvent.setup();
  const gone = event({ calendarId: home.id, title: 'Gone', when: monday });
  const core = firstLaunch({ calendars: [home], events: [gone] });
  await showApp(core);
  await openOccurrence(user, core, gone);

  core.answer('event', failure({ code: 'eventNotFound' }));
  occurrencesChanged(core, [home], []);

  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('an open editor’s draft survives a re-fetch', async () => {
  const user = userEvent.setup();
  const plan = event({ calendarId: home.id, title: 'Plan', when: monday });
  const theirs = { ...plan, title: 'Their plan' };
  const core = firstLaunch({ calendars: [home], events: [plan] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await user.click(
    within(await openOccurrence(user, core, plan)).getByRole('button', { name: 'Edit' }),
  );
  const title = screen.getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'My plan');

  core.answer('event', theirs);
  occurrencesChanged(core, [home], [theirs]);
  await within(grid).findByRole('button', { name: 'Their plan' });

  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('My plan');
  await user.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() =>
    expect(core.callsOf('editEvent')).toEqual([
      [plan.id, expect.objectContaining({ title: 'My plan' })],
    ]),
  );
});

test('hiding a Calendar asks the core, and the grid follows “Calendars changed”', async () => {
  const user = userEvent.setup();
  const work = calendar({ name: 'Work' });
  const atHome = event({ calendarId: home.id, title: 'At home', when: monday });
  const atWork = event({ calendarId: work.id, title: 'At work', when: monday });
  const core = firstLaunch({ calendars: [home, work], events: [atHome, atWork] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await within(grid).findByRole('button', { name: 'At work' });

  await user.click(screen.getByRole('switch', { name: 'Work' }));
  expect(core.callsOf('setCalendarShown')).toEqual([[work.id, false]]);
  core.answer('listCalendars', [home, { ...work, shown: false }]);
  core.answer('listOccurrences', [occurrence(atHome)]);
  core.emit({ kind: 'calendarsChanged' });

  await waitFor(() => expect(within(grid).queryByRole('button', { name: 'At work' })).toBeNull());
  expect(within(grid).getByRole('button', { name: 'At home' })).toBeInTheDocument();
});

test('a link whose text is its URL opens straight away', async () => {
  const user = userEvent.setup();
  const url = 'https://example.org/plain';
  const linked = event({
    calendarId: home.id,
    title: 'Linked',
    when: monday,
    description: { paragraphs: [[{ kind: 'link', text: url, url }]], text: url },
  });
  const core = firstLaunch({ calendars: [home], events: [linked] });
  await showApp(core);
  const details = await openOccurrence(user, core, linked);

  await user.click(within(details).getByRole('button', { name: url }));

  expect(core.callsOf('openLink')).toEqual([[url]]);
});

test('a link in the location opens like one in the description', async () => {
  const user = userEvent.setup();
  const url = 'https://zoom.us/j/123';
  const call = event({
    calendarId: home.id,
    title: 'Call',
    when: monday,
    location: {
      paragraphs: [
        [
          { kind: 'text', text: 'Zoom: ' },
          { kind: 'link', text: url, url },
        ],
      ],
      text: `Zoom: ${url}`,
    },
  });
  const core = firstLaunch({ calendars: [home], events: [call] });
  await showApp(core);
  const details = await openOccurrence(user, core, call);

  expect(details).toHaveTextContent(`Zoom: ${url}`);
  await user.click(within(details).getByRole('button', { name: url }));

  expect(core.callsOf('openLink')).toEqual([[url]]);
});

test('a link whose text differs from its URL asks first', async () => {
  const user = userEvent.setup();
  const url = 'https://example.com/agenda';
  const linked = event({
    calendarId: home.id,
    title: 'Linked',
    when: monday,
    description: { paragraphs: [[{ kind: 'link', text: 'the agenda', url }]], text: '' },
  });
  const core = firstLaunch({ calendars: [home], events: [linked] });
  await showApp(core);
  const details = await openOccurrence(user, core, linked);

  await user.click(within(details).getByRole('button', { name: 'the agenda' }));
  const ask = screen.getByRole('alertdialog', { name: 'Open this link?' });
  expect(ask).toHaveTextContent(`Open ${url} in your browser?`);
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  expect(core.callsOf('openLink')).toEqual([]);
  expect(screen.getByRole('dialog', { name: 'Linked' })).toBeInTheDocument();

  await user.click(within(details).getByRole('button', { name: 'the agenda' }));
  await user.click(screen.getByRole('button', { name: 'Open' }));
  expect(core.callsOf('openLink')).toEqual([[url]]);
});

test('the hostile Event goes through the grid, its details and the editor as text', async () => {
  const user = userEvent.setup();
  const hostileHtml = event({
    calendarId: home.id,
    title: 'Hostile',
    when: monday,
    location: hostile.expected.location as StructuredText,
    description: hostile.expected.html as StructuredText,
    pinnedZone: hostile.pinnedZone,
  });
  const hostileTitle = event({
    calendarId: home.id,
    title: hostile.title,
    when: monday,
    description: hostile.expected.plainText as StructuredText,
  });
  const core = firstLaunch({ calendars: [home], events: [hostileHtml, hostileTitle] });
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await within(grid).findByRole('button', { name: /onerror/ });
  const details = await openOccurrence(user, core, hostileHtml);

  expect(details).toHaveTextContent(hostile.location);
  expect(details).toHaveTextContent(hostile.expected.pinnedZoneNote);
  expect(details).toHaveTextContent('click me');
  // Disallowed links are plain text; only the core's link pieces are links.
  for (const text of ['click me', 'passwords', 'data:text/html', 'javascript:']) {
    expect(within(details).queryByRole('button', { name: new RegExp(text) })).toBeNull();
  }
  expect(
    within(details).getByRole('button', { name: 'https://meet.example.com/abc' }),
  ).toBeInTheDocument();
  expect(within(details).getByRole('button', { name: 'the agenda' })).toHaveAttribute(
    'title',
    'https://example.com/agenda',
  );
  expect(document.querySelector('img, script, b, i, p > p')).toBeNull();

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue(
    hostile.expected.html.text,
  );
  expect(screen.getByRole('textbox', { name: 'Location' })).toHaveValue(hostile.location);
  await user.click(screen.getByRole('button', { name: 'Save' }));
  // Untouched, the fields go back to the core as they came.
  await screen.findByRole('dialog', { name: 'Hostile' });
  expect(core.callsOf('editEvent')).toEqual([
    [
      hostileHtml.id,
      expect.objectContaining({
        title: 'Hostile',
        location: hostile.location,
        description: hostile.expected.html.text,
      }),
    ],
  ]);

  await user.click(screen.getByRole('button', { name: 'Close' }));
  await openOccurrence(user, core, hostileTitle, hostile.title.replace(/\r\n/g, ' '));
  expect(document.querySelector('img, script, b, i')).toBeNull();
});

test('dragging an Event to another time reschedules it through the core', async () => {
  const dentist = event({ calendarId: home.id, title: 'Dentist', when: monday });
  const core = firstLaunch({ calendars: [home], events: [dentist] });
  await showApp(core);

  await stubGrid.props!.onreschedule!(dentist.id, null, later);

  expect(core.callsOf('rescheduleEvent')).toEqual([[dentist.id, later]]);
  occurrencesChanged(core, [home], [{ ...dentist, when: later }]);
  await waitFor(() => expect(stubGrid.props!.occurrences[0]?.when).toEqual(later));
});

test('a refused drag says why and tells the grid to put the Event back', async () => {
  const dentist = event({ calendarId: home.id, title: 'Dentist', when: monday });
  const core = firstLaunch({ calendars: [home], events: [dentist] });
  await showApp(core);
  core.answer('rescheduleEvent', failure({ code: 'localStoreFailed' }));

  await expect(stubGrid.props!.onreschedule!(dentist.id, null, later)).rejects.toBeDefined();

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The event couldn’t be moved: FreeCal’s database couldn’t be read or written',
  );
});

test('trying to drag an Event of a Read-only Calendar says why it can’t move', async () => {
  const fixed = event({ calendarId: locked.id, title: 'Fixed', when: monday });
  const core = firstLaunch({ calendars: [locked], events: [fixed] });
  await showApp(core);

  stubGrid.props!.onrefusedrag!(fixed.id, null);

  expect(await screen.findByRole('status')).toHaveTextContent(
    '“⁨Fixed⁩” can’t be changed: its calendar “⁨Holidays⁩” is read-only.',
  );
  expect(core.callsOf('rescheduleEvent')).toEqual([]);
});

test('trying to drag an Event with guests says why it can’t move', async () => {
  const meeting = event({ calendarId: home.id, title: 'Meeting', when: monday, hasGuests: true });
  const core = firstLaunch({ calendars: [home], events: [meeting] });
  await showApp(core);

  stubGrid.props!.onrefusedrag!(meeting.id, null);

  expect(await screen.findByRole('status')).toHaveTextContent(
    '“⁨Meeting⁩” has guests, so it can’t be changed in FreeCal. Edit it in Google or on its server for now.',
  );
  expect(core.callsOf('rescheduleEvent')).toEqual([]);
});

test('a refused drag of an Event with guests names a hostile Event as text', async () => {
  const fixed = event({ calendarId: home.id, title: hostile.title, when: monday, hasGuests: true });
  await showApp(firstLaunch({ calendars: [home], events: [fixed] }));

  stubGrid.props!.onrefusedrag!(fixed.id, null);

  const toast = await screen.findByRole('status');
  expect(toast.querySelector('p')!.textContent).toBe(hostile.expected.hasGuestsDrag);
  expect(document.querySelector('img, script')).toBeNull();
});

test('a refused drag names a hostile Event and Calendar as text', async () => {
  const hostileCalendar = calendar({ name: hostile.calendarName, readOnly: true });
  const fixed = event({ calendarId: hostileCalendar.id, title: hostile.title, when: monday });
  await showApp(firstLaunch({ calendars: [hostileCalendar], events: [fixed] }));

  stubGrid.props!.onrefusedrag!(fixed.id, null);

  const toast = await screen.findByRole('status');
  expect(toast.querySelector('p')!.textContent).toBe(hostile.expected.readOnlyDrag);
  expect(document.querySelector('img, script')).toBeNull();
});

test('the delete question names a hostile Event as text', async () => {
  const user = userEvent.setup();
  const hostileTitle = event({ calendarId: home.id, title: hostile.title, when: monday });
  const core = firstLaunch({ calendars: [home], events: [hostileTitle] });
  await showApp(core);
  const details = await openOccurrence(
    user,
    core,
    hostileTitle,
    hostile.title.replace(/\r\n/g, ' '),
  );

  await user.click(within(details).getByRole('button', { name: 'Delete' }));

  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(ask.querySelector('.body p')!.textContent).toBe(hostile.expected.deleteQuestion);
  expect(document.querySelector('img, script')).toBeNull();
});

const weeklyThreeTimes: Repeat = {
  frequency: 'weekly',
  interval: 1,
  end: { kind: 'count', count: 3 },
};
/** Yoga, every Monday three times from October 5. */
const yoga = event({ calendarId: home.id, title: 'Yoga', when: monday, repeat: weeklyThreeTimes });
/** The Occurrences of Yoga, as the core gives them. */
const yogaOccurrences = ['2026-10-05', '2026-10-12', '2026-10-19'].map((date) =>
  occurrence(yoga, {
    when: { kind: 'timed', start: `${date}T09:00:00`, end: `${date}T10:30:00` },
    recurrenceId: `${date}T09:00:00`,
  }),
);

/** The Occurrences the grid shows titled `title`, as its buttons. */
function occurrencesTitled(grid: HTMLElement, title: string) {
  return within(grid).queryAllByRole('button', { name: title });
}

/** Opens the details of Yoga's Occurrence `index`. */
async function openYoga(user: ReturnType<typeof userEvent.setup>, index: number) {
  const core = firstLaunch({ calendars: [home], events: yogaOccurrences });
  core.answer('event', yoga);
  await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(3));
  await user.click(occurrencesTitled(grid, 'Yoga')[index]);
  const details = await screen.findByRole('dialog', { name: 'Yoga' });
  return { core, grid, details };
}

test('an Occurrence of a Recurring Event opens with its own date and the repeat', async () => {
  const user = userEvent.setup();
  const { grid, details } = await openYoga(user, 1);

  expect(details).toHaveTextContent('Monday, October 12, 2026');
  expect(details).toHaveTextContent('Every week, 3 times');
  expect(occurrencesTitled(grid, 'Yoga').map((button) => button.ariaCurrent)).toEqual([
    'false',
    'true',
    'false',
  ]);
});

test('deleting an Occurrence asks, and “This event” deletes only that one', async () => {
  const user = userEvent.setup();
  const { core, grid, details } = await openYoga(user, 1);

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(ask).toHaveTextContent('Delete only this event, or all events it repeats with?');
  await user.click(within(ask).getByRole('button', { name: 'This event' }));

  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(core.callsOf('deleteOccurrence')).toEqual([[yoga.id, '2026-10-12T09:00:00']]);
  expect(core.callsOf('deleteEvent')).toEqual([]);
  occurrencesChanged(core, [home], [yogaOccurrences[0], yogaOccurrences[2]]);
  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(2));
});

test('“All events” deletes the whole Recurring Event, and Cancel keeps it', async () => {
  const user = userEvent.setup();
  const { core, details } = await openYoga(user, 2);

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.click(
    within(screen.getByRole('alertdialog')).getByRole('button', { name: 'All events' }),
  );

  await waitFor(() => expect(core.callsOf('deleteEvent')).toEqual([[yoga.id]]));
  expect(core.callsOf('deleteOccurrence')).toEqual([]);
});

test('a new Event can repeat', async () => {
  const user = userEvent.setup();
  const core = firstLaunch({ calendars: [home] });
  await showApp(core);

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), 'Yoga');
  await user.selectOptions(within(editor).getByRole('combobox', { name: 'Repeat' }), 'Daily');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(core.callsOf('createEvent')).toEqual([
      [
        expect.objectContaining({
          repeat: { frequency: 'daily', interval: 1, end: { kind: 'never' } },
        }),
      ],
    ]),
  );
});

test('editing an Occurrence starts from its date and asks which events to change on Save', async () => {
  const user = userEvent.setup();
  const { core, details } = await openYoga(user, 2);
  const pilates = { ...yogaOccurrences[2], title: 'Pilates' };
  core.answer('editOccurrence', { journalTransaction: journalTransactionId(7), value: pilates });

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  const editor = screen.getByRole('dialog', { name: 'Edit event' });
  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-19');
  expect(within(editor).getByRole('combobox', { name: 'Repeat' })).toHaveDisplayValue('Weekly');
  const title = within(editor).getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'Pilates');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));
  const prompt = await screen.findByRole('dialog', { name: 'Change a recurring event' });
  expect(prompt).toHaveTextContent(
    'Change only this event, this and following events, or all events?',
  );
  await user.click(within(prompt).getByRole('radio', { name: 'This and following events' }));
  core.answer('event', { ...yoga, title: 'Pilates' });
  await user.click(within(prompt).getByRole('button', { name: 'Change' }));

  expect(core.callsOf('editOccurrence')).toEqual([
    [
      yoga.id,
      '2026-10-19T09:00:00',
      'thisAndFollowing',
      false,
      expect.objectContaining({ title: 'Pilates', when: yogaOccurrences[2].when }),
    ],
  ]);
  occurrencesChanged(core, [home], [...yogaOccurrences.slice(0, 2), pilates]);
  expect(await screen.findByRole('dialog', { name: 'Pilates' })).toHaveTextContent(
    'Monday, October 19, 2026',
  );
});

test('dropping an Occurrence asks which events to move, and moves them through the core', async () => {
  const core = firstLaunch({ calendars: [home], events: yogaOccurrences });
  await showApp(core);
  const moved: When = { kind: 'timed', start: '2026-10-13T14:00:00', end: '2026-10-13T15:30:00' };

  const dropped = stubGrid.props!.onreschedule!(yoga.id, '2026-10-12T09:00:00', moved);
  const prompt = await screen.findByRole('dialog', { name: 'Change a recurring event' });
  expect(prompt).toHaveTextContent(
    'Move only this event, this and following events, or all events?',
  );
  within(prompt).getByRole('button', { name: 'Move' }).click();
  await dropped;

  expect(core.callsOf('rescheduleOccurrence')).toEqual([
    [yoga.id, '2026-10-12T09:00:00', 'this', false, moved],
  ]);
  expect(core.callsOf('rescheduleEvent')).toEqual([]);
});

test('an Exception edits without a repeat, and Delete still asks', async () => {
  const user = userEvent.setup();
  const exception = event({
    calendarId: home.id,
    title: 'Moved',
    when: monday,
    recurrenceId: '2026-10-05T08:00:00',
  });
  const core = firstLaunch({ calendars: [home], events: [exception] });
  await showApp(core);
  const details = await openOccurrence(user, core, exception);

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(within(ask).getByRole('button', { name: 'This event' })).toBeInTheDocument();
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  await user.click(within(details).getByRole('button', { name: 'Edit' }));

  expect(screen.queryByRole('combobox', { name: 'Repeat' })).toBeNull();
});
