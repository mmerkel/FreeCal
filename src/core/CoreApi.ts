import type {
  Account,
  AccountId,
  Calendar,
  CalendarId,
  Event,
  EventDraft,
  EventId,
  Occurrence,
  Signal,
} from './types';

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
  /**
   * The Occurrences of shown Calendars on the dates `from` up to `to`
   * (exclusive), in the Display Time Zone and ordered by start.
   */
  listOccurrences(from: Temporal.PlainDate, to: Temporal.PlainDate): Promise<Occurrence[]>;
  /** An Event with everything its details and editor show. */
  event(id: EventId): Promise<Event>;
  /** Creates an Event in a writable Calendar. */
  createEvent(draft: EventDraft): Promise<Event>;
  /** Changes an Event, possibly moving it to another writable Calendar. */
  editEvent(id: EventId, draft: EventDraft): Promise<Event>;
  deleteEvent(id: EventId): Promise<void>;
  /** Opens an `http(s)://` or `mailto:` link in the system browser. */
  openLink(url: string): Promise<void>;
}
