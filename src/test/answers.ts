// Scripted Answers for the tests of the whole window (`App.svelte`): a
// RecordingCore scripted as a first launch, and builders for the core's types.
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
import { RecordingCore } from './recordingCore';

/** The Local Account, as every Local Store has it. */
export const localAccount: Account = { id: accountId(1), provider: 'local', server: null };

/** The Sync Statuses with no server Account. */
export const noSyncStatuses: SyncStatuses = {
  overall: { state: { kind: 'synced' }, lastSync: null },
  accounts: [],
  calendars: [],
};

/**
 * A RecordingCore answering as on a first launch on Saturday, October 3,
 * 2026: month view, the sidebar shown at its default width, the default
 * settings, and only the Local Account, with no Calendars and no Events.
 */
export function firstLaunch(): RecordingCore {
  const core = new RecordingCore();
  core.answer('today', Temporal.PlainDate.from('2026-10-03'));
  core.answer('lastUsedView', 'month');
  core.answer('sidebar', { shown: true, width: null });
  core.answer('settings', { ...DEFAULT_SETTINGS });
  core.answer('listAccounts', [localAccount]);
  core.answer('listCalendars', []);
  core.answer('syncStatuses', noSyncStatuses);
  core.answer('listOccurrences', []);
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
    ...rest,
    location: typeof location === 'string' ? plainText(location) : location,
    description: typeof description === 'string' ? plainText(description) : description,
  };
}

/** Text without links, as the core structures it: one paragraph per line. */
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
    ...fields,
  };
}

/** Scripts the Calendars and the Occurrences of `events` for the next reads. */
export function scriptWindow(
  core: RecordingCore,
  { calendars = [], events = [] }: { calendars?: Calendar[]; events?: Event[] },
) {
  core.answer('listCalendars', calendars);
  core.answer(
    'listOccurrences',
    events.map((each) => occurrence(each)),
  );
}
