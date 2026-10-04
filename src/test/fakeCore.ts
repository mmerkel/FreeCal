import type { CoreApi } from '../core/CoreApi';
import type {
  Account,
  AccountId,
  Calendar,
  CalendarId,
  Description,
  Event,
  EventDraft,
  EventId,
  Signal,
  When,
} from '../core/types';

/**
 * A fake core interface for component tests. Like the real core, it sends
 * "Calendars changed" after every change to a Calendar and "Occurrences
 * changed" after every change to an Event. Tests can also emit Signals
 * themselves.
 */
export class FakeCore implements CoreApi {
  accounts: Account[] = [{ id: 1, provider: 'local' }];
  calendars: Calendar[] = [];
  events: Event[] = [];
  currentDate = Temporal.PlainDate.from('2026-10-03');
  /** The names of the interface methods called, in order. */
  calls: string[] = [];
  /** The links opened in the browser. */
  openedLinks: string[] = [];
  private subscribers = new Set<(signal: Signal) => void>();
  private nextCalendarId = 1;
  private nextEventId = 1;

  /** Adds a Calendar directly, as if it had been created earlier, without a Signal. */
  addCalendar(calendar: Partial<Calendar> & Pick<Calendar, 'name'>): Calendar {
    const added: Calendar = {
      id: this.nextCalendarId++,
      accountId: 1,
      colour: '#3366cc',
      shown: true,
      readOnly: false,
      ...calendar,
    };
    this.calendars.push(added);
    return added;
  }

  /**
   * Adds an Event directly, as if it had been created earlier, without a
   * Signal. Its description is plain text unless the test gives the
   * structured content the core would build.
   */
  addEvent(
    event: Partial<Omit<Event, 'description'>> &
      Pick<Event, 'calendarId' | 'when'> & { description?: Description | string },
  ): Event {
    const { description = '', ...fields } = event;
    const added: Event = {
      id: this.nextEventId++,
      title: '',
      location: '',
      ...fields,
      description: typeof description === 'string' ? plainDescription(description) : description,
    };
    this.events.push(added);
    return added;
  }

  async subscribe(onSignal: (signal: Signal) => void) {
    this.calls.push('subscribe');
    this.subscribers.add(onSignal);
    return () => {
      this.subscribers.delete(onSignal);
    };
  }

  emit(signal: Signal) {
    for (const subscriber of this.subscribers) subscriber(signal);
  }

  async today() {
    this.calls.push('today');
    return this.currentDate;
  }

  async listAccounts() {
    this.calls.push('listAccounts');
    return structuredClone(this.accounts);
  }

  async listCalendars() {
    this.calls.push('listCalendars');
    return structuredClone(this.calendars);
  }

  async createCalendar(accountId: AccountId, name: string, colour: string) {
    this.calls.push('createCalendar');
    const created = this.addCalendar({ accountId, name: name.trim(), colour });
    this.emit({ kind: 'calendarsChanged' });
    return structuredClone(created);
  }

  async renameCalendar(id: CalendarId, name: string) {
    this.calls.push('renameCalendar');
    this.change(id, { name: name.trim() });
  }

  async recolourCalendar(id: CalendarId, colour: string) {
    this.calls.push('recolourCalendar');
    this.change(id, { colour });
  }

  async deleteCalendar(id: CalendarId) {
    this.calls.push('deleteCalendar');
    this.calendar(id);
    this.calendars = this.calendars.filter((calendar) => calendar.id !== id);
    this.events = this.events.filter((event) => event.calendarId !== id);
    this.emit({ kind: 'calendarsChanged' });
  }

  async setCalendarShown(id: CalendarId, shown: boolean) {
    this.calls.push('setCalendarShown');
    this.change(id, { shown });
  }

  async showOnlyCalendar(id: CalendarId) {
    this.calls.push('showOnlyCalendar');
    this.calendar(id);
    for (const calendar of this.calendars) calendar.shown = calendar.id === id;
    this.emit({ kind: 'calendarsChanged' });
  }

  async listOccurrences(from: Temporal.PlainDate, to: Temporal.PlainDate) {
    this.calls.push('listOccurrences');
    const shown = new Set(this.calendars.filter((c) => c.shown).map((c) => c.id));
    return this.events
      .filter((event) => shown.has(event.calendarId) && overlaps(event.when, from, to))
      .map((event) => ({
        eventId: event.id,
        calendarId: event.calendarId,
        title: event.title,
        when: { ...event.when },
      }));
  }

  async event(id: EventId) {
    this.calls.push('event');
    return structuredClone(this.findEvent(id));
  }

  async createEvent(draft: EventDraft) {
    this.calls.push('createEvent');
    this.writable(draft.calendarId);
    const created = this.addEvent({ ...draft, description: draft.description });
    this.emit({ kind: 'occurrencesChanged', calendarIds: [draft.calendarId] });
    return structuredClone(created);
  }

  async editEvent(id: EventId, draft: EventDraft) {
    this.calls.push('editEvent');
    const event = this.findEvent(id);
    const before = event.calendarId;
    this.writable(before);
    this.writable(draft.calendarId);
    Object.assign(event, {
      ...draft,
      description:
        draft.description === event.description.text
          ? event.description
          : plainDescription(draft.description),
    });
    this.emit({
      kind: 'occurrencesChanged',
      calendarIds: before === draft.calendarId ? [before] : [before, draft.calendarId],
    });
    return structuredClone(event);
  }

  async deleteEvent(id: EventId) {
    this.calls.push('deleteEvent');
    const { calendarId } = this.findEvent(id);
    this.writable(calendarId);
    this.events = this.events.filter((event) => event.id !== id);
    this.emit({ kind: 'occurrencesChanged', calendarIds: [calendarId] });
  }

  async openLink(url: string) {
    this.calls.push('openLink');
    this.openedLinks.push(url);
  }

  private findEvent(id: EventId): Event {
    const found = this.events.find((event) => event.id === id);
    if (!found) throw new Error(`there is no Event ${id}`);
    return found;
  }

  private writable(id: CalendarId) {
    if (this.calendar(id).readOnly) throw new Error(`Calendar ${id} is read-only`);
  }

  private calendar(id: CalendarId): Calendar {
    const found = this.calendars.find((calendar) => calendar.id === id);
    if (!found) throw new Error(`there is no Calendar ${id}`);
    return found;
  }

  private change(id: CalendarId, fields: Partial<Calendar>) {
    Object.assign(this.calendar(id), fields);
    this.emit({ kind: 'calendarsChanged' });
  }
}

/** A plain-text description as the core would structure it, without links. */
function plainDescription(text: string): Description {
  return {
    paragraphs:
      text === ''
        ? []
        : text.split('\n').map((line) => (line ? [{ kind: 'text', text: line }] : [])),
    text,
  };
}

function overlaps(when: When, from: Temporal.PlainDate, to: Temporal.PlainDate): boolean {
  if (when.kind === 'allDay') {
    return (
      Temporal.PlainDate.compare(when.start, to) < 0 &&
      Temporal.PlainDate.compare(when.end, from) > 0
    );
  }
  const start = Temporal.PlainDateTime.from(when.start);
  const end = Temporal.PlainDateTime.from(when.end);
  return (
    Temporal.PlainDateTime.compare(start, to.toPlainDateTime()) < 0 &&
    Temporal.PlainDateTime.compare(end, from.toPlainDateTime()) > 0
  );
}
