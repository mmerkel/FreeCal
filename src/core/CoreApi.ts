import type {
  Account,
  AccountId,
  Calendar,
  CalendarId,
  Event,
  EventDraft,
  EventId,
  Occurrence,
  Settings,
  SettingsUpdate,
  SidebarState,
  Signal,
  View,
  When,
} from './types';

/** The core application interface. The frontend talks only to this. */
export interface CoreApi {
  /** Starts receiving Signals. Subscribe before reading state, so that nothing falls in the gap. */
  subscribe(onSignal: (signal: Signal) => void): Promise<() => void>;
  /** The current local date. */
  today(): Promise<Temporal.PlainDate>;
  /** The view FreeCal opens in: month on first launch, then the last-used one. */
  lastUsedView(): Promise<View>;
  setLastUsedView(view: View): Promise<void>;
  /** The sidebar as it was left: on first launch shown at the default width. */
  sidebar(): Promise<SidebarState>;
  setSidebar(sidebar: SidebarState): Promise<void>;
  /** The settings of the Settings dialog. */
  settings(): Promise<Settings>;
  /** Changes the settings `update` gives and keeps the others; returns them all as they are now. */
  updateSettings(update: SettingsUpdate): Promise<Settings>;
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
  /**
   * Moves an Event to another time, as a drag or a resize in the grid does,
   * and leaves everything else as it is.
   */
  rescheduleEvent(id: EventId, when: When): Promise<Event>;
  deleteEvent(id: EventId): Promise<void>;
  /** Opens an `http(s)://` or `mailto:` link in the system browser. */
  openLink(url: string): Promise<void>;
  /** FreeCal's version, such as `0.1.0`. */
  version(): Promise<string>;
  /**
   * Shows the window, which starts hidden so that it never flashes the wrong
   * theme. Called once the appearance is known, or that it can't be.
   */
  showWindow(): Promise<void>;
  /** Ends FreeCal, also when it would otherwise keep running in the background. */
  quit(): Promise<void>;
}
