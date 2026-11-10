import type { Frequency, Repeat } from '../core/types';
import { t, tCount, type CountKey } from '../i18n';

/** How an Event's details say how it repeats, with the date in the system locale. */
export function describeRepeat(repeat: Repeat | null, custom: boolean): string | undefined {
  if (!repeat) return custom ? t('repeat.customRule') : undefined;
  const every: Record<Frequency, CountKey> = {
    daily: 'repeat.everyDay',
    weekly: 'repeat.everyWeek',
    monthly: 'repeat.everyMonth',
    yearly: 'repeat.everyYear',
  };
  let rule = tCount(every[repeat.frequency], repeat.interval);
  if (repeat.end.kind === 'count') {
    rule = tCount('repeat.timesTotal', repeat.end.count, { rule });
  } else if (repeat.end.kind === 'until') {
    const date = Temporal.PlainDate.from(repeat.end.date);
    const format = new Intl.DateTimeFormat(undefined, { dateStyle: 'full' });
    rule = t('repeat.until', {
      rule,
      date: format.format(new Date(date.year, date.month - 1, date.day)),
    });
  }
  return custom ? t('repeat.withMore', { rule }) : rule;
}

/** The unit an interval counts in the editor, as in "Every 2 weeks". */
export const intervalUnits: Record<Frequency, CountKey> = {
  daily: 'repeat.days',
  weekly: 'repeat.weeks',
  monthly: 'repeat.months',
  yearly: 'repeat.years',
};

/** Whether two repeat rules, as the editor sets them, are the same. */
export function sameRepeat(a: Repeat | null, b: Repeat | null): boolean {
  if (a === null || b === null) return a === b;
  const end = (repeat: Repeat) =>
    repeat.end.kind === 'count'
      ? `count ${repeat.end.count}`
      : repeat.end.kind === 'until'
        ? `until ${repeat.end.date}`
        : 'never';
  return a.frequency === b.frequency && a.interval === b.interval && end(a) === end(b);
}

/**
 * `date` moved by as many days as a start moved from the date `from` to `to`;
 * unchanged while one of them isn't a date.
 */
export function dateMoved(date: string, from: string, to: string): string {
  try {
    const days = Temporal.PlainDate.from(from).until(Temporal.PlainDate.from(to)).days;
    return Temporal.PlainDate.from(date).add({ days }).toString();
  } catch {
    return date;
  }
}

/**
 * `repeat` as a change that moves its start from the date `from` to `to` leaves
 * it: its end date moved by as many days. The editor moves the end date it
 * shows so, and the core takes a repeat so moved as left alone.
 */
export function repeatMoved(repeat: Repeat | null, from: string, to: string): Repeat | null {
  if (repeat?.end.kind !== 'until') return repeat;
  return { ...repeat, end: { kind: 'until', date: dateMoved(repeat.end.date, from, to) } };
}
