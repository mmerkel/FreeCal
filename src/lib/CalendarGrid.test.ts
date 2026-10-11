import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, test, vi } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar, Occurrence } from '../core/types';
import { accountId, calendarId, eventId } from '../test/ids';
import CalendarGrid from './CalendarGrid.svelte';
import { setClockHours } from './clock.svelte';
import { setFirstDayOfWeek } from './week.svelte';

const today = Temporal.PlainDate.from('2026-10-03');
/** The month view of today, as FreeCal opens. */
const thisMonth = { view: 'month', date: today, today } as const;

afterEach(() => {
  setClockHours(24);
  setFirstDayOfWeek('monday');
});

const home: Calendar = {
  id: calendarId(1),
  accountId: accountId(1),
  name: 'Home',
  colour: '#3366cc',
  shown: true,
  readOnly: false,
};

function occurrence(fields: Partial<Occurrence> & Pick<Occurrence, 'title'>): Occurrence {
  return {
    eventId: eventId(1),
    calendarId: home.id,
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
    ...fields,
  };
}

/** An Occurrence's chip in the grid, found by its title. */
function chip(title: string): HTMLElement {
  const found = screen.getByText(title).closest<HTMLElement>('[role="button"]');
  if (!found) throw new Error(`no chip for ${title}`);
  return found;
}

test('shows the month view of today with today highlighted', async () => {
  render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [] });

  expect(screen.getByRole('grid', { name: 'October 2026' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'October 3, 2026' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});

test('reports the dates it shows', async () => {
  const onrangechange = vi.fn();

  render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [], onrangechange });

  await waitFor(() => expect(onrangechange).toHaveBeenCalled());
  const range = onrangechange.mock.lastCall![0];
  expect(range.from.toString()).toBe('2026-09-28');
  expect(range.to.toString()).toBe('2026-11-09');
});

test('shows the week or the day of the given date', async () => {
  const onrangechange = vi.fn();
  const date = Temporal.PlainDate.from('2026-10-14');

  const { rerender } = render(CalendarGrid, {
    view: 'week',
    date,
    today,
    occurrences: [],
    calendars: [],
    onrangechange,
  });

  await waitFor(() => expect(onrangechange).toHaveBeenCalled());
  expect(onrangechange.mock.lastCall![0].from.toString()).toBe('2026-10-12');
  expect(onrangechange.mock.lastCall![0].to.toString()).toBe('2026-10-19');

  await rerender({ view: 'day' });

  await waitFor(() => expect(onrangechange.mock.lastCall![0].from.toString()).toBe('2026-10-14'));
  expect(onrangechange.mock.lastCall![0].to.toString()).toBe('2026-10-15');
});

test('follows a new date', async () => {
  const onrangechange = vi.fn();
  const { rerender } = render(CalendarGrid, {
    ...thisMonth,
    occurrences: [],
    calendars: [],
    onrangechange,
  });

  await rerender({ date: Temporal.PlainDate.from('2026-12-24') });

  expect(await screen.findByRole('grid', { name: 'December 2026' })).toBeInTheDocument();
  expect(onrangechange.mock.lastCall![0].from.toString()).toBe('2026-11-30');
});

test('moves the today highlight to a new today without moving the view', async () => {
  const { rerender } = render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [] });

  await rerender({ today: Temporal.PlainDate.from('2026-10-04') });

  await waitFor(() =>
    expect(screen.getByRole('gridcell', { name: 'October 4, 2026' })).toHaveAttribute(
      'aria-current',
      'date',
    ),
  );
  expect(screen.getByRole('gridcell', { name: 'October 3, 2026' })).not.toHaveAttribute(
    'aria-current',
  );
  expect(screen.getByRole('grid', { name: 'October 2026' })).toBeInTheDocument();
});

test('reports a click on a day number', async () => {
  const user = userEvent.setup();
  const onopenday = vi.fn();
  render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [], onopenday });

  const cell = screen.getByRole('gridcell', { name: 'October 14, 2026' });
  await user.click(within(cell).getByRole('link'));

  expect(onopenday).toHaveBeenCalledOnce();
  expect(onopenday.mock.lastCall![0].toString()).toBe('2026-10-14');
});

/** The weekday names over the month view's columns, in order. */
function weekdays(): string[] {
  return screen.getAllByRole('columnheader').map((header) => header.textContent ?? '');
}

// The tests run in `en-US`, whose own weeks start on Sunday.
test('starts weeks on Monday, and on Sunday or Saturday when set', async () => {
  const onrangechange = vi.fn();
  render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [], onrangechange });

  expect(weekdays()[0]).toMatch(/^Mon/);

  setFirstDayOfWeek('sunday');

  await waitFor(() => expect(weekdays()[0]).toMatch(/^Sun/));
  expect(onrangechange.mock.lastCall![0].from.toString()).toBe('2026-09-27');

  setFirstDayOfWeek('saturday');

  await waitFor(() => expect(weekdays()[0]).toMatch(/^Sat/));
  expect(onrangechange.mock.lastCall![0].from.toString()).toBe('2026-09-26');
});

test('shows the Occurrences it is given in their Calendar’s colour', async () => {
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Dentist' }),
      occurrence({
        eventId: eventId(2),
        title: 'Holiday',
        when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' },
      }),
      occurrence({ eventId: eventId(3), title: '' }),
    ],
    calendars: [home],
  });

  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());
  expect(screen.getByText('Holiday')).toBeInTheDocument();
  expect(screen.getByText('(No title)')).toBeInTheDocument();
  expect(chip('Dentist').getAttribute('style')).toContain('#3366cc');
});

/** The colour dot at the front of a chip: an empty element before its text. */
function dot(found: HTMLElement): Element | undefined {
  const first = found.firstElementChild;
  return first && first.childElementCount === 0 && first.textContent === '' ? first : undefined;
}

test('marks timed Events with a dot in their Calendar’s colour and fills all-day ones', async () => {
  const shown = render(CalendarGrid, {
    ...thisMonth,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Dentist' }),
      occurrence({
        eventId: eventId(2),
        title: 'Holiday',
        when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' },
      }),
    ],
    calendars: [home],
  });

  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());
  expect(dot(chip('Dentist'))).toBeDefined();
  expect(chip('Dentist').style.getPropertyValue('--fc-event-color')).toBe('#3366cc');
  expect(dot(chip('Holiday'))).toBeUndefined();
  expect(chip('Holiday').style.getPropertyValue('--fc-event-color')).toBe('#3366cc');

  await shown.rerender({ calendars: [{ ...home, colour: '#0b8043' }] });

  await waitFor(() =>
    expect(chip('Dentist').style.getPropertyValue('--fc-event-color')).toBe('#0b8043'),
  );
  expect(chip('Holiday').style.getPropertyValue('--fc-event-color')).toBe('#0b8043');
});

test('writes on a filled Event in whichever of dark or white is easier to read', async () => {
  const work: Calendar = { ...home, id: calendarId(2), name: 'Work', colour: '#f6bf26' };
  const allDay = { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' } as const;
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Yellow', calendarId: work.id, when: allDay }),
      occurrence({ eventId: eventId(2), title: 'Blue', when: allDay }),
    ],
    calendars: [home, work],
  });

  await waitFor(() => expect(screen.getByText('Yellow')).toBeInTheDocument());
  expect(chip('Yellow').style.getPropertyValue('--fc-event-contrast-color')).toBe('#171717');
  expect(chip('Blue').style.getPropertyValue('--fc-event-contrast-color')).toBe('#ffffff');
});

test('shows an Event’s time on a 24-hour clock, and on a 12-hour one when set', async () => {
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [
      occurrence({
        title: 'Lunch',
        when: { kind: 'timed', start: '2026-10-05T13:30:00', end: '2026-10-05T14:30:00' },
      }),
    ],
    calendars: [home],
  });

  await waitFor(() => expect(chip('Lunch')).toHaveTextContent('13:30'));

  setClockHours(12);

  await waitFor(() => expect(chip('Lunch')).toHaveTextContent('1:30p'));
  expect(chip('Lunch')).not.toHaveTextContent('13:30');
});

test('shows new Occurrences when it is given them', async () => {
  const shown = render(CalendarGrid, { ...thisMonth, occurrences: [], calendars: [home] });

  await shown.rerender({ occurrences: [occurrence({ title: 'Later' })] });

  await waitFor(() => expect(screen.getByText('Later')).toBeInTheDocument());
});

test('reports a click on an Occurrence, also with the keyboard', async () => {
  const user = userEvent.setup();
  const onopenevent = vi.fn();
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [occurrence({ eventId: eventId(7), title: 'Dentist' })],
    calendars: [home],
    onopenevent,
  });
  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());

  await user.click(chip('Dentist'));
  chip('Dentist').focus();
  await user.keyboard('{Enter}');

  expect(onopenevent.mock.calls).toEqual([[7], [7]]);
});

test('marks the selected Event', async () => {
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Chosen' }),
      occurrence({ eventId: eventId(2), title: 'Other' }),
    ],
    calendars: [home],
    selectedEventId: eventId(1),
  });

  await waitFor(() => expect(chip('Chosen')).toHaveClass('selected-event'));
  expect(chip('Other')).not.toHaveClass('selected-event');
});

test('draws the hostile title as text, never as markup', async () => {
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: [occurrence({ title: hostile.title })],
    calendars: [home],
  });

  await waitFor(() =>
    expect(screen.getByText(hostile.title.replace(/\s+/g, ' ').trim())).toBeInTheDocument(),
  );
  expect(document.querySelector('img, script:not([src]), b')).toBeNull();
});
