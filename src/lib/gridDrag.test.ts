import { expect, test } from 'vitest';
import type { When } from '../core/types';
import type { Shift } from './when';
import { idOf, rescheduleOfDrop, rescheduleOfResize, type GridEvent } from './gridDrag';

const still: Shift = { years: 0, months: 0, days: 0, milliseconds: 0 };
const monday: When = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' };

/** An Occurrence as FullCalendar hands it back, made from `when` and shown where `allDay` says. */
function gridEvent(when: When, allDay: boolean, start: Date, end: Date | null): GridEvent {
  return { id: '7', allDay, start, end, extendedProps: { when } };
}

test('FullCalendar’s string ID is the Event’s ID again', () => {
  expect(idOf({ id: '42' })).toBe(42);
  expect(idOf({ id: '42/2026-10-05T09:00:00' })).toBe(42);
});

test('a drop moves the Event’s start and end by the drag’s shift', () => {
  const by: Shift = { ...still, days: 2, milliseconds: 3_600_000 };
  const oldEvent = gridEvent(monday, false, new Date(2026, 9, 5, 9), new Date(2026, 9, 5, 10));
  const event = gridEvent(monday, false, new Date(2026, 9, 7, 10), new Date(2026, 9, 7, 11));

  expect(rescheduleOfDrop({ event, oldEvent, delta: by })).toEqual({
    id: 7,
    when: { kind: 'timed', start: '2026-10-07T10:00:00', end: '2026-10-07T11:00:00' },
  });
});

test('a resize moves each edge by its own shift', () => {
  const oldEvent = gridEvent(monday, false, new Date(2026, 9, 5, 9), new Date(2026, 9, 5, 10));
  const event = gridEvent(monday, false, new Date(2026, 9, 5, 9), new Date(2026, 9, 5, 11, 30));

  expect(
    rescheduleOfResize({
      event,
      oldEvent,
      startDelta: still,
      endDelta: { ...still, milliseconds: 5_400_000 },
    }),
  ).toEqual({
    id: 7,
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T11:30:00' },
  });
});

test('a resize from the start keeps the end', () => {
  const oldEvent = gridEvent(monday, false, new Date(2026, 9, 5, 9), new Date(2026, 9, 5, 10));
  const event = gridEvent(monday, false, new Date(2026, 9, 5, 8, 30), new Date(2026, 9, 5, 10));

  expect(
    rescheduleOfResize({
      event,
      oldEvent,
      startDelta: { ...still, milliseconds: -1_800_000 },
      endDelta: still,
    }),
  ).toEqual({
    id: 7,
    when: { kind: 'timed', start: '2026-10-05T08:30:00', end: '2026-10-05T10:00:00' },
  });
});

test('a bar resized from the start keeps its last day', () => {
  const holiday: When = { kind: 'allDay', start: '2026-10-06', end: '2026-10-09' };
  const oldEvent = gridEvent(holiday, true, new Date(2026, 9, 6), new Date(2026, 9, 9));
  const event = gridEvent(holiday, true, new Date(2026, 9, 4), new Date(2026, 9, 9));

  expect(
    rescheduleOfResize({ event, oldEvent, startDelta: { ...still, days: -2 }, endDelta: still })
      .when,
  ).toEqual({ kind: 'allDay', start: '2026-10-04', end: '2026-10-09' });
});

test('a drop into the all-day area takes the day it was dropped on', () => {
  const oldEvent = gridEvent(monday, false, new Date(2026, 9, 5, 9), new Date(2026, 9, 5, 10));
  const event = gridEvent(monday, true, new Date(2026, 9, 6), new Date(2026, 9, 7));

  expect(rescheduleOfDrop({ event, oldEvent, delta: { ...still, days: 1 } }).when).toEqual({
    kind: 'allDay',
    start: '2026-10-06',
    end: '2026-10-07',
  });
});
