import type { Account, AccountId, Calendar, CalendarId, Signal } from './types';

/** The core application interface. The frontend talks only to this. */
export interface CoreApi {
  /** Starts receiving Signals. Subscribe before reading state, so that nothing falls in the gap. */
  subscribe(onSignal: (signal: Signal) => void): Promise<() => void>;
  /** The current local date. */
  today(): Promise<Temporal.PlainDate>;
  listAccounts(): Promise<Account[]>;
  /** Every Calendar of every Account, in the order they were added. */
  listCalendars(): Promise<Calendar[]>;
  /** Creates a Calendar in the Local Account. `colour` is `#rrggbb`. */
  createCalendar(accountId: AccountId, name: string, colour: string): Promise<Calendar>;
  renameCalendar(id: CalendarId, name: string): Promise<void>;
  recolourCalendar(id: CalendarId, colour: string): Promise<void>;
  /** Deletes a Local Calendar with all its Events. */
  deleteCalendar(id: CalendarId): Promise<void>;
  setCalendarShown(id: CalendarId, shown: boolean): Promise<void>;
  /** Shows this Calendar and hides every other Calendar in all Accounts. */
  showOnlyCalendar(id: CalendarId): Promise<void>;
}
