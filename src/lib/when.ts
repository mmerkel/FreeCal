import type { When } from '../core/types';

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

/** How an Event's details say when it takes place, in the system locale. */
export function describeWhen(when: When): string {
  if (when.kind === 'allDay') {
    const start = Temporal.PlainDate.from(when.start);
    const last = Temporal.PlainDate.from(when.end).subtract({ days: 1 });
    const format = new Intl.DateTimeFormat(undefined, { dateStyle: 'full' });
    return start.equals(last)
      ? format.format(asDate(start.toPlainDateTime()))
      : format.formatRange(asDate(start.toPlainDateTime()), asDate(last.toPlainDateTime()));
  }
  const format = new Intl.DateTimeFormat(undefined, { dateStyle: 'full', timeStyle: 'short' });
  return format.formatRange(
    asDate(Temporal.PlainDateTime.from(when.start)),
    asDate(Temporal.PlainDateTime.from(when.end)),
  );
}

function asDate(time: Temporal.PlainDateTime): Date {
  return new Date(time.year, time.month - 1, time.day, time.hour, time.minute);
}
