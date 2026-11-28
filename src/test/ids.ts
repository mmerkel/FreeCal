import type { AccountId, CalendarId, CombinedEventKey, JournalTransactionId } from '../core/types';

// Only the core makes IDs and keys; tests make them up with these.
export const accountId = (id: number) => id as AccountId;
export const calendarId = (id: number) => id as CalendarId;
/** A Combined Event Key; the window never looks inside one. */
export const combinedEventKey = (key: string) => key as CombinedEventKey;
export const journalTransactionId = (id: number) => id as JournalTransactionId;
