import type { Calendar, SkippedEvent } from '../core/types';
import { t } from '../i18n';

/**
 * Why an undo left a Skipped Event as it is, naming the Event, and the
 * Calendar it is read-only in.
 */
export function skippedText(skipped: SkippedEvent, calendars: readonly Calendar[]): string {
  const { title, reason } = skipped;
  switch (reason.kind) {
    case 'readOnlyCalendar': {
      const calendar = calendars.find((calendar) => calendar.id === reason.calendarId);
      return calendar
        ? t('undo.skipped.readOnlyCalendar', { title, calendar: calendar.name })
        : t('undo.skipped.readOnly', { title });
    }
    case 'hasGuests':
      return t('undo.skipped.hasGuests', { title });
    case 'notInChangeJournal':
      return t('undo.skipped.notInChangeJournal', { title });
    case 'changedTogetherWith':
      return t('undo.skipped.changedTogetherWith', { title, with: reason.title });
  }
}
