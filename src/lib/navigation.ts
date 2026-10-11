/**
 * What the grid shows, and whether it is tracking today (spec: "View state"
 * and "Tracking today"). Tracking today is a mode, like "follow my position"
 * in a map app: the Today button's pressed state is the mode.
 */
import type { View } from '../core/types';
import { startOfWeek } from './week.svelte';

export interface Navigation {
  view: View;
  /** The date the shown period is around. */
  date: Temporal.PlainDate;
  tracking: boolean;
}

/** FreeCal always opens on today, tracking it. */
export function opening(view: View, today: Temporal.PlainDate): Navigation {
  return { view, date: today, tracking: true };
}

/** Switching the view keeps the date and the tracking. */
export function switchView(navigation: Navigation, view: View): Navigation {
  return { ...navigation, view };
}

/** The previous (-1) or next (1) period. Changing the date ends tracking, even onto today. */
export function step(navigation: Navigation, by: -1 | 1): Navigation {
  const unit = { day: 'days', week: 'weeks', month: 'months' }[navigation.view];
  return { ...navigation, date: navigation.date.add({ [unit]: by }), tracking: false };
}

/** A day the user picked, such as a clicked day number, in day view. */
export function openDay(date: Temporal.PlainDate): Navigation {
  return { view: 'day', date, tracking: false };
}

/**
 * A month name clicked in the title, in month view. Its own month is only a
 * view switch; another month changes the date, which ends tracking.
 */
export function openMonth(navigation: Navigation, month: Temporal.PlainYearMonth): Navigation {
  return month.equals(navigation.date.toPlainYearMonth())
    ? switchView(navigation, 'month')
    : { view: 'month', date: month.toPlainDate({ day: 1 }), tracking: false };
}

/** Jumps to today and tracks it. */
export function goToToday(navigation: Navigation, today: Temporal.PlainDate): Navigation {
  return { ...navigation, date: today, tracking: true };
}

/** The Today button: pressing it goes to today; un-pressing leaves the view where it is. */
export function toggleToday(navigation: Navigation, today: Temporal.PlainDate): Navigation {
  return navigation.tracking ? { ...navigation, tracking: false } : goToToday(navigation, today);
}

/** On "Date changed" the view follows the new today only while tracking it. */
export function dateChanged(navigation: Navigation, today: Temporal.PlainDate): Navigation {
  return navigation.tracking ? { ...navigation, date: today } : navigation;
}

/** A piece of the period title: text, and for a month's name, the month it opens. */
export interface TitlePart {
  text: string;
  month?: Temporal.PlainYearMonth;
}

/**
 * The shown period in words, in the system locale: a day, a week's range or
 * a month. In day and week view the month names open their month.
 */
export function periodTitle({ view, date }: Navigation): TitlePart[] {
  switch (view) {
    case 'day':
      return titleParts(
        new Intl.DateTimeFormat(undefined, { dateStyle: 'full' }).formatToParts(asDate(date)),
        () => date.toPlainYearMonth(),
      );
    case 'week': {
      const start = startOfWeek(date);
      const end = start.add({ days: 6 });
      return titleParts(
        new Intl.DateTimeFormat(undefined, {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        }).formatRangeToParts(asDate(start), asDate(end)),
        (source) => (source === 'endRange' ? end : start).toPlainYearMonth(),
      );
    }
    case 'month':
      return [{ text: monthTitle(date.toPlainYearMonth()) }];
  }
}

/** A month and its year in words, such as the month view's title. */
export function monthTitle(month: Temporal.PlainYearMonth): string {
  return new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long' }).format(
    asDate(month.toPlainDate({ day: 1 })),
  );
}

/** Joins the formatted parts into text and month names, asking each month name which month it is. */
function titleParts(
  parts: { type: string; value: string; source?: string }[],
  monthOf: (source: string | undefined) => Temporal.PlainYearMonth,
): TitlePart[] {
  const joined: TitlePart[] = [];
  for (const part of parts) {
    const last = joined.at(-1);
    if (part.type === 'month') joined.push({ text: part.value, month: monthOf(part.source) });
    else if (last && !last.month) last.text += part.value;
    else joined.push({ text: part.value });
  }
  return joined;
}

function asDate(day: Temporal.PlainDate): Date {
  return new Date(day.year, day.month - 1, day.day);
}
