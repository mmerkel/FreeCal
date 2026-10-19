// The App's Event flows: creating, opening, editing and deleting Events, run
// against the fake core with a stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import App from './App.svelte';
import type { Repeat, StructuredText, When } from './core/types';
import { FakeCore } from './test/fakeCore';
import { stubGrid } from './test/stubGrid';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

afterEach(() => vi.useRealTimers());

const monday: When = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' };

async function showApp(core: FakeCore) {
  render(App, { core });
  return screen.findByRole('list', { name: 'Grid' });
}

/** The user selects a slot in the grid, as FullCalendar reports it. */
async function selectSlot(when: When) {
  stubGrid.props!.onselectslot!(when);
  await tick();
}

async function openOccurrence(user: ReturnType<typeof userEvent.setup>, title: string) {
  const grid = screen.getByRole('list', { name: 'Grid' });
  await user.click(await within(grid).findByRole('button', { name: title }));
  return screen.findByRole('dialog', { name: title });
}

test('selecting a slot opens the editor with that time, and saving creates the Event', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const grid = await showApp(core);

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-05');
  expect(within(editor).getByLabelText('Start time')).toHaveValue('09:00');
  expect(within(editor).getByLabelText('End time')).toHaveValue('10:30');
  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), 'Dentist{Enter}');

  expect(await within(grid).findByRole('button', { name: 'Dentist' })).toBeInTheDocument();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(core.events).toEqual([
    expect.objectContaining({ calendarId: home.id, title: 'Dentist', when: monday }),
  ]);
});

test('a selection of whole days creates an all-day Event', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await selectSlot({ kind: 'allDay', start: '2026-10-07', end: '2026-10-09' });
  expect(screen.getByRole('checkbox', { name: 'All day' })).toBeChecked();
  expect(screen.getByLabelText('End date')).toHaveValue('2026-10-08');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(core.events[0]?.when).toEqual({
      kind: 'allDay',
      start: '2026-10-07',
      end: '2026-10-09',
    }),
  );
});

test('the editor offers only writable Calendars, the first shown one first', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Hidden', shown: false });
  core.addCalendar({ name: 'Locked', readOnly: true });
  core.addCalendar({ name: 'Home' });
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
  const core = new FakeCore();
  core.addCalendar({ name: 'Locked', readOnly: true });
  await showApp(core);

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
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.keyboard('c');

  const editor = screen.getByRole('dialog', { name: 'New event' });
  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-03');
  expect(within(editor).getByLabelText('Start time')).toHaveValue('15:00');
  expect(within(editor).getByLabelText('End time')).toHaveValue('16:00');
  expect(within(editor).getByRole('textbox', { name: 'Title' })).toHaveValue('');
});

test('c does nothing while typing or with a modifier', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.type(screen.getByRole('textbox', { name: 'Name' }), 'calendar');
  await user.keyboard('{Escape}');
  await user.keyboard('{Control>}c{/Control}');

  expect(screen.queryByRole('dialog')).toBeNull();
});

test('an Event’s details show its title, time, location, description and Calendar', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({
    calendarId: home.id,
    title: 'Dentist',
    when: monday,
    location: 'Main Street 1',
    description: 'Bring the card',
  });
  await showApp(core);

  const details = await openOccurrence(user, 'Dentist');

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
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({
    calendarId: home.id,
    title: 'Floating',
    when: monday,
    pinnedZone: 'Europe/Berlin',
  });
  core.addEvent({ calendarId: home.id, title: 'Zoned', when: monday });
  await showApp(core);

  const floating = await openOccurrence(user, 'Floating');
  expect(floating).toHaveTextContent(
    'The original has no time zone, so FreeCal uses Europe/Berlin.',
  );
  await user.click(within(floating).getByRole('button', { name: 'Close' }));

  const zoned = await openOccurrence(user, 'Zoned');
  expect(zoned).not.toHaveTextContent('no time zone');
});

test('an Event of a Read-only Calendar can’t be edited or deleted', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const locked = core.addCalendar({ name: 'Locked', readOnly: true });
  core.addEvent({ calendarId: locked.id, title: 'Fixed', when: monday });
  await showApp(core);

  const details = await openOccurrence(user, 'Fixed');

  expect(
    within(details)
      .getAllByRole('button')
      .map((button) => button.getAttribute('aria-label')),
  ).toEqual(['Close']);
  expect(within(details).getByText('Can’t be changed: its calendar is read-only.')).toBeVisible();
});

test('the details offer Edit, Delete and Close as icons with tooltips, ✕ focused', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Dentist', when: monday });
  await showApp(core);

  const details = await openOccurrence(user, 'Dentist');

  const buttons = within(details).getAllByRole('button');
  expect(buttons.map((button) => [button.getAttribute('aria-label'), button.title])).toEqual([
    ['Edit', 'Edit'],
    ['Delete', 'Delete'],
    ['Close', 'Close'],
  ]);
  expect(buttons.every((button) => button.querySelector('svg'))).toBe(true);
  expect(within(details).getByRole('button', { name: 'Close' })).toHaveFocus();
});

test('✕ and a click on the backdrop close the details', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Dentist', when: monday });
  await showApp(core);

  let details = await openOccurrence(user, 'Dentist');
  await user.click(within(details).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).toBeNull();

  details = await openOccurrence(user, 'Dentist');
  await user.click(within(details).getByText('Home'));
  expect(details).toBeInTheDocument();
  // The backdrop belongs to the dialog element itself; its content doesn't.
  await user.click(details);
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('a click on the editor’s backdrop doesn’t close it', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  await showApp(core);

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  await user.click(editor);

  expect(editor).toBeInTheDocument();
});

test('an Event of a writable Calendar doesn’t say it is read-only', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Dentist', when: monday });
  await showApp(core);

  const details = await openOccurrence(user, 'Dentist');

  expect(within(details).queryByText(/read-only/)).toBeNull();
});

test('editing an Event changes it and returns to its details', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const event = core.addEvent({ calendarId: home.id, title: 'Lunch', when: monday });
  await showApp(core);
  const details = await openOccurrence(user, 'Lunch');

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  const editor = screen.getByRole('dialog', { name: 'Edit event' });
  const title = within(editor).getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'Long lunch{Enter}');

  expect(await screen.findByRole('dialog', { name: 'Long lunch' })).toBeInTheDocument();
  expect(core.events[0]).toMatchObject({ id: event.id, title: 'Long lunch', when: monday });
  expect(
    within(screen.getByRole('list', { name: 'Grid' })).getByRole('button', { name: 'Long lunch' }),
  ).toBeInTheDocument();
});

test('cancelling an edit returns to the unchanged details', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Lunch', when: monday });
  await showApp(core);
  const details = await openOccurrence(user, 'Lunch');

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  await user.type(screen.getByRole('textbox', { name: 'Title' }), ' changed');
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Discard' }));

  expect(await screen.findByRole('dialog', { name: 'Lunch' })).toBeInTheDocument();
  expect(core.calls).not.toContain('editEvent');
});

test('deleting an Event removes it from the grid', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Doomed', when: monday });
  core.addEvent({ calendarId: home.id, title: 'Kept', when: monday });
  const grid = await showApp(core);
  const details = await openOccurrence(user, 'Doomed');

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(ask).toHaveTextContent('“\u2068Doomed\u2069” will be deleted.');
  expect(within(ask).queryByRole('button', { name: 'Close' })).toBeNull();
  expect(core.calls).not.toContain('deleteEvent');
  await user.click(within(ask).getByRole('button', { name: 'Delete' }));

  await waitFor(() => expect(within(grid).queryByRole('button', { name: 'Doomed' })).toBeNull());
  expect(within(grid).getByRole('button', { name: 'Kept' })).toBeInTheDocument();
  expect(screen.queryByRole('dialog')).toBeNull();
});

test('Cancel and Escape in the delete question keep the Event', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Kept', when: monday });
  await showApp(core);
  const details = await openOccurrence(user, 'Kept');

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(within(ask).getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('alertdialog')).toBeNull();
  expect(screen.getByRole('dialog', { name: 'Kept' })).toBeInTheDocument();
  expect(core.calls).not.toContain('deleteEvent');
  expect(core.events).toHaveLength(1);
});

test('“Occurrences changed” re-fetches the visible range and keeps the open Event', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const event = core.addEvent({ calendarId: home.id, title: 'Standup', when: monday });
  const grid = await showApp(core);
  await openOccurrence(user, 'Standup');

  // A change made elsewhere, such as a sync, that the core signals.
  event.title = 'Standup (moved)';
  core.addEvent({ calendarId: home.id, title: 'Retro', when: monday });
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });

  expect(await within(grid).findByRole('button', { name: 'Retro' })).toBeInTheDocument();
  expect(await screen.findByRole('dialog', { name: 'Standup (moved)' })).toBeInTheDocument();
  expect(within(grid).getByRole('button', { name: 'Standup (moved)' })).toHaveAttribute(
    'aria-current',
    'true',
  );
});

test('the open details close when their Event is deleted elsewhere', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Gone', when: monday });
  await showApp(core);
  await openOccurrence(user, 'Gone');

  core.events = [];
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });

  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('an open editor’s draft survives a re-fetch', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const event = core.addEvent({ calendarId: home.id, title: 'Plan', when: monday });
  const grid = await showApp(core);
  await user.click(
    within(await openOccurrence(user, 'Plan')).getByRole('button', { name: 'Edit' }),
  );
  const title = screen.getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'My plan');

  event.title = 'Their plan';
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });
  await within(grid).findByRole('button', { name: 'Their plan' });

  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('My plan');
  await user.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(core.events[0]?.title).toBe('My plan'));
});

test('hiding a Calendar hides its Events', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const work = core.addCalendar({ name: 'Work' });
  core.addEvent({ calendarId: home.id, title: 'At home', when: monday });
  core.addEvent({ calendarId: work.id, title: 'At work', when: monday });
  const grid = await showApp(core);
  await within(grid).findByRole('button', { name: 'At work' });

  await user.click(screen.getByRole('switch', { name: 'Work' }));

  await waitFor(() => expect(within(grid).queryByRole('button', { name: 'At work' })).toBeNull());
  expect(within(grid).getByRole('button', { name: 'At home' })).toBeInTheDocument();
});

test('a link whose text is its URL opens straight away', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const url = 'https://example.org/plain';
  core.addEvent({
    calendarId: home.id,
    title: 'Linked',
    when: monday,
    description: { paragraphs: [[{ kind: 'link', text: url, url }]], text: url },
  });
  await showApp(core);
  const details = await openOccurrence(user, 'Linked');

  await user.click(within(details).getByRole('button', { name: url }));

  expect(core.openedLinks).toEqual([url]);
});

test('a link in the location opens like one in the description', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const url = 'https://zoom.us/j/123';
  core.addEvent({
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
  await showApp(core);
  const details = await openOccurrence(user, 'Call');

  expect(details).toHaveTextContent(`Zoom: ${url}`);
  await user.click(within(details).getByRole('button', { name: url }));

  expect(core.openedLinks).toEqual([url]);
});

test('a link whose text differs from its URL asks first', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const url = 'https://example.com/agenda';
  core.addEvent({
    calendarId: home.id,
    title: 'Linked',
    when: monday,
    description: { paragraphs: [[{ kind: 'link', text: 'the agenda', url }]], text: '' },
  });
  await showApp(core);
  const details = await openOccurrence(user, 'Linked');

  await user.click(within(details).getByRole('button', { name: 'the agenda' }));
  const ask = screen.getByRole('alertdialog', { name: 'Open this link?' });
  expect(ask).toHaveTextContent(`Open ${url} in your browser?`);
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  expect(core.openedLinks).toEqual([]);
  expect(screen.getByRole('dialog', { name: 'Linked' })).toBeInTheDocument();

  await user.click(within(details).getByRole('button', { name: 'the agenda' }));
  await user.click(screen.getByRole('button', { name: 'Open' }));
  expect(core.openedLinks).toEqual([url]);
});

test('the hostile Event goes through the grid, its details and the editor as text', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({
    calendarId: home.id,
    title: 'Hostile',
    when: monday,
    location: hostile.expected.location as StructuredText,
    description: hostile.expected.html as StructuredText,
    pinnedZone: hostile.pinnedZone,
  });
  core.addEvent({
    calendarId: home.id,
    title: hostile.title,
    when: monday,
    description: hostile.expected.plainText as StructuredText,
  });
  const grid = await showApp(core);
  await within(grid).findByRole('button', { name: /onerror/ });
  const details = await openOccurrence(user, 'Hostile');

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
  // Untouched, the description keeps the core's original.
  await screen.findByRole('dialog', { name: 'Hostile' });
  expect(core.events[0]?.description).toEqual(hostile.expected.html);

  await user.click(screen.getByRole('button', { name: 'Close' }));
  await openOccurrence(user, hostile.title.replace(/\r\n/g, ' '));
  expect(document.querySelector('img, script, b, i')).toBeNull();
});

test('dragging an Event to another time reschedules it through the core', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const dentist = core.addEvent({ calendarId: home.id, title: 'Dentist', when: monday });
  await showApp(core);
  const later: When = { kind: 'timed', start: '2026-10-06T14:00:00', end: '2026-10-06T15:30:00' };

  await stubGrid.props!.onreschedule!(dentist.id, null, later);

  expect(core.calls.filter((call) => call === 'rescheduleEvent')).toHaveLength(1);
  expect(core.events[0]).toMatchObject({ title: 'Dentist', when: later });
  await waitFor(() => expect(stubGrid.props!.occurrences[0]?.when).toEqual(later));
});

test('a refused drag says why and tells the grid to put the Event back', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const dentist = core.addEvent({ calendarId: home.id, title: 'Dentist', when: monday });
  await showApp(core);
  vi.spyOn(core, 'rescheduleEvent').mockRejectedValueOnce(new Error('disk full'));
  const later: When = { kind: 'timed', start: '2026-10-06T14:00:00', end: '2026-10-06T15:30:00' };

  await expect(stubGrid.props!.onreschedule!(dentist.id, null, later)).rejects.toThrow('disk full');

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The event couldn’t be moved: Error: disk full',
  );
  expect(core.events[0]?.when).toEqual(monday);

  await stubGrid.props!.onreschedule!(dentist.id, null, later);

  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});

test('trying to drag an Event of a Read-only Calendar says why it can’t move', async () => {
  const core = new FakeCore();
  const locked = core.addCalendar({ name: 'Holidays', readOnly: true });
  const fixed = core.addEvent({ calendarId: locked.id, title: 'Fixed', when: monday });
  await showApp(core);

  stubGrid.props!.onrefusedrag!(fixed.id, null);

  expect(await screen.findByRole('status')).toHaveTextContent(
    '“\u2068Fixed\u2069” can’t be changed: its calendar “\u2068Holidays\u2069” is read-only.',
  );
  expect(core.calls).not.toContain('rescheduleEvent');
});

test('a refused drag names a hostile Event and Calendar as text', async () => {
  const core = new FakeCore();
  const locked = core.addCalendar({ name: hostile.calendarName, readOnly: true });
  const fixed = core.addEvent({ calendarId: locked.id, title: hostile.title, when: monday });
  await showApp(core);

  stubGrid.props!.onrefusedrag!(fixed.id, null);

  const toast = await screen.findByRole('status');
  expect(toast.querySelector('p')!.textContent).toBe(hostile.expected.readOnlyDrag);
  expect(document.querySelector('img, script')).toBeNull();
});

test('the delete question names a hostile Event as text', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: hostile.title, when: monday });
  await showApp(core);
  const details = await openOccurrence(user, hostile.title.replace(/\r\n/g, ' '));

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

/** The Occurrences the grid shows titled `title`, as its buttons. */
function occurrencesTitled(grid: HTMLElement, title: string) {
  return within(grid).queryAllByRole('button', { name: title });
}

test('a Recurring Event is on every date, and an Occurrence opens with its date and repeat', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Yoga', when: monday, repeat: weeklyThreeTimes });
  const grid = await showApp(core);

  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(3));
  await user.click(occurrencesTitled(grid, 'Yoga')[1]);
  const details = await screen.findByRole('dialog', { name: 'Yoga' });

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
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const event = core.addEvent({
    calendarId: home.id,
    title: 'Yoga',
    when: monday,
    repeat: weeklyThreeTimes,
  });
  const grid = await showApp(core);
  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(3));
  await user.click(occurrencesTitled(grid, 'Yoga')[1]);
  const details = await screen.findByRole('dialog', { name: 'Yoga' });

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(ask).toHaveTextContent('Delete only this event, or all events it repeats with?');
  await user.click(within(ask).getByRole('button', { name: 'This event' }));

  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(2));
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(core.cancelled.get(event.id)).toEqual(['2026-10-12T09:00:00']);
});

test('“All events” deletes the whole Recurring Event, and Cancel keeps it', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({ calendarId: home.id, title: 'Yoga', when: monday, repeat: weeklyThreeTimes });
  const grid = await showApp(core);
  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(3));
  await user.click(occurrencesTitled(grid, 'Yoga')[2]);
  const details = await screen.findByRole('dialog', { name: 'Yoga' });

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  await user.click(
    within(screen.getByRole('alertdialog')).getByRole('button', { name: 'All events' }),
  );

  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(0));
  expect(core.events).toEqual([]);
});

test('a new Event can repeat', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const grid = await showApp(core);

  await selectSlot(monday);
  const editor = screen.getByRole('dialog', { name: 'New event' });
  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), 'Yoga');
  await user.selectOptions(within(editor).getByRole('combobox', { name: 'Repeat' }), 'Daily');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(core.events[0]?.repeat).toEqual({
      frequency: 'daily',
      interval: 1,
      end: { kind: 'never' },
    }),
  );
  expect(occurrencesTitled(grid, 'Yoga').length).toBeGreaterThan(20);
});

/** Opens the Occurrence of Yoga on `index`'s date, of three weekly ones from Monday, in the editor. */
async function editYoga(user: ReturnType<typeof userEvent.setup>, index: number) {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const yoga = core.addEvent({
    calendarId: home.id,
    title: 'Yoga',
    when: monday,
    repeat: weeklyThreeTimes,
  });
  const grid = await showApp(core);
  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(3));
  await user.click(occurrencesTitled(grid, 'Yoga')[index]);
  const details = await screen.findByRole('dialog', { name: 'Yoga' });
  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  return { core, grid, yoga, editor: screen.getByRole('dialog', { name: 'Edit event' }) };
}

test('editing an Occurrence starts from its date and asks which events to change on Save', async () => {
  const user = userEvent.setup();
  const { core, yoga, editor } = await editYoga(user, 2);

  expect(within(editor).getByLabelText('Start date')).toHaveValue('2026-10-19');
  expect(within(editor).getByRole('combobox', { name: 'Repeat' })).toHaveDisplayValue('Weekly');
  const title = within(editor).getByRole('textbox', { name: 'Title' });
  await user.clear(title);
  await user.type(title, 'Pilates');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));
  const prompt = screen.getByRole('dialog', { name: 'Change a recurring event' });
  expect(prompt).toHaveTextContent(
    'Change only this event, this and following events, or all events?',
  );
  await user.click(within(prompt).getByRole('button', { name: 'This and following events' }));

  expect(core.scopedChanges).toEqual([
    {
      id: yoga.id,
      recurrenceId: '2026-10-19T09:00:00',
      scope: 'thisAndFollowing',
      draft: expect.objectContaining({
        title: 'Pilates',
        when: { ...monday, start: '2026-10-19T09:00:00', end: '2026-10-19T10:30:00' },
      }),
    },
  ]);
  expect(await screen.findByRole('dialog', { name: 'Pilates' })).toHaveTextContent(
    'Monday, October 19, 2026',
  );
});

test('“This event” changes only that Occurrence, “All events” the whole', async () => {
  const user = userEvent.setup();
  const { core, grid, yoga, editor } = await editYoga(user, 1);

  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), ' outside');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));
  const prompt = screen.getByRole('dialog', { name: 'Change a recurring event' });
  await user.click(within(prompt).getByRole('button', { name: 'This event' }));

  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga outside')).toHaveLength(1));
  expect(occurrencesTitled(grid, 'Yoga')).toHaveLength(2);
  await user.click(
    within(screen.getByRole('dialog', { name: 'Yoga outside' })).getByRole('button', {
      name: 'Close',
    }),
  );

  await user.click(occurrencesTitled(grid, 'Yoga')[0]);
  await user.click(
    within(await screen.findByRole('dialog', { name: 'Yoga' })).getByRole('button', {
      name: 'Edit',
    }),
  );
  const again = screen.getByRole('dialog', { name: 'Edit event' });
  await user.type(within(again).getByRole('textbox', { name: 'Title' }), ' class');
  await user.click(within(again).getByRole('button', { name: 'Save' }));
  await user.click(
    within(screen.getByRole('dialog', { name: 'Change a recurring event' })).getByRole('button', {
      name: 'All events',
    }),
  );

  await waitFor(() => expect(occurrencesTitled(grid, 'Yoga class')).toHaveLength(2));
  expect(core.scopedChanges.map(({ id, scope }) => [id, scope])).toEqual([
    [yoga.id, 'this'],
    [yoga.id, 'all'],
  ]);
});

test('cancelling the question keeps the editor open with what was typed', async () => {
  const user = userEvent.setup();
  const { core, editor } = await editYoga(user, 1);

  await user.type(within(editor).getByRole('textbox', { name: 'Title' }), ' outside');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));
  await user.click(
    within(screen.getByRole('dialog', { name: 'Change a recurring event' })).getByRole('button', {
      name: 'Cancel',
    }),
  );

  expect(screen.queryByRole('dialog', { name: 'Change a recurring event' })).toBeNull();
  expect(within(editor).getByRole('textbox', { name: 'Title' })).toHaveValue('Yoga outside');
  expect(core.scopedChanges).toEqual([]);
});

test('a changed repeat can’t be for this event alone', async () => {
  const user = userEvent.setup();
  const { editor } = await editYoga(user, 1);

  await user.selectOptions(within(editor).getByRole('combobox', { name: 'Repeat' }), 'Daily');
  await user.click(within(editor).getByRole('button', { name: 'Save' }));
  const prompt = screen.getByRole('dialog', { name: 'Change a recurring event' });

  expect(prompt).toHaveTextContent('Change this and following events, or all events?');
  expect(within(prompt).queryByRole('button', { name: 'This event' })).toBeNull();
  expect(within(prompt).getByRole('button', { name: 'All events' })).toBeInTheDocument();
});

test('dropping an Occurrence asks which events to move, and Cancel puts it back', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const yoga = core.addEvent({
    calendarId: home.id,
    title: 'Yoga',
    when: monday,
    repeat: weeklyThreeTimes,
  });
  await showApp(core);
  const later: When = { kind: 'timed', start: '2026-10-13T14:00:00', end: '2026-10-13T15:30:00' };

  const cancelled = stubGrid.props!.onreschedule!(yoga.id, '2026-10-12T09:00:00', later);
  await tick();
  const prompt = screen.getByRole('dialog', { name: 'Change a recurring event' });
  expect(prompt).toHaveTextContent(
    'Move only this event, this and following events, or all events?',
  );
  within(prompt).getByRole('button', { name: 'Cancel' }).click();
  await expect(cancelled).rejects.toBeDefined();
  expect(core.scopedChanges).toEqual([]);
  expect(screen.queryByRole('alert')).toBeNull();

  const moved = stubGrid.props!.onreschedule!(yoga.id, '2026-10-12T09:00:00', later);
  await tick();
  screen.getByRole('button', { name: 'This event' }).click();
  await moved;
  expect(core.scopedChanges).toEqual([
    { id: yoga.id, recurrenceId: '2026-10-12T09:00:00', scope: 'this', when: later },
  ]);
  expect(core.calls).not.toContain('rescheduleEvent');
});

test('an Exception edits without a repeat, and Delete still asks', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({
    calendarId: home.id,
    title: 'Moved',
    when: monday,
    recurrenceId: '2026-10-05T08:00:00',
  });
  await showApp(core);
  const details = await openOccurrence(user, 'Moved');

  await user.click(within(details).getByRole('button', { name: 'Delete' }));
  const ask = screen.getByRole('alertdialog', { name: 'Delete this event?' });
  expect(within(ask).getByRole('button', { name: 'This event' })).toBeInTheDocument();
  await user.click(within(ask).getByRole('button', { name: 'Cancel' }));
  await user.click(within(details).getByRole('button', { name: 'Edit' }));

  expect(screen.queryByRole('combobox', { name: 'Repeat' })).toBeNull();
});
