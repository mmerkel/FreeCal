import { afterEach, expect, test } from 'vitest';
import { setClockHours } from './clock.svelte';
import {
  allDaySpan,
  describeWhen,
  endsBeforeStart,
  fieldsOf,
  whenForShortcut,
  whenOf,
  whenAfterDrag,
  whenOfSelection,
  type Shift,
} from './when';

afterEach(() => setClockHours(24));

test('a selection of whole days is an all-day When with an exclusive end', () => {
  expect(whenOfSelection(new Date(2026, 9, 5), new Date(2026, 9, 8), true)).toEqual({
    kind: 'allDay',
    start: '2026-10-05',
    end: '2026-10-08',
  });
});

test('a selection of times is a timed When in local wall-clock time', () => {
  expect(whenOfSelection(new Date(2026, 9, 5, 9, 30), new Date(2026, 9, 5, 11), false)).toEqual({
    kind: 'timed',
    start: '2026-10-05T09:30:00',
    end: '2026-10-05T11:00:00',
  });
});

test('the shortcut’s Event starts at the next full hour and lasts an hour', () => {
  const at = (time: string) => whenForShortcut(Temporal.PlainDateTime.from(time));

  expect(at('2026-10-05T10:20:00')).toEqual({
    kind: 'timed',
    start: '2026-10-05T11:00:00',
    end: '2026-10-05T12:00:00',
  });
  expect(at('2026-10-05T10:00:00').start).toBe('2026-10-05T10:00:00');
  expect(at('2026-10-05T23:30:00')).toEqual({
    kind: 'timed',
    start: '2026-10-06T00:00:00',
    end: '2026-10-06T01:00:00',
  });
});

test('the editor shows an all-day Event’s last day, not the day after', () => {
  const fields = fieldsOf({ kind: 'allDay', start: '2026-10-05', end: '2026-10-08' });

  expect(fields).toMatchObject({ allDay: true, startDate: '2026-10-05', endDate: '2026-10-07' });
  expect(whenOf(fields)).toEqual({ kind: 'allDay', start: '2026-10-05', end: '2026-10-08' });
});

test('timed fields turn back into the same When', () => {
  const when = { kind: 'timed', start: '2026-10-05T09:30:00', end: '2026-10-06T01:15:00' } as const;

  const fields = fieldsOf(when);

  expect(fields).toEqual({
    allDay: false,
    startDate: '2026-10-05',
    startTime: '09:30',
    endDate: '2026-10-06',
    endTime: '01:15',
  });
  expect(whenOf(fields)).toEqual(when);
});

test('switching all-day off brings back times', () => {
  const fields = fieldsOf({ kind: 'allDay', start: '2026-10-05', end: '2026-10-06' });

  expect(whenOf({ ...fields, allDay: false })).toEqual({
    kind: 'timed',
    start: '2026-10-05T09:00:00',
    end: '2026-10-05T10:00:00',
  });
});

test('fields that aren’t filled in describe no When', () => {
  const fields = fieldsOf({
    kind: 'timed',
    start: '2026-10-05T09:00:00',
    end: '2026-10-05T10:00:00',
  });

  expect(whenOf({ ...fields, startDate: '' })).toBeUndefined();
  expect(whenOf({ ...fields, endTime: '' })).toBeUndefined();
});

test('an Event may not end before it starts; a timed one may take no time', () => {
  expect(
    endsBeforeStart({ kind: 'timed', start: '2026-10-05T10:00:00', end: '2026-10-05T09:00:00' }),
  ).toBe(true);
  expect(
    endsBeforeStart({ kind: 'timed', start: '2026-10-05T10:00:00', end: '2026-10-05T10:00:00' }),
  ).toBe(false);
  expect(endsBeforeStart({ kind: 'allDay', start: '2026-10-05', end: '2026-10-05' })).toBe(true);
  expect(endsBeforeStart({ kind: 'allDay', start: '2026-10-05', end: '2026-10-06' })).toBe(false);
});

const afternoon = {
  kind: 'timed',
  start: '2026-10-05T14:30:00',
  end: '2026-10-05T16:00:00',
} as const;

// The tests run in `en-US`, whose own default is a 12-hour clock.
test('the details show times on a 24-hour clock and dates in the system locale', () => {
  const said = describeWhen(afternoon);

  expect(said).toContain('Monday, October 5, 2026');
  expect(said).toMatch(/14:30\s*–\s*16:00/);
  expect(said).not.toMatch(/PM/);
});

test('on a 12-hour clock the details show AM and PM', () => {
  setClockHours(12);

  expect(describeWhen(afternoon)).toMatch(/2:30\s*–\s*4:00\s*PM/);
});

const still: Shift = { years: 0, months: 0, days: 0, milliseconds: 0 };
const hours = (count: number) => count * 3_600_000;

test('a drag moves start and end by the same shift, in wall-clock time', () => {
  const by: Shift = { ...still, days: 1, milliseconds: hours(-2) };
  const was = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' } as const;

  expect(
    whenAfterDrag(was, false, {
      allDay: false,
      start: new Date(2026, 9, 6, 7),
      end: new Date(2026, 9, 6, 8, 30),
      startBy: by,
      endBy: by,
    }),
  ).toEqual({ kind: 'timed', start: '2026-10-06T07:00:00', end: '2026-10-06T08:30:00' });
});

test('a drag keeps a wall time across a change of the clocks', () => {
  // Europe moves its clocks on 2026-10-25; a week later is still 09:00.
  const by: Shift = { ...still, days: 7 };
  const was = { kind: 'timed', start: '2026-10-20T09:00:00', end: '2026-10-20T10:00:00' } as const;

  expect(
    whenAfterDrag(was, false, {
      allDay: false,
      start: new Date(2026, 9, 27, 9),
      end: new Date(2026, 9, 27, 10),
      startBy: by,
      endBy: by,
    }),
  ).toEqual({ kind: 'timed', start: '2026-10-27T09:00:00', end: '2026-10-27T10:00:00' });
});

test('a resize moves only the edge it was given', () => {
  const was = { kind: 'allDay', start: '2026-10-05', end: '2026-10-06' } as const;

  expect(
    whenAfterDrag(was, true, {
      allDay: true,
      start: new Date(2026, 9, 5),
      end: new Date(2026, 9, 8),
      startBy: still,
      endBy: { ...still, days: 2 },
    }),
  ).toEqual({ kind: 'allDay', start: '2026-10-05', end: '2026-10-08' });
});

test('a long timed Event shown in the all-day area keeps its times when dragged there', () => {
  const by: Shift = { ...still, days: 3 };
  const was = { kind: 'timed', start: '2026-10-05T18:00:00', end: '2026-10-07T09:00:00' } as const;

  expect(
    whenAfterDrag(was, true, {
      allDay: true,
      start: new Date(2026, 9, 8),
      end: new Date(2026, 9, 11),
      startBy: by,
      endBy: by,
    }),
  ).toEqual({ kind: 'timed', start: '2026-10-08T18:00:00', end: '2026-10-10T09:00:00' });
});

test('a drag between the all-day area and the time grid takes the dates it was dropped on', () => {
  const lunch = {
    kind: 'timed',
    start: '2026-10-05T12:00:00',
    end: '2026-10-05T13:00:00',
  } as const;
  const holiday = { kind: 'allDay', start: '2026-10-05', end: '2026-10-06' } as const;
  const by: Shift = { ...still, days: 1 };

  expect(
    whenAfterDrag(lunch, false, {
      allDay: true,
      start: new Date(2026, 9, 6),
      end: new Date(2026, 9, 7),
      startBy: by,
      endBy: by,
    }),
  ).toEqual({ kind: 'allDay', start: '2026-10-06', end: '2026-10-07' });
  expect(
    whenAfterDrag(holiday, true, {
      allDay: false,
      start: new Date(2026, 9, 5, 14),
      end: new Date(2026, 9, 5, 15),
      startBy: { ...still, milliseconds: hours(14) },
      endBy: { ...still, milliseconds: hours(14) },
    }),
  ).toEqual({ kind: 'timed', start: '2026-10-05T14:00:00', end: '2026-10-05T15:00:00' });
});

test('a timed Event of a day or more covers the dates it touches in the all-day area', () => {
  expect(
    allDaySpan({ kind: 'timed', start: '2026-10-05T18:00:00', end: '2026-10-07T09:00:00' }),
  ).toEqual({ start: '2026-10-05', end: '2026-10-08' });
  expect(
    allDaySpan({ kind: 'timed', start: '2026-10-05T00:00:00', end: '2026-10-07T00:00:00' }),
  ).toEqual({ start: '2026-10-05', end: '2026-10-07' });
});

test('shorter timed Events and all-day Events keep their own place', () => {
  expect(
    allDaySpan({ kind: 'timed', start: '2026-10-05T18:00:00', end: '2026-10-06T17:59:00' }),
  ).toBeUndefined();
  expect(allDaySpan({ kind: 'allDay', start: '2026-10-05', end: '2026-10-06' })).toBeUndefined();
});
