import type { When } from '../core/types';
import { hourCycle } from './clock.svelte';

/** The dates the grid shows, `to` exclusive. */
export interface VisibleRange {
  from: Temporal.PlainDate;
  to: Temporal.PlainDate;
}

/** The local date of a JavaScript `Date`, as FullCalendar reports dates. */
export function plainDateOf(date: Date): Temporal.PlainDate {
  return Temporal.PlainDate.from({
    year: date.getFullYear(),
    month: date.getMonth() + 1,
    day: date.getDate(),
  });
}

function plainDateTimeOf(date: Date): Temporal.PlainDateTime {
  return plainDateOf(date).toPlainDateTime({
    hour: date.getHours(),
    minute: date.getMinutes(),
    second: date.getSeconds(),
  });
}

/** What a selection in the grid covers, as the core's `When`. */
export function whenOfSelection(start: Date, end: Date, allDay: boolean): When {
  return allDay
    ? { kind: 'allDay', start: plainDateOf(start).toString(), end: plainDateOf(end).toString() }
    : timed(plainDateTimeOf(start), plainDateTimeOf(end));
}

/** How far a drag or resize moved a start or end, as FullCalendar measures it. */
export interface Shift {
  years: number;
  months: number;
  days: number;
  /** The change of the time of day; its sign may differ from the days'. */
  milliseconds: number;
}

/** Where a drag or resize in the grid left an Occurrence. */
export interface Drag {
  /** Whether it is now in the all-day area (or the month view). */
  allDay: boolean;
  start: Date;
  end: Date;
  startBy: Shift;
  endBy: Shift;
}

/**
 * The `When` after a drag or resize of an Occurrence that was at `was`, and
 * shown in the all-day area if `shownAllDay`. Within the same area the start
 * and end move by the drag's shift in wall-clock time, so that a change of
 * the clocks doesn't move them by an hour and a long timed Event dragged in
 * the all-day area keeps its times. Dropped into the other area, it takes the
 * dates it was dropped on.
 */
export function whenAfterDrag(was: When, shownAllDay: boolean, drag: Drag): When {
  if (drag.allDay !== shownAllDay) return whenOfSelection(drag.start, drag.end, drag.allDay);
  if (was.kind === 'allDay') {
    const date = (text: string, by: Shift) => shifted(Temporal.PlainDate.from(text), by).toString();
    return { kind: 'allDay', start: date(was.start, drag.startBy), end: date(was.end, drag.endBy) };
  }
  const time = (text: string, by: Shift) =>
    shifted(Temporal.PlainDateTime.from(text), by).add({ milliseconds: by.milliseconds });
  return timed(time(was.start, drag.startBy), time(was.end, drag.endBy));
}

/** `date` moved by the shift's years, months and days, one unit at a time, because Temporal refuses a shift with mixed signs. */
function shifted<T extends Temporal.PlainDate | Temporal.PlainDateTime>(date: T, by: Shift): T {
  return date.add({ years: by.years }).add({ months: by.months }).add({ days: by.days }) as T;
}

/**
 * The dates a timed Event lasting a day or more covers in the all-day area of
 * the day and week views, `end` exclusive; `undefined` for any other Event.
 */
export function allDaySpan(when: When): { start: string; end: string } | undefined {
  if (when.kind === 'allDay') return undefined;
  const start = Temporal.PlainDateTime.from(when.start);
  const end = Temporal.PlainDateTime.from(when.end);
  if (Temporal.PlainDateTime.compare(end, start.add({ days: 1 })) < 0) return undefined;
  const lastDay = end.toPlainTime().equals(Temporal.PlainTime.from('00:00'))
    ? end.toPlainDate()
    : end.toPlainDate().add({ days: 1 });
  return { start: start.toPlainDate().toString(), end: lastDay.toString() };
}

/** A new Event from the keyboard shortcut: an hour from the next full hour. */
export function whenForShortcut(now: Temporal.PlainDateTime): When {
  const start = now.round({ smallestUnit: 'hour', roundingMode: 'ceil' });
  return timed(start, start.add({ hours: 1 }));
}

function timed(start: Temporal.PlainDateTime, end: Temporal.PlainDateTime): When {
  const text = (time: Temporal.PlainDateTime) => time.toString({ smallestUnit: 'second' });
  return { kind: 'timed', start: text(start), end: text(end) };
}

/**
 * The editor's form of a `When`: dates and times as `<input>` values, with
 * the last day of an all-day Event included, as people say it. Times are kept
 * while all-day is on, so that switching it off again brings them back.
 */
export interface WhenFields {
  allDay: boolean;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
}

export function fieldsOf(when: When): WhenFields {
  if (when.kind === 'allDay') {
    return {
      allDay: true,
      startDate: when.start,
      startTime: '09:00',
      endDate: Temporal.PlainDate.from(when.end).subtract({ days: 1 }).toString(),
      endTime: '10:00',
    };
  }
  const start = Temporal.PlainDateTime.from(when.start);
  const end = Temporal.PlainDateTime.from(when.end);
  const time = (moment: Temporal.PlainDateTime) =>
    moment.toPlainTime().toString({ smallestUnit: 'minute' });
  return {
    allDay: false,
    startDate: start.toPlainDate().toString(),
    startTime: time(start),
    endDate: end.toPlainDate().toString(),
    endTime: time(end),
  };
}

/** The `When` the fields describe, or `undefined` while one of them isn't filled in. */
export function whenOf(fields: WhenFields): When | undefined {
  try {
    const startDate = Temporal.PlainDate.from(fields.startDate);
    const endDate = Temporal.PlainDate.from(fields.endDate);
    if (fields.allDay) {
      return {
        kind: 'allDay',
        start: startDate.toString(),
        end: endDate.add({ days: 1 }).toString(),
      };
    }
    return timed(
      startDate.toPlainDateTime(Temporal.PlainTime.from(fields.startTime)),
      endDate.toPlainDateTime(Temporal.PlainTime.from(fields.endTime)),
    );
  } catch {
    return undefined;
  }
}

/** Whether the Event ends before it starts, which the core refuses. */
export function endsBeforeStart(when: When): boolean {
  return when.kind === 'allDay'
    ? Temporal.PlainDate.compare(when.end, when.start) <= 0
    : Temporal.PlainDateTime.compare(when.end, when.start) < 0;
}

/**
 * How an Event's details say when it takes place: dates in the system
 * locale, times on FreeCal's clock.
 */
export function describeWhen(when: When): string {
  if (when.kind === 'allDay') {
    const start = Temporal.PlainDate.from(when.start);
    const last = Temporal.PlainDate.from(when.end).subtract({ days: 1 });
    const format = new Intl.DateTimeFormat(undefined, { dateStyle: 'full' });
    return start.equals(last)
      ? format.format(asDate(start.toPlainDateTime()))
      : format.formatRange(asDate(start.toPlainDateTime()), asDate(last.toPlainDateTime()));
  }
  const format = new Intl.DateTimeFormat(undefined, {
    dateStyle: 'full',
    timeStyle: 'short',
    hourCycle: hourCycle(),
  });
  return format.formatRange(
    asDate(Temporal.PlainDateTime.from(when.start)),
    asDate(Temporal.PlainDateTime.from(when.end)),
  );
}

function asDate(time: Temporal.PlainDateTime): Date {
  return new Date(time.year, time.month - 1, time.day, time.hour, time.minute);
}
