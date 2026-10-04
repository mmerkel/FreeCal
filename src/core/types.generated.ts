// Generated from the Rust core by core/src/typescript.rs. Don't edit;
// run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` instead.

export type AccountId = number;

/**
 * A kind of calendar service FreeCal can talk to.
 */
export type Provider = "local";

export type Account = { id: AccountId, provider: Provider, };

export type CalendarId = number;

/**
 * A Calendar's colour, always `#rrggbb` in lower case, so that it can only
 * ever be a colour wherever it is used.
 */
export type Colour = string;

export type Calendar = { id: CalendarId, accountId: AccountId, name: string, colour: Colour,
/**
 * Whether the user shows this Calendar's Events or has hidden them.
 */
shown: boolean,
/**
 * A Read-only Calendar: FreeCal never changes its Events. Setting it
 * comes with ticket 17.
 */
readOnly: boolean, };

export type EventId = number;

/**
 * When an Event takes place. Times are wall-clock times in the Display Time
 * Zone; ends are exclusive, so a one-day all-day Event ends the next day.
 */
export type When = { "kind": "allDay", start: string, end: string, } | { "kind": "timed", start: string, end: string, };

/**
 * What the user gives an Event when creating or editing it.
 */
export type EventDraft = { calendarId: CalendarId, title: string, when: When, location: string,
/**
 * The plain-text description. When it equals the plain-text form of an
 * Event's description, the original is kept byte for byte.
 */
description: string, };

/**
 * An Event with everything its details and editor show.
 */
export type Event = { id: EventId, calendarId: CalendarId, title: string, when: When, location: string, description: Description, };

/**
 * One dated appearance of an Event in the grid. A single Event has exactly
 * one; Recurring Events get one per date (ticket 07).
 */
export type Occurrence = { eventId: EventId, calendarId: CalendarId, title: string, when: When, };

/**
 * An Event's description, ready to show and to edit.
 */
export type Description = {
/**
 * One paragraph per line, for showing the description.
 */
paragraphs: Array<Array<Piece>>,
/**
 * The plain-text form, which the editor edits. A link whose text differs
 * from its URL is written as `text (url)`, so that the URL survives an
 * edit.
 */
text: string, };

/**
 * One piece of a paragraph.
 */
export type Piece = { "kind": "text", text: string, } | { "kind": "link", text: string, url: string, };

/**
 * A message from the core saying that something changed.
 *
 * Always called Signals, never "events" (calendar Events) or "notifications"
 * (desktop notifications). Signals are fire-and-forget. The v1 Signals are
 * added by the tickets that first send them.
 */
export type Signal = { "kind": "calendarsChanged" } | { "kind": "occurrencesChanged", calendarIds: Array<CalendarId>, };
