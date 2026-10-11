/**
 * The clock times are shown on: 24-hour by default, whatever the system
 * locale; Settings can switch it to 12-hour (spec: 'Time zones and locale').
 * Every place that shows a time formats it through this module, and follows
 * a change at once.
 */
import type { FormatterInput } from 'fullcalendar';

export type ClockHours = 24 | 12;

const clock = $state<{ hours: ClockHours }>({ hours: 24 });

export function clockHours(): ClockHours {
  return clock.hours;
}

export function setClockHours(hours: ClockHours): void {
  clock.hours = hours;
}

/** The `hourCycle` for `Intl.DateTimeFormat`, which also works with `timeStyle`. */
export function hourCycle(): 'h23' | 'h12' {
  return clock.hours === 24 ? 'h23' : 'h12';
}

/**
 * How the grid writes a time of day, as FullCalendar formatter options:
 * `09:00` on a 24-hour clock, the theme's short `9a` and `9:30a` on a
 * 12-hour one.
 */
export function gridTimeFormat(): FormatterInput {
  return clock.hours === 24
    ? { hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }
    : {
        hour: 'numeric',
        minute: '2-digit',
        hourCycle: 'h12',
        omitZeroMinute: true,
        meridiem: 'narrow',
      };
}
