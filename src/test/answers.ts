// Scripted Answers for the tests of the whole window (`App.svelte`): a
// FakeCore scripted as a first launch, and builders for the core's types.
// The builders only fill in fields the test leaves out; the test gives
// everything the core would decide, such as an Occurrence's time or an
// Event's structured text.
import type {
  Account,
  AccountId,
  Calendar,
  Event,
  Occurrence,
  StructuredText,
  SyncStatuses,
} from '../core/types';
import { DEFAULT_SETTINGS } from '../lib/settings';
import { accountId, calendarId, eventId } from './ids';
import { FakeCore } from './fakeCore';

/** The Local Account, as every Local Store has it. */
export const localAccount: Account = { id: accountId(1), provider: 'local', server: null };

/** The Sync Statuses with no server Account. */
const noSyncStatuses: SyncStatuses = {
  overall: { state: { kind: 'synced' }, lastSync: null },
  accounts: [],
  calendars: [],
};

/**
 * What the window reads of the Local Store: the server Accounts besides the
 * Local Account, the Calendars, and the Events or Occurrences of the visible
 * range. An Event stands for its Occurrence at its own time.
 */
export interface ScriptedWindow {
  accounts?: Account[];
  calendars?: Calendar[];
  events?: (Event | Occurrence)[];
}

/**
 * A FakeCore answering as on a first launch on Saturday, October 3,
 * 2026: month view, the sidebar shown at its default width, the default
 * settings, and only the Local Account, with no Calendars and no Events,
 * unless `window` gives them.
 */
export function firstLaunch(window: ScriptedWindow = {}): FakeCore {
  const core = new FakeCore();
  core.answer('today', Temporal.PlainDate.from('2026-10-03'));
  core.answer('lastUsedView', 'month');
  core.answer('sidebar', { shown: true, width: null });
  core.answer('settings', { ...DEFAULT_SETTINGS });
  core.answer('listAccounts', [localAccount]);
  core.answer('listCalendars', []);
  core.answer('syncStatuses', noSyncStatuses);
  core.answer('listOccurrences', []);
  scriptWindow(core, window);
  return core;
}

let nextId = 100;

/** A writable, shown Calendar of the Local Account, with `fields`. */
export function calendar(fields: Partial<Calendar> & Pick<Calendar, 'name'>): Calendar {
  return {
    id: calendarId(nextId++),
    accountId: localAccount.id,
    colour: '#3366cc',
    shown: true,
    readOnly: false,
    serverReadOnly: false,
    ...fields,
  };
}

/** A CalDAV Account of `username` on dav.example.org. */
export function caldavAccount(username: string, id: AccountId = accountId(nextId++)): Account {
  return { id, provider: 'caldav', server: { url: 'https://dav.example.org/', username } };
}

/**
 * A single Event with `fields`. Its location and description are one line of
 * plain text unless the test gives the structured text the core would build.
 */
export function event(
  fields: Partial<Omit<Event, 'location' | 'description'>> &
    Pick<Event, 'calendarId' | 'title' | 'when'> & {
      location?: StructuredText | string;
      description?: StructuredText | string;
    },
): Event {
  const { location = '', description = '', ...rest } = fields;
  return {
    id: eventId(nextId++),
    pinnedZone: null,
    repeat: null,
    customRepeat: false,
    recurrenceId: null,
    hasGuests: false,
    ...rest,
    location: typeof location === 'string' ? plainText(location) : location,
    description: typeof description === 'string' ? plainText(description) : description,
  };
}

/** Structured text with no links: each line a paragraph of one text piece. */
export function plainText(text: string): StructuredText {
  return {
    paragraphs:
      text === ''
        ? []
        : text.split('\n').map((line) => (line ? [{ kind: 'text', text: line }] : [])),
    text,
  };
}

/**
 * The Occurrence of `of` at its own time, or at `fields.when` with the
 * recurrence ID the test gives for an Occurrence of a Recurring Event.
 */
export function occurrence(of: Event, fields: Partial<Occurrence> = {}): Occurrence {
  return {
    eventId: of.id,
    calendarId: of.calendarId,
    title: of.title,
    when: of.when,
    recurrenceId: of.recurrenceId,
    hasGuests: of.hasGuests,
    ...fields,
  };
}

/** Scripts what `window` gives for the next reads, and leaves the rest as it was scripted. */
export function scriptWindow(core: FakeCore, { accounts, calendars, events }: ScriptedWindow) {
  if (accounts) core.answer('listAccounts', [localAccount, ...accounts]);
  if (calendars) core.answer('listCalendars', calendars);
  if (events) {
    core.answer(
      'listOccurrences',
      events.map((shown) => ('eventId' in shown ? shown : occurrence(shown))),
    );
  }
}

/** Emits "Date changed" to `date`, as the core does at midnight. */
export function dateChanged(core: FakeCore, date: string) {
  core.emit({ kind: 'dateChanged', date });
}

/** Scripts `window` and emits "Calendars changed", as the core does after a change to them. */
export function calendarsChanged(core: FakeCore, window: ScriptedWindow) {
  scriptWindow(core, window);
  core.emit({ kind: 'calendarsChanged' });
}

/**
 * Scripts the Occurrences of `events` and emits "Occurrences changed" for
 * `calendars`, as the core does after a change to Events.
 */
export function occurrencesChanged(
  core: FakeCore,
  calendars: Calendar[],
  events: ScriptedWindow['events'],
) {
  scriptWindow(core, { events });
  core.emit({ kind: 'occurrencesChanged', calendarIds: calendars.map((each) => each.id) });
}
