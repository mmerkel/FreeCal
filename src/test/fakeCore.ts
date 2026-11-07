import type { CoreApi } from '../core/CoreApi';
import type {
  Account,
  AccountId,
  AvailableCalendar,
  Calendar,
  CalendarId,
  Event,
  ErrorCode,
  EventDraft,
  EventId,
  ExceptionCounts,
  ExistingEvents,
  ImportOutcome,
  Occurrence,
  PinnedZone,
  Scope,
  Settings,
  SettingsUpdate,
  SidebarState,
  Signal,
  SyncStatusReport,
  SyncStatuses,
  StructuredText,
  JournalTransactionId,
  View,
  When,
} from '../core/types';
import { DEFAULT_SETTINGS } from '../lib/settings';
import { accountId, calendarId, eventId, journalTransactionId } from './ids';

/** A file a test lets the user pick to import. */
export interface FakeImportFile {
  name: string;
  /** Its Events, each with its UID. */
  events: (Pick<Event, 'title' | 'when'> & { uid: string })[];
  /** The zones its floating times were pinned to. */
  pinned?: PinnedZone[];
}

/** A Journal Event Modification: one Event's change in a Journal Transaction, its Event Versions before and after, none when it was created or deleted. */
interface JournalEventModification {
  id: EventId;
  uid: string;
  before?: Event;
  after?: Event;
}

/** A Journal Transaction: changes made together, which are undone together. */
interface JournalTransaction {
  journalEventModifications: JournalEventModification[];
  undone: boolean;
}

/** A change to an Occurrence in a recurrence scope: a draft from the editor, or a drag's new time. */
export interface ScopedChange {
  id: EventId;
  recurrenceId: string;
  scope: Scope;
  exceptionsToo: boolean;
  draft?: EventDraft;
  when?: When;
}

/**
 * A fake core interface for component tests. Like the real core, it sends
 * "Calendars changed" after every change to a Calendar, and records every
 * change to Events as a Journal Transaction and sends "Occurrences changed"
 * for it. Tests can also emit Signals themselves.
 */
export class FakeCore implements CoreApi {
  accounts: Account[] = [{ id: accountId(1), provider: 'local', server: null }];
  calendars: Calendar[] = [];
  events: Event[] = [];
  currentDate = Temporal.PlainDate.from('2026-10-03');
  lastView: View = 'month';
  /** The sidebar as the Local Store keeps it. */
  storedSidebar: SidebarState = { shown: true, width: null };
  /** The sidebar states written with `setSidebar`, in order. */
  sidebarSaves: SidebarState[] = [];
  /** The settings as the Local Store keeps them. */
  storedSettings: Settings = { ...DEFAULT_SETTINGS };
  /** The updates written with `updateSettings`, in order. */
  settingsUpdates: SettingsUpdate[] = [];
  /**
   * Set to make the core fail to start, as when it can't open the Local
   * Store: `startupFailure` returns it and every read rejects with it.
   */
  startupError?: ErrorCode;
  /** Whether the window has been shown; it starts hidden. */
  windowShown = false;
  hasQuit = false;
  /** The names of the interface methods called, in order. */
  calls: string[] = [];
  /** The links opened in the browser. */
  openedLinks: string[] = [];
  /**
   * What the next file dialog gives: a file, `null` as when the user
   * cancels, or the failure reading the file.
   */
  importFile: FakeImportFile | ErrorCode | null = null;
  /** The file the user picked and the shell holds, if any. */
  pickedImport?: FakeImportFile;
  /** The title and filter name the file dialogs were opened with. */
  fileDialogs: { title: string; filter: string }[] = [];
  /** Each Event's UID. */
  uids = new Map<EventId, string>();
  /** The recurrence IDs of each Recurring Event's cancelled Occurrences. */
  cancelled = new Map<EventId, string[]>();
  /** The changes made to Occurrences in a recurrence scope, in order. */
  scopedChanges: ScopedChange[] = [];
  /** What `exceptionCounts` gives for every Occurrence. */
  givenExceptionCounts: ExceptionCounts = { thisAndFollowing: 0, all: 0 };
  /**
   * What the server of the next CalDAV Account added offers, or the failure
   * adding it gives.
   */
  caldavServer: AvailableCalendar[] | ErrorCode = [];
  /** Every Calendar each server Account's server offers, added or not. */
  offered = new Map<AccountId, AvailableCalendar[]>();
  /** Where each server Calendar lives on its server. */
  calendarUrls = new Map<CalendarId, string>();
  /** The password the keyring holds for each server Account. */
  keyring = new Map<AccountId, string>();
  /**
   * The Sync Status of every server Account and Calendar, as
   * `syncStatuses` gives it. Tests change it with `reportAccount`,
   * `reportCalendar` and `setOverall`.
   */
  statuses: SyncStatuses = {
    overall: { state: { kind: 'synced' }, lastSync: null },
    accounts: [],
    calendars: [],
  };
  /** Each sync now asked for: an Account's ID, or `null` for every server Account. */
  syncedNow: (AccountId | null)[] = [];
  /** While set, first syncs run until it resolves. */
  private heldFirstSyncs?: Promise<void>;
  private journalTransactions = new Map<JournalTransactionId, JournalTransaction>();
  private nextJournalTransactionId = 1;
  private subscribers = new Set<(signal: Signal) => void>();
  private nextCalendarId = 1;
  private nextAccountId = 2;
  private nextEventId = 1;

  /** Adds a Calendar directly, as if it had been created earlier, without a Signal. */
  addCalendar(calendar: Partial<Calendar> & Pick<Calendar, 'name'>): Calendar {
    const added: Calendar = {
      id: calendarId(this.nextCalendarId++),
      accountId: accountId(1),
      colour: '#3366cc',
      shown: true,
      readOnly: false,
      serverReadOnly: false,
      ...calendar,
    };
    this.calendars.push(added);
    return added;
  }

  /**
   * Adds an Event directly, as if it had been created earlier, without a
   * Signal. Its location and description are plain text unless the test gives
   * the structured content the core would build.
   */
  addEvent(
    event: Partial<Omit<Event, 'location' | 'description'>> &
      Pick<Event, 'calendarId' | 'when'> & {
        location?: StructuredText | string;
        description?: StructuredText | string;
        uid?: string;
      },
  ): Event {
    const { location = '', description = '', uid, ...fields } = event;
    const structured = (text: StructuredText | string) =>
      typeof text === 'string' ? plainText(text) : text;
    const added: Event = {
      id: eventId(this.nextEventId++),
      title: '',
      pinnedZone: null,
      repeat: null,
      customRepeat: false,
      recurrenceId: null,
      ...fields,
      location: structured(location),
      description: structured(description),
    };
    this.events.push(added);
    this.uids.set(added.id, uid ?? `event-${added.id}`);
    return added;
  }

  async startupFailure() {
    this.calls.push('startupFailure');
    return this.startupError ?? null;
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

  /** Gives a server Account a new Sync Status Report and sends "Sync status changed" for it. */
  reportAccount(id: AccountId, report: SyncStatusReport) {
    this.statuses.accounts = [
      ...this.statuses.accounts.filter((account) => account.accountId !== id),
      { accountId: id, report },
    ];
    this.emit({ kind: 'accountSyncStatusChanged', accountId: id, report: structuredClone(report) });
  }

  /**
   * Gives a server Calendar a new Sync Status Report, with whether its First
   * Sync is unfinished, and sends "Sync status changed" for it.
   */
  reportCalendar(id: CalendarId, report: SyncStatusReport, firstSyncUnfinished = false) {
    this.statuses.calendars = [
      ...this.statuses.calendars.filter((calendar) => calendar.calendarId !== id),
      { calendarId: id, report, firstSyncUnfinished },
    ];
    this.emit({
      kind: 'calendarSyncStatusChanged',
      calendarId: id,
      report: structuredClone(report),
      firstSyncUnfinished,
    });
  }

  /** Sets the overall status, without a Signal: the next "Sync status changed" comes with it. */
  setOverall(overall: SyncStatuses['overall']) {
    this.statuses.overall = overall;
  }

  async syncStatuses() {
    this.calls.push('syncStatuses');
    this.started();
    return structuredClone(this.statuses);
  }

  async syncNow(accountId: AccountId | null) {
    this.calls.push('syncNow');
    this.syncedNow.push(accountId);
  }

  async today() {
    this.calls.push('today');
    this.started();
    return this.currentDate;
  }

  /** Moves the local date on, as at midnight, and sends "Date changed" like the core. */
  changeDate(date: string) {
    this.currentDate = Temporal.PlainDate.from(date);
    this.emit({ kind: 'dateChanged', date });
  }

  async lastUsedView() {
    this.calls.push('lastUsedView');
    this.started();
    return this.lastView;
  }

  async setLastUsedView(view: View) {
    this.calls.push('setLastUsedView');
    this.lastView = view;
  }

  async sidebar() {
    this.calls.push('sidebar');
    this.started();
    return { ...this.storedSidebar };
  }

  async setSidebar(sidebar: SidebarState) {
    this.calls.push('setSidebar');
    this.sidebarSaves.push({ ...sidebar });
    this.storedSidebar = { ...sidebar };
  }

  async settings() {
    this.calls.push('settings');
    this.started();
    return { ...this.storedSettings };
  }

  async updateSettings(update: SettingsUpdate) {
    this.calls.push('updateSettings');
    this.settingsUpdates.push({ ...update });
    this.storedSettings = { ...this.storedSettings, ...update };
    return { ...this.storedSettings };
  }

  async listAccounts() {
    this.calls.push('listAccounts');
    this.started();
    return structuredClone(this.accounts);
  }

  /**
   * Adds a CalDAV Account directly, as if it had been added earlier, whose
   * server offers `offered`, without a Signal.
   */
  addCalDavAccountNow(username: string, offered: AvailableCalendar[] = []): Account {
    const added: Account = {
      id: accountId(this.nextAccountId++),
      provider: 'caldav',
      server: { url: 'https://dav.example.org/', username },
    };
    this.accounts.push(added);
    this.offered.set(added.id, structuredClone(offered));
    this.keyring.set(added.id, 'secret');
    return added;
  }

  /**
   * Adds an offered Calendar of a server Account directly, as if it had been
   * added earlier, without a Signal.
   */
  addServerCalendar(id: AccountId, url: string, fields: Partial<Calendar> = {}): Calendar {
    const available = this.offered.get(id)!.find((calendar) => calendar.url === url)!;
    const added = this.addCalendar({
      accountId: id,
      name: available.name,
      colour: available.colour ?? '#808080',
      // Every newly added server Calendar, until 1.0.
      readOnly: true,
      serverReadOnly: available.readOnly,
      ...fields,
    });
    this.calendarUrls.set(added.id, url);
    return added;
  }

  /** Makes first syncs run until the returned function ends them. */
  holdFirstSyncs(): () => void {
    let release!: () => void;
    this.heldFirstSyncs = new Promise((resolve) => (release = resolve));
    return () => {
      this.heldFirstSyncs = undefined;
      release();
    };
  }

  async addCalDavAccount(url: string, username: string, password: string) {
    this.calls.push('addCalDavAccount');
    if (!Array.isArray(this.caldavServer)) throw this.caldavServer;
    const account = this.addCalDavAccountNow(username, this.caldavServer);
    account.server = { url, username };
    this.keyring.set(account.id, password);
    this.emit({ kind: 'calendarsChanged' });
    return { account: structuredClone(account), available: structuredClone(this.caldavServer) };
  }

  async availableCalendars(id: AccountId) {
    this.calls.push('availableCalendars');
    const added = new Set(this.calendarUrls.values());
    return structuredClone(this.serverOf(id)?.filter((calendar) => !added.has(calendar.url)) ?? []);
  }

  async addCalendars(id: AccountId, urls: string[]) {
    this.calls.push('addCalendars');
    const taken = new Set(this.calendarUrls.values());
    const available = urls.map((url) => {
      const offered = this.serverOf(id)?.find((calendar) => calendar.url === url);
      if (!offered || taken.has(url)) throw failure('calendarNotAvailable');
      taken.add(url);
      return offered;
    });
    const added = available.map((calendar) => this.addServerCalendar(id, calendar.url));
    this.emit({ kind: 'calendarsChanged' });
    const report = (status: SyncStatusReport['status']) => ({
      status,
      lastSync: null,
      unsentChanges: 0,
    });
    for (const calendar of added) {
      this.reportCalendar(calendar.id, report({ kind: 'syncing', byUser: true }), true);
    }
    await this.heldFirstSyncs;
    for (const calendar of added) this.reportCalendar(calendar.id, report({ kind: 'synced' }));
    return structuredClone(added);
  }

  async removeAccount(id: AccountId) {
    this.calls.push('removeAccount');
    if (!this.serverOf(id)) throw failure('localAccountCannotBeRemoved');
    for (const calendar of this.calendars.filter((calendar) => calendar.accountId === id)) {
      this.dropCalendar(calendar.id);
    }
    this.accounts = this.accounts.filter((account) => account.id !== id);
    this.offered.delete(id);
    this.keyring.delete(id);
    this.emit({ kind: 'calendarsChanged' });
  }

  async listCalendars() {
    this.calls.push('listCalendars');
    this.started();
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
    if (this.calendarUrls.has(this.calendar(id).id)) throw failure('notLocalCalendar');
    this.dropCalendar(id);
    this.emit({ kind: 'calendarsChanged' });
  }

  async removeCalendar(id: CalendarId) {
    this.calls.push('removeCalendar');
    if (!this.calendarUrls.has(this.calendar(id).id)) throw failure('notServerCalendar');
    this.dropCalendar(id);
    this.emit({ kind: 'calendarsChanged' });
  }

  /**
   * Deletes a Calendar's local copy. Like the real core, its Events leave
   * the Change Journal with it, and so does each Event's history up to its
   * last Event Version in it.
   */
  private dropCalendar(id: CalendarId) {
    this.calendarUrls.delete(id);
    this.statuses.calendars = this.statuses.calendars.filter(
      (calendar) => calendar.calendarId !== id,
    );
    this.calendars = this.calendars.filter((calendar) => calendar.id !== id);
    const gone = new Set(
      this.events.filter((event) => event.calendarId === id).map((event) => event.id),
    );
    this.events = this.events.filter((event) => !gone.has(event.id));
    const journalTransactions = [...this.journalTransactions];
    const lastBack = new Map<EventId, number>();
    journalTransactions.forEach(([, { journalEventModifications: modifications }], index) => {
      for (const modification of modifications) {
        if (modification.before?.calendarId === id) lastBack.set(modification.id, index);
      }
    });
    for (const [
      index,
      [journalTransaction, { journalEventModifications: modifications }],
    ] of journalTransactions.entries()) {
      const kept = modifications.filter(
        (modification) =>
          !gone.has(modification.id) && index > (lastBack.get(modification.id) ?? -1),
      );
      if (kept.length === 0) this.journalTransactions.delete(journalTransaction);
      else this.journalTransactions.get(journalTransaction)!.journalEventModifications = kept;
    }
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
      .filter((event) => shown.has(event.calendarId))
      .flatMap((event) => this.occurrencesOf(event))
      .filter((occurrence) => overlaps(occurrence.when, from, to));
  }

  /**
   * An Event's Occurrences. The fake stands in for the core, which expands
   * repeat rules: it repeats the first one by the rule's frequency and
   * interval, at most 400 times.
   */
  private occurrencesOf(event: Event): Occurrence[] {
    const occurrence = (when: When, recurrenceId: string | null): Occurrence => ({
      eventId: event.id,
      calendarId: event.calendarId,
      title: event.title,
      when,
      recurrenceId,
    });
    if (!event.repeat) return [occurrence({ ...event.when }, event.recurrenceId)];
    const { frequency, interval, end } = event.repeat;
    const unit = ({ daily: 'days', weekly: 'weeks', monthly: 'months', yearly: 'years' } as const)[
      frequency
    ];
    const cancelled = this.cancelled.get(event.id) ?? [];
    const found: Occurrence[] = [];
    for (let index = 0; index < 400; index++) {
      if (end.kind === 'count' && index >= end.count) break;
      const when = shifted(event.when, { [unit]: index * interval });
      if (end.kind === 'until' && when.start.slice(0, 10) > end.date) break;
      if (!cancelled.includes(when.start)) found.push(occurrence(when, when.start));
    }
    return found;
  }

  async event(id: EventId) {
    this.calls.push('event');
    return structuredClone(this.findEvent(id));
  }

  async createEvent(draft: EventDraft) {
    this.calls.push('createEvent');
    this.writable(draft.calendarId);
    const created = this.addEvent({ ...draft, description: draft.description });
    const journalTransaction = this.record([
      this.journalEventModification(created.id, undefined, created),
    ]);
    return { journalTransaction, value: structuredClone(created) };
  }

  async editEvent(id: EventId, draft: EventDraft) {
    this.calls.push('editEvent');
    const event = this.findEvent(id);
    const before = structuredClone(event);
    this.writable(before.calendarId);
    this.writable(draft.calendarId);
    Object.assign(event, {
      ...draft,
      location: plainText(draft.location),
      description:
        draft.description === event.description.text
          ? event.description
          : plainText(draft.description),
      pinnedZone: keptPinnedZone(event, draft.when),
    });
    const journalTransaction = this.record([this.journalEventModification(id, before, event)]);
    return { journalTransaction, value: structuredClone(event) };
  }

  async rescheduleEvent(id: EventId, when: When) {
    this.calls.push('rescheduleEvent');
    const event = this.findEvent(id);
    const before = structuredClone(event);
    this.writable(event.calendarId);
    event.pinnedZone = keptPinnedZone(event, when);
    event.when = { ...when };
    const journalTransaction = this.record([this.journalEventModification(id, before, event)]);
    return { journalTransaction, value: structuredClone(event) };
  }

  /**
   * Like the core, "this event" makes the Occurrence an Exception and "all
   * events" changes the Recurring Event, here without moving its times. The
   * fake doesn't split for "this and following"; the test reads what was
   * asked in `scopedChanges`. Nothing of this is in the fake's Change Journal.
   */
  async editOccurrence(
    id: EventId,
    recurrenceId: string,
    scope: Scope,
    exceptionsToo: boolean,
    draft: EventDraft,
  ) {
    this.calls.push('editOccurrence');
    this.scopedChanges.push({
      id,
      recurrenceId,
      scope,
      exceptionsToo,
      draft: structuredClone(draft),
    });
    return this.changeOccurrence(id, recurrenceId, scope, draft);
  }

  async rescheduleOccurrence(
    id: EventId,
    recurrenceId: string,
    scope: Scope,
    exceptionsToo: boolean,
    when: When,
  ) {
    this.calls.push('rescheduleOccurrence');
    this.scopedChanges.push({ id, recurrenceId, scope, exceptionsToo, when: { ...when } });
    const event = this.findEvent(id);
    const draft: EventDraft = {
      calendarId: event.calendarId,
      title: event.title,
      when,
      location: event.location.text,
      description: event.description.text,
      repeat: event.repeat,
    };
    return this.changeOccurrence(id, recurrenceId, scope, draft);
  }

  /** Gives `givenExceptionCounts`; the fake keeps no Exceptions of their own to count. */
  async exceptionCounts(id: EventId) {
    this.calls.push('exceptionCounts');
    this.findEvent(id);
    return { ...this.givenExceptionCounts };
  }

  private changeOccurrence(
    id: EventId,
    recurrenceId: string,
    scope: Scope,
    draft: EventDraft,
  ): { journalTransaction: null; value: Occurrence } {
    const event = this.findEvent(id);
    this.writable(event.calendarId);
    const fields = {
      calendarId: draft.calendarId,
      title: draft.title,
      location: plainText(draft.location),
      description: plainText(draft.description),
    };
    let changed = event;
    if (scope === 'this' && event.recurrenceId === null) {
      this.cancelled.set(id, [...(this.cancelled.get(id) ?? []), recurrenceId]);
      changed = this.addEvent({
        ...fields,
        when: draft.when,
        recurrenceId,
        uid: this.uids.get(id),
      });
    } else {
      Object.assign(
        event,
        fields,
        scope === 'all' ? { repeat: draft.repeat } : { when: draft.when },
      );
    }
    this.emit({ kind: 'occurrencesChanged', calendarIds: [event.calendarId, draft.calendarId] });
    const value: Occurrence = {
      eventId: changed.id,
      calendarId: changed.calendarId,
      title: changed.title,
      when: draft.when,
      recurrenceId: changed.recurrenceId ?? recurrenceId,
    };
    return { journalTransaction: null, value };
  }

  /** Like the core, deleting a Recurring Event or an Exception deletes all Events of its UID. */
  async deleteEvent(id: EventId) {
    this.calls.push('deleteEvent');
    const deleted = this.findEvent(id);
    this.writable(deleted.calendarId);
    const recurring = deleted.repeat !== null || deleted.recurrenceId !== null;
    const gone = this.events.filter(
      (event) =>
        event.id === id ||
        (recurring &&
          event.calendarId === deleted.calendarId &&
          this.uids.get(event.id) === this.uids.get(id)),
    );
    this.events = this.events.filter((event) => !gone.includes(event));
    const modifications = gone.map((event) =>
      this.journalEventModification(event.id, event, undefined),
    );
    return { journalTransaction: this.record(modifications), value: null };
  }

  /**
   * Cancels one Occurrence; an Exception is deleted. The cancelled ones
   * aren't part of the fake's Change Journal.
   */
  async deleteOccurrence(id: EventId, recurrenceId: string) {
    this.calls.push('deleteOccurrence');
    const event = this.findEvent(id);
    this.writable(event.calendarId);
    if (event.recurrenceId !== null) {
      this.events = this.events.filter((other) => other.id !== id);
      return {
        journalTransaction: this.record([this.journalEventModification(id, event, undefined)]),
        value: null,
      };
    }
    this.cancelled.set(id, [...(this.cancelled.get(id) ?? []), recurrenceId]);
    this.emit({ kind: 'occurrencesChanged', calendarIds: [event.calendarId] });
    return { journalTransaction: null, value: null };
  }

  async pickImportFile(title: string, filter: string) {
    this.calls.push('pickImportFile');
    this.fileDialogs.push({ title, filter });
    const file = this.importFile;
    if (file && 'code' in file) throw file;
    if (file) this.pickedImport = structuredClone(file);
    return file?.name ?? null;
  }

  async previewImport(calendarId: CalendarId) {
    this.calls.push('previewImport');
    const file = this.picked();
    this.calendar(calendarId);
    const existingEvents = file.events.filter((event) =>
      this.eventWithUid(calendarId, event.uid),
    ).length;
    return {
      calendarId,
      newEvents: file.events.length - existingEvents,
      existingEvents,
      pinned: structuredClone(file.pinned ?? []),
    };
  }

  async runImport(calendarId: CalendarId, existing: ExistingEvents) {
    this.calls.push('runImport');
    const file = this.picked();
    this.writable(calendarId);
    const modifications: JournalEventModification[] = [];
    const outcome: ImportOutcome = {
      journalTransaction: null,
      imported: 0,
      updated: 0,
      skipped: 0,
    };
    for (const imported of file.events) {
      const found = this.eventWithUid(calendarId, imported.uid);
      if (found && existing === 'skip') {
        outcome.skipped++;
      } else if (found && existing === 'update') {
        const before = structuredClone(found);
        Object.assign(found, { title: imported.title, when: { ...imported.when } });
        modifications.push(this.journalEventModification(found.id, before, found));
        outcome.updated++;
      } else {
        const uid = found ? `${imported.uid}-copy` : imported.uid;
        const { title, when } = imported;
        const added = this.addEvent({ calendarId, title, when: { ...when }, uid });
        modifications.push(this.journalEventModification(added.id, undefined, added));
        outcome.imported++;
      }
    }
    this.pickedImport = undefined;
    outcome.journalTransaction = this.record(modifications);
    return outcome;
  }

  async discardImport() {
    this.calls.push('discardImport');
    this.pickedImport = undefined;
  }

  /** The simple undo: refused while an Event of the Journal Transaction changed since. */
  async undoJournalTransaction(id: JournalTransactionId) {
    this.calls.push('undoJournalTransaction');
    const journalTransaction = this.journalTransactions.get(id);
    if (!journalTransaction) throw failure('journalTransactionNotFound');
    if (journalTransaction.undone) throw failure('alreadyUndone');
    const modifications = [...journalTransaction.journalEventModifications].reverse();
    const current = (modification: JournalEventModification) =>
      this.events.find(
        (event) => event.id === modification.id && this.uids.get(event.id) === modification.uid,
      );
    for (const modification of modifications) {
      if (JSON.stringify(current(modification)) !== JSON.stringify(modification.after)) {
        throw failure('changedSinceJournalTransaction');
      }
      for (const eventVersion of [modification.before, modification.after]) {
        if (eventVersion) this.writable(eventVersion.calendarId);
      }
    }
    for (const modification of modifications) {
      this.events = this.events.filter((event) => event !== current(modification));
      if (modification.before) this.events.push(structuredClone(modification.before));
    }
    journalTransaction.undone = true;
    const undo = modifications.map((modification) => ({
      ...modification,
      before: modification.after,
      after: modification.before,
    }));
    return { journalTransaction: this.record(undo), value: null };
  }

  async openLink(url: string) {
    this.calls.push('openLink');
    this.openedLinks.push(url);
  }

  async version() {
    this.calls.push('version');
    return '0.1.0';
  }

  async showWindow() {
    this.calls.push('showWindow');
    this.windowShown = true;
  }

  async quit() {
    this.calls.push('quit');
    this.hasQuit = true;
  }

  private started() {
    if (this.startupError) throw this.startupError;
  }

  private journalEventModification(
    id: EventId,
    before?: Event,
    after?: Event,
  ): JournalEventModification {
    const uid = this.uids.get(id)!;
    return { id, uid, before: structuredClone(before), after: structuredClone(after) };
  }

  /**
   * Records the changes `modifications` as one Journal Transaction and
   * sends "Occurrences changed" for their Calendars. A Journal Transaction
   * that changed nothing isn't kept.
   */
  private record(modifications: JournalEventModification[]): JournalTransactionId | null {
    const changed = modifications.filter(
      (modification) => JSON.stringify(modification.before) !== JSON.stringify(modification.after),
    );
    if (changed.length === 0) return null;
    const id = journalTransactionId(this.nextJournalTransactionId++);
    this.journalTransactions.set(id, { journalEventModifications: changed, undone: false });
    const calendarIds = [
      ...new Set(
        changed.flatMap((modification) =>
          [modification.before, modification.after].flatMap((eventVersion) =>
            eventVersion ? [eventVersion.calendarId] : [],
          ),
        ),
      ),
    ];
    this.emit({ kind: 'occurrencesChanged', calendarIds });
    return id;
  }

  private picked(): FakeImportFile {
    if (!this.pickedImport) throw failure('noFileToImport');
    return this.pickedImport;
  }

  private eventWithUid(calendarId: CalendarId, uid: string): Event | undefined {
    return this.events.find(
      (event) => event.calendarId === calendarId && this.uids.get(event.id) === uid,
    );
  }

  private findEvent(id: EventId): Event {
    const found = this.events.find((event) => event.id === id);
    if (!found) throw failure('eventNotFound');
    return found;
  }

  private writable(id: CalendarId) {
    if (this.calendar(id).readOnly) throw failure('calendarReadOnly');
  }

  private calendar(id: CalendarId): Calendar {
    const found = this.calendars.find((calendar) => calendar.id === id);
    if (!found) throw failure('calendarNotFound');
    return found;
  }

  /** What a server Account's server offers now; none for the Local Account. */
  private serverOf(id: AccountId): AvailableCalendar[] | undefined {
    if (!this.accounts.some((account) => account.id === id)) throw failure('accountNotFound');
    return this.offered.get(id);
  }

  private change(id: CalendarId, fields: Partial<Calendar>) {
    Object.assign(this.calendar(id), fields);
    this.emit({ kind: 'calendarsChanged' });
  }
}

/** A failure about no file or server, as the real core reports it. */
type PlainFailure = Exclude<ErrorCode, { file: string } | { status: number }>;

function failure(code: PlainFailure['code']): ErrorCode {
  // TypeScript matches `{ code }` against a union of only up to 25 codes.
  return { code } as PlainFailure;
}

/** Plain text as the core would structure it, without links. */
function plainText(text: string): StructuredText {
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

/** `when` moved by `by`, its dates and times in wall-clock time. */
function shifted(when: When, by: Temporal.DurationLike): When {
  if (when.kind === 'allDay') {
    const date = (text: string) => Temporal.PlainDate.from(text).add(by).toString();
    return { kind: 'allDay', start: date(when.start), end: date(when.end) };
  }
  const time = (text: string) =>
    Temporal.PlainDateTime.from(text).add(by).toString({ smallestUnit: 'second' });
  return { kind: 'timed', start: time(when.start), end: time(when.end) };
}

/** Like the core, a time the user changes is no longer a pinned one. */
function keptPinnedZone(event: Event, when: When): string | null {
  return JSON.stringify(event.when) === JSON.stringify(when) ? event.pinnedZone : null;
}
