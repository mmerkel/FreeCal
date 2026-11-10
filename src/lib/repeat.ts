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
