import { tick } from 'svelte';
import type { CoreApi } from '../core/CoreApi';
import type {
  Account,
  Calendar,
  Event,
  EventId,
  Occurrence,
  Settings,
  SettingsUpdate,
  SidebarState,
  Signal,
  SyncStatuses,
} from '../core/types';
import { errorText, startupText, t } from '../i18n';
import { dateChanged, opening, type Navigation } from './navigation';
import { NewestRead } from './newestRead';
import { DEFAULT_SETTINGS } from './settings';
import { SyncingDelay } from './syncDisplay.svelte';
import type { VisibleRange } from './when';

/** The Accounts and Calendars, as last read together (Directory, new term). */
interface Directory {
  accounts: Account[];
  calendars: Calendar[];
}

/** The Event whose details are open, as the window told the model (Open Details, new term). */
interface OpenDetails {
  id: EventId;
  /** Runs when the Event can't be read any more, as when it was deleted. */
  ongone: () => void;
}

/**
 * What the window shows, kept current (spec: "Signals"): the Accounts and
 * Calendars, the Sync Statuses, the Occurrences in view, the Event whose
 * details are open, today and the view, the sidebar and the settings. It
 * only reads; changes go through the core and come back as Signals, except
 * the window's own view, sidebar and settings, which it shows at once and
 * writes in the background. Read its fields as `model.x`; destructuring
 * them loses their reactivity.
 */
export class WindowModel {
  /** Why FreeCal couldn't start, in words, for the startup error screen. */
  #startupReason = $state<string>();
  #settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  #settingsFailure = $state<string>();
  #today = $state<Temporal.PlainDate>();
  #navigation = $state<Navigation>();
  #viewSaveFailure = $state<string>();
  #sidebar = $state<SidebarState>();
  #sidebarSaveFailure = $state<string>();
  #directory = $state<Directory>();
  /** The Sync Status of every server Account and Calendar, as last read. */
  #syncStatuses = $state<SyncStatuses>();
  readonly #syncingDelay = new SyncingDelay();
  #shownSyncStatuses = $derived(this.#syncStatuses && this.#syncingDelay.shown(this.#syncStatuses));
  #readFailure = $state<string>();
  #range: VisibleRange | undefined;
  #occurrences = $state<Occurrence[]>([]);
  #details: OpenDetails | undefined;
  #opened = $state<Event>();

  readonly #directoryRead = new NewestRead();
  readonly #syncStatusesRead = new NewestRead();
  readonly #occurrencesRead = new NewestRead();
  readonly #eventRead = new NewestRead();

  /** How many hold back a "Date changed" now, such as the grid's press. */
  #holds = 0;
  /** Whether a "Date changed" came while held back, and waits for the last release. */
  #dateChangeHeld = false;

  readonly #core: CoreApi;

  constructor(core: CoreApi) {
    this.#core = core;
  }

  get startupReason() {
    return this.#startupReason;
  }
  get settings() {
    return this.#settings;
  }
  /** Why the last settings write failed, until one succeeds or the settings open again. */
  get settingsFailure() {
    return this.#settingsFailure;
  }
  get today() {
    return this.#today;
  }
  get navigation() {
    return this.#navigation;
  }
  get viewSaveFailure() {
    return this.#viewSaveFailure;
  }
  get sidebar() {
    return this.#sidebar;
  }
  get sidebarSaveFailure() {
    return this.#sidebarSaveFailure;
  }
  get directory() {
    return this.#directory;
  }
  /** The Sync Statuses as shown, with a routine sync's Syncing held back for a moment. */
  get syncStatuses() {
    return this.#shownSyncStatuses;
  }
  /** Why the last read of what the window shows failed. */
  get readFailure() {
    return this.#readFailure;
  }
  /** The Occurrences of the visible range. */
  get occurrences() {
    return this.#occurrences;
  }
  /** The Event whose details are open, as last read. */
  get opened() {
    return this.#opened;
  }

  /**
   * Starts FreeCal's window: the startup failure, else subscribing to the
   * Signals, the settings and showing the window, then what it shows.
   * Returns the function that stops it.
   */
  start(): () => void {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;

    const start = async () => {
      const failure = await this.#core.startupFailure();
      if (failure) return this.#failStartup(startupText(failure));
      // Subscribe first, then read, so that no change falls in the gap.
      const stop = await this.#core.subscribe((signal) => {
        if (!stopped) this.#onSignal(signal);
      });
      if (stopped) return stop();
      unsubscribe = stop;
      // The window shows once its appearance is known, so that it never
      // flashes the wrong theme.
      this.#settings = await this.#core.settings();
      await tick();
      this.#showWindow();
      const [currentDate, view, sidebar] = await Promise.all([
        this.#core.today(),
        this.#core.lastUsedView(),
        this.#core.sidebar(),
        this.#readDirectory(),
        this.#readSyncStatuses(),
      ]);
      // A "Date changed" that came in meanwhile is newer.
      this.#today ??= currentDate;
      this.#sidebar = sidebar;
      this.#navigation = opening(view, this.#today);
    };

    start().catch((error) =>
      // Drawn with the default settings.
      this.#failStartup(t('app.loadFailed', { error: errorText(error) })),
    );
    return () => {
      stopped = true;
      unsubscribe?.();
      this.#syncingDelay.stop();
    };
  }

  /** Shows what `next` says, remembering a changed view as the last-used one. */
  navigate(next: Navigation) {
    if (next.view !== this.#navigation?.view) {
      this.#core.setLastUsedView(next.view).then(
        () => (this.#viewSaveFailure = undefined),
        (error) => (this.#viewSaveFailure = t('app.viewSaveFailed', { error: errorText(error) })),
      );
    }
    this.#navigation = next;
  }

  /** Shows the sidebar as `next` says and remembers it straight away. */
  changeSidebar(next: SidebarState) {
    this.#sidebar = next;
    this.#core.setSidebar(next).then(
      () => (this.#sidebarSaveFailure = undefined),
      (error) =>
        (this.#sidebarSaveFailure = t('app.sidebarSaveFailed', { error: errorText(error) })),
    );
  }

  toggleSidebar() {
    if (this.#sidebar) this.changeSidebar({ ...this.#sidebar, shown: !this.#sidebar.shown });
  }

  /** Writes one changed setting; it has taken effect before the core answers. */
  changeSettings(update: SettingsUpdate) {
    this.#settings = { ...this.#settings, ...update };
    this.#core.updateSettings(update).then(
      () => (this.#settingsFailure = undefined),
      (error) => {
        this.#settingsFailure = t('settings.saveFailed', { error: errorText(error) });
        // Back to what the Local Store kept, so that the window doesn't show
        // a setting it won't open with next time.
        this.#core.settings().then(
          (stored) => (this.#settings = stored),
          () => {},
        );
      },
    );
  }

  /** Forgets why the last settings write failed, as the settings open afresh. */
  forgetSettingsFailure() {
    this.#settingsFailure = undefined;
  }

  /** The grid shows `range` now; reads its Occurrences. */
  showRange(range: VisibleRange) {
    this.#range = range;
    this.#readOccurrences().catch(this.#reportRead);
  }

  /**
   * The details of Event `id` are open: reads it now and again on each
   * change. `ongone` runs when it can't be read any more.
   */
  openDetails(id: EventId, ongone: () => void) {
    this.#details = { id, ongone };
    this.#opened = undefined;
    void this.#readOpenedEvent();
  }

  /** No Event's details are open any more. */
  closeDetails() {
    this.#details = undefined;
    this.#opened = undefined;
  }

  /**
   * Holds back a "Date changed", so that the view doesn't move under the
   * user, as during a press on the grid. Returns the release; once the last
   * holder has released, the view follows a "Date changed" that came
   * meanwhile.
   */
  hold(): () => void {
    this.#holds++;
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.#holds--;
      if (this.#holds === 0 && this.#dateChangeHeld) {
        this.#dateChangeHeld = false;
        this.#followToday();
      }
    };
  }

  /** Shows the startup error screen, saying `reason`. */
  #failStartup(reason: string) {
    this.#startupReason = reason;
    this.#showWindow();
  }

  /**
   * Shows the window, which starts hidden. Should that fail, the shell shows
   * it a few seconds later anyway.
   */
  #showWindow() {
    this.#core.showWindow().catch(() => {});
  }

  #onSignal(signal: Signal) {
    switch (signal.kind) {
      case 'calendarsChanged':
        // Everything shown depends on the Calendars.
        this.#readDirectory().catch(this.#reportRead);
        this.#readSyncStatuses().catch(this.#reportRead);
        this.#readOccurrences().catch(this.#reportRead);
        void this.#readOpenedEvent();
        break;
      case 'occurrencesChanged':
        // The open details and the grid show the new state; an open editor
        // keeps its draft.
        this.#readOccurrences().catch(this.#reportRead);
        void this.#readOpenedEvent();
        break;
      case 'dateChanged':
        this.#today = Temporal.PlainDate.from(signal.date);
        this.#followToday();
        break;
      case 'accountSyncStatusChanged':
      case 'calendarSyncStatusChanged':
        // Read whole, as the overall status comes with them.
        this.#readSyncStatuses().catch(this.#reportRead);
        break;
    }
  }

  /** Moves the view to a new today if it tracks it and nothing holds it back. */
  #followToday() {
    if (this.#holds > 0) {
      this.#dateChangeHeld = true;
    } else if (this.#navigation && this.#today) {
      this.#navigation = dateChanged(this.#navigation, this.#today);
    }
  }

  #reportRead = (error: unknown) => {
    this.#readFailure = errorText(error);
  };

  #readDirectory() {
    return this.#directoryRead.run(
      () => Promise.all([this.#core.listAccounts(), this.#core.listCalendars()]),
      ([accounts, calendars]) => {
        this.#directory = { accounts, calendars };
        this.#readFailure = undefined;
      },
    );
  }

  #readSyncStatuses() {
    return this.#syncStatusesRead.run(
      () => this.#core.syncStatuses(),
      (statuses) => (this.#syncStatuses = statuses),
    );
  }

  async #readOccurrences() {
    const range = this.#range;
    if (!range) return;
    await this.#occurrencesRead.run(
      () => this.#core.listOccurrences(range.from, range.to),
      (found) => (this.#occurrences = found),
    );
  }

  /** Reads the Event whose details are open again; says it is gone if it can't. */
  async #readOpenedEvent() {
    const details = this.#details;
    if (!details) return;
    try {
      await this.#eventRead.run(
        () => this.#core.event(details.id),
        (event) => {
          if (this.#details === details) this.#opened = event;
        },
      );
    } catch {
      if (this.#details === details) details.ongone();
    }
  }
}
