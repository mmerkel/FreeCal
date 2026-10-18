import type {
  Account,
  AccountId,
  Calendar,
  CalendarId,
  Changed,
  Event,
  ErrorCode,
  EventDraft,
  EventId,
  ExistingEvents,
  ImportOutcome,
  ImportSummary,
  Occurrence,
  Scope,
  Settings,
  SettingsUpdate,
  SidebarState,
  Signal,
  UnitId,
  View,
  When,
} from './types';

/**
 * The core application interface. The frontend talks only to this. A method
 * that fails rejects with an `ErrorCode`, which `errorText` puts into words.
 */
export interface CoreApi {
  /**
   * Why FreeCal couldn't start, such as another running FreeCal holding the
   * Local Store, or `null` when it started. Asked before anything else: after
   * a startup failure every other read fails.
   */
  startupFailure(): Promise<ErrorCode | null>;
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
  /**
   * Creates an Event in a writable Calendar. Each change to Events comes
   * back with its Change Journal unit, which undoes it, or `null` when it
   * changed nothing.
   */
  createEvent(draft: EventDraft): Promise<Changed<Event>>;
  /**
   * Changes an Event, possibly moving it to another writable Calendar. A
   * repeat rule left as the Event's `repeat` keeps its stored rule. A
   * Recurring Event changes as a whole, an Exception alone.
   */
  editEvent(id: EventId, draft: EventDraft): Promise<Changed<Event>>;
  /**
   * Moves an Event to another time, as a drag or a resize in the grid does,
   * and leaves everything else as it is.
   */
  rescheduleEvent(id: EventId, when: When): Promise<Changed<Event>>;
  /**
   * Changes one Occurrence of a Recurring Event, given by its `eventId` and
   * `recurrenceId`, in `scope`: alone, with every later one, or with all of
   * them. What the user changed is what differs from the Occurrence as it
   * was. Resolves to the Occurrence as it is now.
   */
  editOccurrence(
    id: EventId,
    recurrenceId: string,
    scope: Scope,
    draft: EventDraft,
  ): Promise<Changed<Occurrence>>;
  /** Moves one Occurrence of a Recurring Event to another time in `scope`, as a drag or a resize does. */
  rescheduleOccurrence(
    id: EventId,
    recurrenceId: string,
    scope: Scope,
    when: When,
  ): Promise<Changed<Occurrence>>;
  /**
   * Deletes an Event; for a Recurring Event or one of its Exceptions, the
   * whole Recurring Event.
   */
  deleteEvent(id: EventId): Promise<Changed<null>>;
  /**
   * Deletes one Occurrence of a Recurring Event, given by its `eventId` and
   * `recurrenceId`, and leaves the rest of the Recurring Event in place.
   */
  deleteOccurrence(id: EventId, recurrenceId: string): Promise<Changed<null>>;
  /**
   * Asks the user for an `.ics` or `.zip` file to import through the system's
   * file dialog, with the dialog's `title` and the name of its file `filter`,
   * and reads it. Resolves to the file's name, or `null` when the user
   * cancels. The shell holds the file; the window never sees its path.
   */
  pickImportFile(title: string, filter: string): Promise<string | null>;
  /** Previews importing the picked file into a Calendar. Changes no Events. */
  previewImport(calendarId: CalendarId): Promise<ImportSummary>;
  /**
   * Imports the picked file into a writable Calendar, doing with the Events
   * it already has (same UID) what `existing` says, as one undoable unit.
   * Afterwards no file is picked.
   */
  runImport(calendarId: CalendarId, existing: ExistingEvents): Promise<ImportOutcome>;
  /** Lets go of the picked file without importing it. */
  discardImport(): Promise<void>;
  /**
   * Undoes a Change Journal unit whole, refused while an Event it changed
   * was changed since. A unit is undone once; the undo is a unit itself,
   * which redoes it.
   */
  undoUnit(id: UnitId): Promise<Changed<null>>;
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
