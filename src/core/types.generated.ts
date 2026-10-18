// Generated from the Rust core by core/src/typescript.rs. Don't edit;
// run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` instead.

export type Account = { id: AccountId, provider: Provider,
/**
 * Where a server Account syncs and who it logs in as; none for the
 * Local Account.
 */
server: ServerLogin | null, };

/**
 * Branded in TypeScript, so that it can't be mixed up with other IDs.
 */
export type AccountId = number & { readonly __brand: "AccountId" };

/**
 * A server Account just added, with the Calendars its server offers.
 */
export type AddedAccount = { account: Account, available: Array<AvailableCalendar>, };

/**
 * Whether the window is light or dark.
 */
export type Appearance = "system" | "light" | "dark";

/**
 * A Calendar a server Account offers that the user hasn't added. FreeCal
 * stores nothing for it; it is fetched from the server each time.
 */
export type AvailableCalendar = {
/**
 * Where it lives on the server, which also names it to the core.
 */
url: string,
/**
 * Its name on the server, untrusted like every name; the last part of
 * its address when the server gives none.
 */
name: string,
/**
 * Its colour on the server, if it has a usable one.
 */
colour: Colour | null, };

export type Calendar = { id: CalendarId, accountId: AccountId, name: string, colour: Colour,
/**
 * Whether the user shows this Calendar's Events or has hidden them.
 */
shown: boolean,
/**
 * A Read-only Calendar: FreeCal never changes its Events. Setting it
 * comes with ticket 17.
 */
readOnly: boolean,
/**
 * Whether its first sync is running, so that its Events are still
 * loading; the sidebar says so. Sync Status may take this over (ticket
 * 15).
 */
firstSync: boolean, };

/**
 * Branded in TypeScript, so that it can't be mixed up with other IDs.
 */
export type CalendarId = number & { readonly __brand: "CalendarId" };

/**
 * What a change to Events returns: its value, and the unit that undoes it,
 * none when it changed nothing.
 */
export type Changed<T> = { unit: UnitId | null, value: T, };

/**
 * A Calendar's colour, always `#rrggbb` in lower case, so that it can only
 * ever be a colour wherever it is used.
 */
export type Colour = string;

/**
 * A failure as it crosses the application interface: a code the frontend
 * translates through its i18n layer, with the names of files it is about.
 * Most codes stand for a [`CoreError`]; the last ones are failures of the
 * shell's own commands.
 */
export type ErrorCode = { "code": "localStoreFailed" } | { "code": "dataDirectoryFailed" } | { "code": "localStoreInUse" } | { "code": "accountNotFound" } | { "code": "calendarNotFound" } | { "code": "calendarReadOnly" } | { "code": "eventNotFound" } | { "code": "endBeforeStart" } | { "code": "invalidRepeat" } | { "code": "occurrenceNotFound" } | { "code": "unreadableEventTime" } | { "code": "emptyCalendarName" } | { "code": "invalidColour" } | { "code": "localAccountCannotBeRemoved" } | { "code": "newerLocalStore" } | { "code": "unknownProvider" } | { "code": "importUnreadable", file: string, } | { "code": "importNotCalendar", file: string, } | { "code": "importZipWithoutCalendars", file: string, } | { "code": "importTooLarge", file: string, } | { "code": "unitNotFound" } | { "code": "changedSinceUnit" } | { "code": "alreadyUndone" } | { "code": "notLocalAccount" } | { "code": "notLocalCalendar" } | { "code": "notServerCalendar" } | { "code": "calendarNotAvailable" } | { "code": "invalidServerUrl" } | { "code": "wrongCredentials" } | { "code": "serverUnreachable" } | { "code": "secureConnectionFailed" } | { "code": "notCalDavServer" } | { "code": "serverRefused", status: number, message: string, } | { "code": "keyringUnavailable" } | { "code": "passwordNotInKeyring" } | { "code": "redirectedElsewhere", url: string, } | { "code": "noFileToImport" } | { "code": "linkNotAllowed" } | { "code": "linkNotOpened" } | { "code": "windowNotShown" };

/**
 * An Event with everything its details and editor show.
 */
export type Event = { id: EventId, calendarId: CalendarId, title: string, when: When, location: StructuredText, description: StructuredText,
/**
 * The zone FreeCal gave the Event when it arrived with a time of day but
 * no time zone (ADR 0007), which its details mention.
 */
pinnedZone: string | null,
/**
 * A Recurring Event's repeat rule as the editor shows it, for an
 * Exception its Recurring Event's; none for an Event that doesn't
 * repeat, and for a rule the editor can't show, such as an hourly one.
 */
repeat: Repeat | null,
/**
 * Whether the repeat rule has more than `repeat` shows, such as the
 * weekdays of a weekly rule. Those are kept while the frequency stays.
 */
customRepeat: boolean,
/**
 * For an Exception, the recurrence ID of the Occurrence of its Recurring
 * Event that it changed; none for every other Event.
 */
recurrenceId: string | null, };

/**
 * What the user gives an Event when creating or editing it.
 */
export type EventDraft = { calendarId: CalendarId, title: string, when: When, location: string,
/**
 * The plain-text description. When it equals the plain-text form of an
 * Event's description, the original is kept byte for byte.
 */
description: string,
/**
 * The repeat rule, none for an Event that doesn't repeat. When it
 * equals an Event's `repeat`, its stored rule is kept as it is.
 */
repeat: Repeat | null, };

/**
 * Branded in TypeScript, so that it can't be mixed up with other IDs.
 */
export type EventId = number & { readonly __brand: "EventId" };

/**
 * What an import does with the Events its Calendar already has (same UID),
 * chosen once per import.
 */
export type ExistingEvents = "update" | "skip" | "copy";

export type Frequency = "daily" | "weekly" | "monthly" | "yearly";

/**
 * What running an import did.
 */
export type ImportOutcome = {
/**
 * What undoes it; none when it changed nothing.
 */
unit: UnitId | null,
/**
 * How many Events it added, copies included.
 */
imported: number, updated: number, skipped: number, };

/**
 * An import preview as the window shows it: the counts, not the Events.
 */
export type ImportSummary = { calendarId: CalendarId, newEvents: number, existingEvents: number, pinned: Array<PinnedZone>, };

/**
 * One dated appearance of an Event in the grid. A single Event has exactly
 * one; Recurring Events get one per date they fall on.
 */
export type Occurrence = {
/**
 * The Event it belongs to: its Recurring Event, or the Exception that
 * changed it.
 */
eventId: EventId, calendarId: CalendarId, title: string, when: When,
/**
 * Which Occurrence of a Recurring Event this is: its original start, as
 * text that only the core reads. None for a single Event.
 */
recurrenceId: string | null, };

/**
 * One piece of a paragraph.
 */
export type Piece = { "kind": "text", text: string, } | { "kind": "link", text: string, url: string, };

/**
 * How many Events of an import had floating times pinned to `zone`.
 */
export type PinnedZone = { zone: string, events: number, };

/**
 * A kind of calendar service FreeCal can talk to.
 */
export type Provider = "local" | "caldav";

/**
 * A repeat rule as the editor sets it.
 */
export type Repeat = { frequency: Frequency,
/**
 * Every how many days, weeks, months or years; at least 1.
 */
interval: number, end: RepeatEnd, };

/**
 * When a Recurring Event stops.
 */
export type RepeatEnd = { "kind": "never" } | { "kind": "until", date: string, } | { "kind": "count", count: number, };

/**
 * Which Occurrences of a Recurring Event a change to one of them changes.
 */
export type Scope = "this" | "thisAndFollowing" | "all";

/**
 * A server Account's server and the username it logs in with. Its
 * password lives in the keyring.
 */
export type ServerLogin = { url: string, username: string, };

/**
 * The settings of the Settings dialog, as the Local Store keeps them.
 */
export type Settings = { appearance: Appearance, };

/**
 * A change to the settings: only the settings it gives change, so that
 * each change in the Settings dialog writes just that one setting.
 */
export type SettingsUpdate = { appearance?: Appearance, };

/**
 * The sidebar as the user left it: shown or hidden, and its width in `rem`,
 * or `None` for the default width. The frontend shows the width within its
 * limits without changing what is stored.
 */
export type SidebarState = { shown: boolean, width: number | null, };

/**
 * A message from the core saying that something changed.
 *
 * Always called Signals, never "events" (calendar Events) or "notifications"
 * (desktop notifications). Signals are fire-and-forget. The v1 Signals are
 * added by the tickets that first send them.
 */
export type Signal = { "kind": "calendarsChanged" } | { "kind": "occurrencesChanged", calendarIds: Array<CalendarId>, } | { "kind": "dateChanged", date: string, };

/**
 * An Event's description or location, ready to show and to edit.
 */
export type StructuredText = {
/**
 * One paragraph per line, for showing the text.
 */
paragraphs: Array<Array<Piece>>,
/**
 * The plain-text form, which the editor edits. A link whose text differs
 * from its URL is written as `text (url)`, so that the URL survives an
 * edit.
 */
text: string, };

/**
 * A Change Journal unit: what undoes a change, or changes made together.
 */
export type UnitId = number & { readonly __brand: "UnitId" };

/**
 * The views the grid offers. FreeCal opens in month view on first launch,
 * then in the last-used one.
 */
export type View = "day" | "week" | "month";

/**
 * When an Event takes place. Times are wall-clock times in the Display Time
 * Zone; ends are exclusive, so a one-day all-day Event ends the next day.
 */
export type When = { "kind": "allDay", start: string, end: string, } | { "kind": "timed", start: string, end: string, };
