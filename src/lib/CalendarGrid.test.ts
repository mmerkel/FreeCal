import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
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
    recurrenceId: null,
    ...fields,
  };
}

/** Occurrences of one Recurring Event on Monday and Tuesday at 9:00, titled after their day. */
const recurring = ['05', '06'].map((day) =>
  occurrence({
    eventId: eventId(7),
    title: `Yoga ${day}`,
    when: { kind: 'timed', start: `2026-10-${day}T09:00:00`, end: `2026-10-${day}T10:00:00` },
    recurrenceId: `2026-10-${day}T09:00:00`,
  }),
);

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

  expect(onopenevent.mock.calls).toEqual([
    [7, null],
    [7, null],
  ]);
});

test('reports which Occurrence of a Recurring Event was clicked', async () => {
  const user = userEvent.setup();
  const onopenevent = vi.fn();
  render(CalendarGrid, { ...thisMonth, occurrences: recurring, calendars: [home], onopenevent });
  await waitFor(() => expect(screen.getByText('Yoga 06')).toBeInTheDocument());

  await user.click(chip('Yoga 06'));

  expect(onopenevent.mock.calls).toEqual([[7, '2026-10-06T09:00:00']]);
});

test('marks only the open Occurrence of a Recurring Event', async () => {
  render(CalendarGrid, {
    ...thisMonth,
    occurrences: recurring,
    calendars: [home],
    selectedEventId: eventId(7),
    selectedRecurrenceId: '2026-10-06T09:00:00',
  });

  await waitFor(() => expect(chip('Yoga 06')).toHaveClass('selected-event'));
  expect(chip('Yoga 05')).not.toHaveClass('selected-event');
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

test('marks the chips filled with their Calendar colour, for their hover effect', async () => {
  const occurrences = [
    occurrence({ eventId: eventId(1), title: 'Meeting' }),
    occurrence({
      eventId: eventId(2),
      title: 'Holiday',
      when: { kind: 'allDay', start: '2026-10-06', end: '2026-10-07' },
    }),
    occurrence({
      eventId: eventId(3),
      title: 'Trip',
      when: { kind: 'timed', start: '2026-10-07T09:00:00', end: '2026-10-09T18:00:00' },
    }),
  ];
  const { rerender } = render(CalendarGrid, { ...thisMonth, occurrences, calendars: [home] });

  await waitFor(() => expect(chip('Holiday')).toHaveClass('event-block'));
  expect(chip('Trip')).toHaveClass('event-block');
  // A timed Event in the month view is a dot and text, not filled.
  expect(chip('Meeting')).not.toHaveClass('event-block');

  await rerender({ view: 'week', date: Temporal.PlainDate.from('2026-10-05') });

  await waitFor(() => expect(chip('Meeting')).toHaveClass('event-block'));
  expect(chip('Holiday')).toHaveClass('event-block');
  expect(chip('Trip')).toHaveClass('event-block');
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

/** The week of 2026-10-05, a Monday, with today on its Saturday. */
const thatWeek = {
  view: 'week',
  date: Temporal.PlainDate.from('2026-10-05'),
  today: Temporal.PlainDate.from('2026-10-10'),
} as const;

/** Whether a chip is in the all-day area of the day or week view. */
function inAllDayArea(found: HTMLElement): boolean {
  const row = found.closest('[role="row"]');
  return within(row as HTMLElement).queryByRole('rowheader', { name: 'All-day' }) !== null;
}

test('lays out overlapping Events side by side in the week view', async () => {
  render(CalendarGrid, {
    ...thatWeek,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'First' }),
      occurrence({
        eventId: eventId(2),
        title: 'Second',
        when: { kind: 'timed', start: '2026-10-05T09:30:00', end: '2026-10-05T10:30:00' },
      }),
    ],
    calendars: [home],
  });
  await waitFor(() => expect(screen.getByText('Second')).toBeInTheDocument());

  // Each takes half the day's width, rather than one covering the other.
  const placement = (title: string) => chip(title).parentElement!.style;
  expect(placement('First').insetInlineEnd).toBe('50%');
  expect(placement('Second').insetInlineStart).toBe('50%');
});

test('shows all-day Events and timed Events of a day or more in the all-day area', async () => {
  render(CalendarGrid, {
    ...thatWeek,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Meeting' }),
      occurrence({
        eventId: eventId(2),
        title: 'Holiday',
        when: { kind: 'allDay', start: '2026-10-06', end: '2026-10-09' },
      }),
      occurrence({
        eventId: eventId(3),
        title: 'Conference',
        when: { kind: 'timed', start: '2026-10-07T18:00:00', end: '2026-10-09T12:00:00' },
      }),
    ],
    calendars: [home],
  });
  await waitFor(() => expect(screen.getByText('Conference')).toBeInTheDocument());

  expect(inAllDayArea(chip('Holiday'))).toBe(true);
  expect(inAllDayArea(chip('Conference'))).toBe(true);
  expect(inAllDayArea(chip('Meeting'))).toBe(false);
});

/** The handle on a chip's edge that resizes it: an empty first or last child. */
function resizer(found: HTMLElement, edge: 'start' | 'end'): Element | undefined {
  const handle = edge === 'start' ? found.firstElementChild : found.lastElementChild;
  return handle && handle.childElementCount === 0 && handle.textContent === '' ? handle : undefined;
}

test('offers a resize on both edges for Events of writable Calendars only', async () => {
  const shared: Calendar = { ...home, id: calendarId(2), name: 'Shared', readOnly: true };
  render(CalendarGrid, {
    ...thatWeek,
    occurrences: [
      occurrence({ eventId: eventId(1), title: 'Mine' }),
      occurrence({ eventId: eventId(2), title: 'Theirs', calendarId: shared.id }),
      occurrence({
        eventId: eventId(3),
        title: 'Holiday',
        when: { kind: 'allDay', start: '2026-10-06', end: '2026-10-09' },
      }),
      occurrence({
        eventId: eventId(4),
        title: 'Their holiday',
        calendarId: shared.id,
        when: { kind: 'allDay', start: '2026-10-06', end: '2026-10-09' },
      }),
    ],
    calendars: [home, shared],
    onreschedule: vi.fn(async () => {}),
  });
  await waitFor(() => expect(screen.getByText('Theirs')).toBeInTheDocument());

  for (const edge of ['start', 'end'] as const) {
    expect(resizer(chip('Mine'), edge)).toBeDefined();
    expect(resizer(chip('Holiday'), edge)).toBeDefined();
    expect(resizer(chip('Theirs'), edge)).toBeUndefined();
    expect(resizer(chip('Their holiday'), edge)).toBeUndefined();
  }
});

// Moving one needs the recurrence scope prompt (ticket 08).
test('offers no drag or resize for Occurrences of Recurring Events', async () => {
  render(CalendarGrid, {
    ...thatWeek,
    occurrences: recurring,
    calendars: [home],
    onreschedule: vi.fn(async () => {}),
  });
  await waitFor(() => expect(screen.getByText('Yoga 06')).toBeInTheDocument());

  for (const edge of ['start', 'end'] as const) {
    expect(resizer(chip('Yoga 05'), edge)).toBeUndefined();
    expect(resizer(chip('Yoga 06'), edge)).toBeUndefined();
  }
});

/** The day the grid highlights as today. */
function highlighted(): string | null | undefined {
  return document
    .querySelector('[role="columnheader"][aria-current="date"]')
    ?.getAttribute('data-date');
}

const newToday = Temporal.PlainDate.from('2026-10-11');

/** Presses on `target` the way each interaction starts, and checks that a new today waits for the release. */
async function holdsBackTodayWhilePressing(target: () => Element) {
  const oninteract = vi.fn();
  const shown = render(CalendarGrid, {
    ...thatWeek,
    occurrences: [occurrence({ title: 'Dentist' })],
    calendars: [home],
    onreschedule: vi.fn(async () => {}),
    oninteract,
  });
  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());
  expect(highlighted()).toBe('2026-10-10');

  fireEvent.mouseDown(target());
  await shown.rerender({ today: newToday });

  expect(oninteract.mock.calls).toEqual([[true]]);
  expect(highlighted()).toBe('2026-10-10');

  fireEvent.mouseUp(window);

  await waitFor(() => expect(highlighted()).toBe('2026-10-11'));
  expect(oninteract.mock.calls).toEqual([[true], [false]]);
}

test('holds back a new today during a drag', async () => {
  await holdsBackTodayWhilePressing(() => chip('Dentist'));
});

test('holds back a new today during a resize', async () => {
  await holdsBackTodayWhilePressing(() => resizer(chip('Dentist'), 'end')!);
});

test('holds back a new today during a drag-to-create', async () => {
  await holdsBackTodayWhilePressing(() =>
    document.querySelector('[role="gridcell"][data-date="2026-10-08"]')!,
  );
});

test('shows a new today straight away when nothing is pressed', async () => {
  const shown = render(CalendarGrid, { ...thatWeek, occurrences: [], calendars: [] });
  await waitFor(() => expect(highlighted()).toBe('2026-10-10'));

  await shown.rerender({ today: newToday });

  await waitFor(() => expect(highlighted()).toBe('2026-10-11'));
});
