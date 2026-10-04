import type { CoreApi } from '../core/CoreApi';
import type { Account, AccountId, Calendar, CalendarId, Signal } from '../core/types';

/**
 * A fake core interface for component tests. Like the real core, it sends
 * "Calendars changed" after every change to a Calendar. Tests can also emit
 * Signals themselves.
 */
export class FakeCore implements CoreApi {
  accounts: Account[] = [{ id: 1, provider: 'local' }];
  calendars: Calendar[] = [];
  currentDate = Temporal.PlainDate.from('2026-10-03');
  /** The names of the interface methods called, in order. */
  calls: string[] = [];
  private subscribers = new Set<(signal: Signal) => void>();
  private nextCalendarId = 1;

  /** Adds a Calendar directly, as if it had been created earlier, without a Signal. */
  addCalendar(calendar: Partial<Calendar> & Pick<Calendar, 'name'>): Calendar {
    const added: Calendar = {
      id: this.nextCalendarId++,
      accountId: 1,
      colour: '#3366cc',
      shown: true,
      ...calendar,
    };
    this.calendars.push(added);
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
