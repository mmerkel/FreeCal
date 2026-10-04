// The App's Event flows: creating, opening, editing and deleting Events, run
// against the fake core with a stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import App from './App.svelte';
import type { Description, When } from './core/types';
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

test('an Event’s details show its title, time, place, description and Calendar', async () => {
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

test('an Event of a Read-only Calendar can’t be edited or deleted', async () => {
  const user = userEvent.setup();
  const core = new FakeCore();
  const locked = core.addCalendar({ name: 'Locked', readOnly: true });
  core.addEvent({ calendarId: locked.id, title: 'Fixed', when: monday });
  await showApp(core);

  const details = await openOccurrence(user, 'Fixed');

  expect(within(details).queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(within(details).queryByRole('button', { name: 'Delete' })).toBeNull();
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

  await waitFor(() => expect(within(grid).queryByRole('button', { name: 'Doomed' })).toBeNull());
  expect(within(grid).getByRole('button', { name: 'Kept' })).toBeInTheDocument();
  expect(screen.queryByRole('dialog')).toBeNull();
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
    location: hostile.location,
    description: hostile.expected.html as Description,
  });
  core.addEvent({
    calendarId: home.id,
    title: hostile.title,
    when: monday,
    description: hostile.expected.plainText as Description,
  });
  const grid = await showApp(core);
  await within(grid).findByRole('button', { name: /onerror/ });
  const details = await openOccurrence(user, 'Hostile');

  expect(details).toHaveTextContent(hostile.location);
  expect(details).toHaveTextContent('click me');
  // Disallowed links are plain text; only the core's link pieces are links.
  for (const text of ['click me', 'passwords', 'data:text/html']) {
    expect(within(details).queryByRole('button', { name: new RegExp(text) })).toBeNull();
  }
  expect(within(details).getByRole('button', { name: 'the agenda' })).toHaveAttribute(
    'title',
    'https://example.com/agenda',
  );
  expect(document.querySelector('img, script, b, i, p > p')).toBeNull();

  await user.click(within(details).getByRole('button', { name: 'Edit' }));
  expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue(
    hostile.expected.html.text,
  );
  expect(screen.getByRole('textbox', { name: 'Place' })).toHaveValue(hostile.location);
  await user.click(screen.getByRole('button', { name: 'Save' }));
  // Untouched, the description keeps the core's original.
  await screen.findByRole('dialog', { name: 'Hostile' });
  expect(core.events[0]?.description).toEqual(hostile.expected.html);

  await user.click(screen.getByRole('button', { name: 'Close' }));
  await openOccurrence(user, hostile.title.replace(/\r\n/g, ' '));
  expect(document.querySelector('img, script, b, i')).toBeNull();
});
