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

/** The shown period in words, in the system locale: a day, a week's range or a month. */
export function periodTitle({ view, date }: Navigation): string {
  const asDate = (day: Temporal.PlainDate) => new Date(day.year, day.month - 1, day.day);
  switch (view) {
    case 'day':
      return new Intl.DateTimeFormat(undefined, { dateStyle: 'full' }).format(asDate(date));
    case 'week': {
      const start = startOfWeek(date);
      return new Intl.DateTimeFormat(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      }).formatRange(asDate(start), asDate(start.add({ days: 6 })));
    }
    case 'month':
      return new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long' }).format(
        asDate(date),
      );
  }
}
