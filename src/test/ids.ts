import type { AccountId, CalendarId, EventId, JournalTransactionId } from '../core/types';

// Only the core makes IDs; tests make them up with these.
export const accountId = (id: number) => id as AccountId;
export const calendarId = (id: number) => id as CalendarId;
export const eventId = (id: number) => id as EventId;
export const journalTransactionId = (id: number) => id as JournalTransactionId;
