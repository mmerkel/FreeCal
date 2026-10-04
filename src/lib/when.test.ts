import { expect, test } from 'vitest';
import { endsBeforeStart, fieldsOf, whenForShortcut, whenOf, whenOfSelection } from './when';

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
