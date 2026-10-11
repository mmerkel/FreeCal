/**
 * The day weeks start on: Monday by default, whatever the system locale;
 * Settings can switch it to Saturday or Sunday (spec: 'Time zones and
 * locale'). Everything that lays out weeks reads it from this module, and
 * follows a change at once. It only affects display, never repeat rules.
 */

export type FirstDayOfWeek = 'monday' | 'saturday' | 'sunday';

const week = $state<{ firstDay: FirstDayOfWeek }>({ firstDay: 'monday' });

export function firstDayOfWeek(): FirstDayOfWeek {
  return week.firstDay;
}

export function setFirstDayOfWeek(firstDay: FirstDayOfWeek): void {
  week.firstDay = firstDay;
}

/** The first day as a JavaScript day number (Sunday is 0), as FullCalendar's `firstDay` takes it. */
export function firstWeekdayNumber(): 0 | 1 | 6 {
  return { sunday: 0, monday: 1, saturday: 6 }[week.firstDay] as 0 | 1 | 6;
}

/** The first day of the week that `date` is in. */
export function startOfWeek(date: Temporal.PlainDate): Temporal.PlainDate {
  // Temporal numbers the days from Monday (1) to Sunday (7).
  const first = { monday: 1, saturday: 6, sunday: 7 }[week.firstDay];
  return date.subtract({ days: (date.dayOfWeek - first + 7) % 7 });
}
