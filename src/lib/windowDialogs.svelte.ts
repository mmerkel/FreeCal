import type { CoreApi } from '../core/CoreApi';
import type {
  Calendar,
  CalendarId,
  DangerousUndo,
  EventDraft,
  CombinedEventKey,
  ExceptionCounts,
  ExistingEvents,
  JournalTransactionId,
  When,
} from '../core/types';
import { errorText, t, tCount } from '../i18n';
import { sameRepeat } from './repeat';
import { actionText, runUndo, stepUndoing, type Undoing, type UndoResult } from './undo';
import { skippedText } from './skippedEvents';
import type { ScopeChange, ScopeChoice } from './ScopeDialog.svelte';
import { unsentAtQuit, unsentCount, type UnsentAtQuit } from './unsent';
import { canEdit, whyReadOnly, type WindowModel } from './windowModel.svelte';

/**
 * What is open over the grid, if anything. The Event of `details` and
 * `editing` is the selected one, which the grid marks; the Window Model
 * holds the Combined Event Key of the Event whose details are open.
 * `recurrenceId` says which Occurrence of a Recurring Event was opened,
 * none for the whole.
 */
type Open =
  | { kind: 'details'; recurrenceId: string | null }
  | {
      kind: 'editing';
      combinedEventKey: CombinedEventKey;
      recurrenceId: string | null;
      draft: EventDraft;
      customRepeat: boolean;
      repeatable: boolean;
    }
  | { kind: 'creating'; draft: EventDraft }
  | { kind: 'importing'; file: string; calendarId: CalendarId }
  | { kind: 'notice'; title: string; body: string }
  | { kind: 'settings' }
  | { kind: 'about' }
  | { kind: 'changeJournal' };

/**
 * The open question which Occurrences of a Recurring Event a change
 * changes, over the editor or, for a drop, the grid (Scope Prompt, new
 * term). It is asked once the core has counted the Exceptions it offers to
 * change too; until then it has no `counts`.
 */
interface ScopePrompt {
  change: ScopeChange;
  counts?: ExceptionCounts;
}

/**
 * The message at the bottom of the window, with its action, such as Undo,
 * if it has one. A confirmation goes after a while; a toast reporting a
 * problem `stays` until dismissed.
 */
interface Toast {
  message: string;
  action?: { label: string; run: () => void };
  stays: boolean;
}

/** The question before a Dangerous Undo or redo, with how to answer it. */
interface UndoQuestion {
  danger: DangerousUndo;
  /** The change it undoes or redoes, as `Undoing` names it. */
  change: string;
  redo: boolean;
  answer: (approved: boolean) => void;
}

/**
 * What is open over the calendar grid, one at a time, and the steps between
 * them: an Event's details and editor, the recurrence scope question,
 * import from asking for the file to its Undo, notices, settings and About,
 * with the message at the bottom of the window and the modal "Working…".
 * It writes through the core and tells the Window Model which Event's
 * details are open; the Window Model never knows about it. Read its fields
 * as `dialogs.x`; destructuring them loses their reactivity.
 */
export class WindowDialogs {
  readonly #core: CoreApi;
  readonly #model: WindowModel;

  #open = $state<Open>();
  #scopePrompt = $state<ScopePrompt>();
  /** Answers the Scope Prompt once its counts are there. */
  #chooseScope: ((choice: ScopeChoice | null) => void) | undefined;
  /** Why an action in the open details or About dialog failed. */
  #failure = $state<string>();
  #rescheduleFailure = $state<string>();
  #toast = $state<Toast>();
  /** Whether an undo or redo runs, during which another does nothing. */
  #undoing = false;
  /** Counts the user's changes, so that an undo knows whether one came while it ran. */
  #changes = 0;
  #undoQuestion = $state<UndoQuestion>();
  /** Counts editor sessions, so that each one starts from its own draft. */
  #editorSession = $state(0);
  /** FreeCal's version, read when About first opens. */
  #version = $state<string>();
  /** Why the last import from the open import dialog failed. */
  #importFailure = $state<string>();
  /** What the modal "Working…" dialog says while a long command runs. */
  #working = $state<string>();
  /**
   * The Calendars whose Unsent Changes still wait after quitting tried to
   * send them, while the quit dialog asks whether to quit anyway.
   */
  #quitting = $state<UnsentAtQuit[]>();
  /** Why quitting failed, if it did. */
  #quitFailure = $state<string>();

  /** The Calendars new, edited and imported Events can go into: those that aren't Read-only. */
  readonly #writableCalendars: Calendar[];

  constructor(core: CoreApi, model: WindowModel) {
    this.#core = core;
    this.#model = model;
    this.#writableCalendars = $derived(
      model.directory?.calendars.filter((calendar) => canEdit(calendar, false)) ?? [],
    );
  }

  /** What is open over the grid, apart from the Scope Prompt. */
  get open() {
    return this.#open;
  }
  /**
   * Whether anything is open over the grid, the Scope Prompt, the question
   * before a Dangerous Undo and the quit dialog too.
   */
  get isOpen() {
    return (
      this.#open !== undefined ||
      this.#scopePrompt !== undefined ||
      this.#undoQuestion !== undefined ||
      this.#quitting !== undefined
    );
  }
  /** The question before a Dangerous Undo or redo, while it asks. */
  get undoQuestion() {
    return this.#undoQuestion;
  }
  get scopePrompt() {
    return this.#scopePrompt;
  }
  /** Why an action in the open details or About dialog failed. */
  get failure() {
    return this.#failure;
  }
  /** Why the last drag or resize failed, until one succeeds. */
  get rescheduleFailure() {
    return this.#rescheduleFailure;
  }
  get toast() {
    return this.#toast;
  }
  /** Changes with each editor opened, so that each one starts from its own draft. */
  get editorSession() {
    return this.#editorSession;
  }
  get version() {
    return this.#version;
  }
  /** Why the last import from the open import dialog failed. */
  get importFailure() {
    return this.#importFailure;
  }
  /** What the modal "Working…" dialog says while a long command runs. */
  get working() {
    return this.#working;
  }
  /** The Calendars the quit dialog lists, while it is open. */
  get quitting() {
    return this.#quitting;
  }
  /** Why quitting failed, if it did. */
  get quitFailure() {
    return this.#quitFailure;
  }
  /** The Calendars new, edited and imported Events can go into: those that aren't Read-only. */
  get writableCalendars() {
    return this.#writableCalendars;
  }
  /** The Event whose details or editor are open, which the grid marks. */
  get selectedEventKey() {
    const open = this.#open;
    if (open?.kind === 'details') return this.#model.openedCombinedEventKey;
    return open?.kind === 'editing' ? open.combinedEventKey : undefined;
  }
  get selectedRecurrenceId() {
    const open = this.#open;
    return open?.kind === 'details' || open?.kind === 'editing'
      ? (open.recurrenceId ?? undefined)
      : undefined;
  }
  /** The Occurrence of a Recurring Event whose details are open, as the grid shows it. */
  get openedOccurrence() {
    const open = this.#open;
    const combinedEventKey = this.#model.openedCombinedEventKey;
    if (open?.kind !== 'details' || open.recurrenceId === null || !combinedEventKey)
      return undefined;
    return this.#occurrence(combinedEventKey, open.recurrenceId);
  }
  /** The Occurrence in view of the Event with key `combinedEventKey` with `recurrenceId`. */
  #occurrence(combinedEventKey: CombinedEventKey, recurrenceId: string | null) {
    return this.#model.occurrences.find(
      (occurrence) =>
        occurrence.combinedEventKey === combinedEventKey &&
        occurrence.recurrenceId === recurrenceId,
    );
  }
  /**
   * Whether the grid holds back a new today for a dropped Occurrence while
   * the user chooses its recurrence scope, as it does during the drag.
   */
  get dropHeld() {
    return this.#scopePrompt?.change === 'move';
  }

  /** The Calendar new things go into: the first shown writable one. */
  #defaultCalendar(): Calendar | undefined {
    const writable = this.writableCalendars;
    return writable.find((calendar) => calendar.shown) ?? writable[0];
  }

  /** Opens the editor for a new Event, or says where to create a Calendar first. */
  startCreating(when: When) {
    const target = this.#defaultCalendar();
    if (!target) {
      this.#showNotice(
        t('event.noCalendarTitle'),
        t('event.noCalendarBody', { account: t('account.local') }),
      );
      return;
    }
    this.#editorSession++;
    this.#show({
      kind: 'creating',
      draft: {
        calendarId: target.id,
        title: '',
        when,
        location: '',
        description: '',
        repeat: null,
      },
    });
  }

  /** Opens the details of the Event with key `combinedEventKey`, or of its Occurrence `recurrenceId`. */
  openDetails(combinedEventKey: CombinedEventKey, recurrenceId: string | null = null) {
    this.#failure = undefined;
    this.#show({ kind: 'details', recurrenceId });
    this.#model.openDetails(combinedEventKey, () => this.close());
  }

  /**
   * Edits the Event whose details are open; for an Occurrence of a
   * Recurring Event, that Occurrence, and Save asks which Occurrences the
   * change is for.
   */
  startEditing() {
    const event = this.#model.opened;
    if (this.#open?.kind !== 'details' || !event) return;
    const { recurrenceId } = this.#open;
    this.#editorSession++;
    this.#show({
      kind: 'editing',
      combinedEventKey: event.combinedEventKey,
      recurrenceId,
      draft: {
        calendarId: event.calendarId,
        title: event.title,
        when: this.openedOccurrence?.when ?? event.when,
        location: event.location.text,
        description: event.description.text,
        repeat: event.repeat,
      },
      customRepeat: event.customRepeat,
      // An Exception repeats with its Recurring Event, if that is still there.
      repeatable: event.recurrenceId === null || event.repeat !== null,
    });
  }

  /**
   * Saves the open editor's draft. An edit returns to the details of the
   * Occurrence it changed; for an Occurrence of a Recurring Event it asks
   * which Occurrences the change is for first, and cancelling that leaves
   * the editor open. Rejects when the core does, for the editor to say why.
   */
  async save(draft: EventDraft) {
    const editor = this.#open;
    const title = draft.title;
    if (editor?.kind === 'creating') {
      const { journalTransaction } = await this.#core.createEvent(draft);
      this.close();
      this.#recorded(journalTransaction, t('undo.done.created'), actionText('created', title));
      return;
    }
    if (editor?.kind !== 'editing') return;
    const { combinedEventKey, recurrenceId } = editor;
    if (recurrenceId === null) {
      const { journalTransaction, value } = await this.#core.editEvent(combinedEventKey, draft);
      this.openDetails(value.combinedEventKey, value.recurrenceId);
      this.#recorded(journalTransaction, t('undo.done.edited'), actionText('edited', title));
      return;
    }
    const change = sameRepeat(draft.repeat, editor.draft.repeat) ? 'edit' : 'editRepeat';
    const choice = await this.#askScope(change, combinedEventKey, recurrenceId);
    if (choice === null) return;
    const { scope, exceptionsToo } = choice;
    const { journalTransaction, value } = await this.#core.editOccurrence(
      combinedEventKey,
      recurrenceId,
      scope,
      exceptionsToo,
      draft,
    );
    this.openDetails(value.combinedEventKey, value.recurrenceId);
    this.#recorded(journalTransaction, t('undo.done.edited'), actionText('edited', title));
  }

  /** Leaves the editor: back to the details of an existing Event, else closed. */
  cancelEditing() {
    if (this.#open?.kind === 'editing')
      this.openDetails(this.#open.combinedEventKey, this.#open.recurrenceId);
    else this.close();
  }

  /** The user chose in the Scope Prompt, or cancelled it with `null`. */
  chooseScope(choice: ScopeChoice | null) {
    this.#chooseScope?.(choice);
  }

  /**
   * A drag or resize in the grid; for an Occurrence of a Recurring Event it
   * asks which Occurrences move first. Rejects when the user cancels or the
   * core fails, and the grid puts the Occurrence back.
   */
  async reschedule(combinedEventKey: CombinedEventKey, recurrenceId: string | null, when: When) {
    const title = this.#occurrence(combinedEventKey, recurrenceId)?.title ?? '';
    let write: () => Promise<{ journalTransaction: JournalTransactionId | null }>;
    if (recurrenceId === null) {
      write = () => this.#core.rescheduleEvent(combinedEventKey, when);
    } else {
      const choice = await this.#askScope('move', combinedEventKey, recurrenceId);
      if (choice === null) throw new Error('The move was cancelled.');
      const { scope, exceptionsToo } = choice;
      write = () =>
        this.#core.rescheduleOccurrence(combinedEventKey, recurrenceId, scope, exceptionsToo, when);
    }
    try {
      const { journalTransaction } = await write();
      this.#rescheduleFailure = undefined;
      this.#recorded(journalTransaction, t('undo.done.moved'), actionText('moved', title));
    } catch (error) {
      this.#rescheduleFailure = t('event.rescheduleFailed', { error: errorText(error) });
      throw error;
    }
  }

  /**
   * A drag the grid refused, because the Occurrence's Calendar is read-only
   * or its Event has guests, as `whyReadOnly` says.
   */
  refuseDrag(combinedEventKey: CombinedEventKey, recurrenceId: string | null) {
    const dragged = this.#occurrence(combinedEventKey, recurrenceId);
    const calendar = this.#model.directory?.calendars.find(
      (found) => found.id === dragged?.calendarId,
    );
    if (!dragged || !calendar) return;
    const title = dragged.title || t('event.untitled');
    const why = whyReadOnly(calendar, dragged.hasGuests);
    if (why === 'calendarReadOnly') {
      this.#toast = {
        message: t('event.readOnlyDrag', { title, calendar: calendar.name }),
        stays: true,
      };
    } else if (why === 'eventHasGuests') {
      this.#toast = { message: t('event.hasGuestsDrag', { title }), stays: true };
    }
  }

  /** Deletes the Event whose details are open, all its Occurrences. */
  async remove() {
    const combinedEventKey = this.#model.openedCombinedEventKey;
    if (this.#open?.kind !== 'details' || !combinedEventKey) return;
    await this.#deleting(() => this.#core.deleteEvent(combinedEventKey));
  }

  /** Deletes the Occurrence of a Recurring Event whose details are open. */
  async removeOccurrence() {
    const combinedEventKey = this.#model.openedCombinedEventKey;
    if (this.#open?.kind !== 'details' || this.#open.recurrenceId === null || !combinedEventKey)
      return;
    const { recurrenceId } = this.#open;
    await this.#deleting(() => this.#core.deleteOccurrence(combinedEventKey, recurrenceId));
  }

  /** Opens a link of the open details or About in the system browser. */
  async openLink(url: string) {
    try {
      await this.#core.openLink(url);
    } catch (error) {
      this.#failure = t('link.openFailed', { error: errorText(error) });
    }
  }

  openSettings() {
    this.#model.forgetSettingsFailure();
    this.#show({ kind: 'settings' });
  }

  /** Opens the Change Journal list. */
  openChangeJournal() {
    this.#show({ kind: 'changeJournal' });
  }

  openAbout() {
    this.#failure = undefined;
    this.#show({ kind: 'about' });
    // Without the version About still says everything else.
    if (this.#version === undefined)
      this.#core.version().then(
        (read) => (this.#version = read),
        () => {},
      );
  }

  /** Asks for a file to import, then opens the import dialog for it. */
  async startImport() {
    if (!this.#defaultCalendar()) {
      this.#showNotice(
        t('import.noCalendarTitle'),
        t('event.noCalendarBody', { account: t('account.local') }),
      );
      return;
    }
    let file: string | null;
    try {
      file = await this.#core.pickImportFile(t('import.fileDialogTitle'), t('import.fileFilter'));
    } catch (error) {
      this.#showNotice(
        t('import.pickFailedTitle'),
        t('import.pickFailed', { error: errorText(error) }),
      );
      return;
    }
    if (file === null) return;
    const target = this.#defaultCalendar();
    // The user may have opened something else while the file dialog was showing.
    if (!target || this.isOpen) {
      this.#discardImport();
      return;
    }
    this.#importFailure = undefined;
    this.#show({ kind: 'importing', file, calendarId: target.id });
  }

  /** Closes the import dialog and lets its file go, unless its import runs. */
  cancelImport() {
    // Nothing closes the dialog while its import runs, not even an Escape
    // that reaches it under "Working…".
    if (this.#working) return;
    this.#discardImport();
    if (this.#open?.kind === 'importing') this.close();
  }

  /**
   * Imports the open import dialog's file into `calendarId`, saying
   * "Working…" meanwhile; once done, the toast says what changed and offers
   * Undo. A failure stays in the dialog.
   */
  async runImport(calendarId: CalendarId, existing: ExistingEvents) {
    if (this.#open?.kind !== 'importing') return;
    const { file } = this.#open;
    try {
      const outcome = await this.#whileWorking(t('working.importing', { file }), () =>
        this.#core.runImport(calendarId, existing),
      );
      this.close();
      const message = t('import.done', {
        file,
        imported: String(outcome.imported),
        updated: String(outcome.updated),
        skipped: String(outcome.skipped),
      });
      // An import that changed nothing has no Journal Transaction to undo,
      // yet its toast says what it skipped.
      if (outcome.journalTransaction === null) this.#toast = { message, stays: false };
      const change = t('undo.action.importedFile', { file });
      this.#recorded(outcome.journalTransaction, message, change, true);
    } catch (error) {
      this.#importFailure = t('import.failed', { error: errorText(error) });
    }
  }

  dismissToast() {
    this.#toast = undefined;
  }

  /**
   * Ctrl+Z: undoes the next step of the Undo History, asking first if it is
   * a Dangerous Undo; the toast says what it did and offers Redo.
   */
  undo() {
    return this.#undoStep(false);
  }

  /** Ctrl+Shift+Z: redoes the next step of the Undo History, by undoing an undo. */
  redo() {
    return this.#undoStep(true);
  }

  /**
   * Undoes `undoing`, from a toast, the Change Journal list or an Event's
   * History, asking first if it is a Dangerous Undo; the toast says what it
   * did and offers Redo. Resolves once it is done or failed.
   */
  undoChange(undoing: Undoing) {
    return this.#runUndo(undoing, false);
  }

  /** The user approved the Dangerous Undo asked about, or not. */
  answerUndo(approved: boolean) {
    this.#undoQuestion?.answer(approved);
  }

  /**
   * Quit, from the main menu or the window's close (spec: "Quitting").
   * With Unsent Changes waiting, it first tries to send them for a few
   * seconds under "Working…", and quits if that sent them all; otherwise
   * the quit dialog asks, listing the Calendars whose changes still wait
   * and why. What is open over the grid stays open beneath. A Quit while
   * one runs or asks does nothing.
   */
  async quit() {
    if (this.#working !== undefined || this.#quitting !== undefined) return;
    // Read now, as the window's may lag behind a change just made.
    const before = await this.#core.syncStatuses().catch(() => this.#model.syncStatuses);
    if (!before || unsentCount(before) === 0) return this.quitAnyway();
    const after = await this.#whileWorking(tCount('working.sending', unsentCount(before)), () =>
      // What waited before still waits, as far as the window knows.
      this.#core.sendBeforeQuit().catch(() => before),
    );
    const unsent = unsentAtQuit(after, this.#model.directory?.calendars ?? []);
    if (unsent.length === 0) return this.quitAnyway();
    this.#quitting = unsent;
  }

  /** Closes the quit dialog without quitting. */
  cancelQuit() {
    this.#quitting = undefined;
  }

  /** Quits at once, leaving any Unsent Changes for the next start. */
  quitAnyway() {
    this.#quitting = undefined;
    this.#quitFailure = undefined;
    this.#core
      .quit()
      .catch((error) => (this.#quitFailure = t('app.quitFailed', { error: errorText(error) })));
  }

  /** Closes what is open over the grid. */
  close() {
    this.#undoQuestion?.answer(false);
    this.#show(undefined);
    this.#failure = undefined;
  }

  #showNotice(title: string, body: string) {
    this.#show({ kind: 'notice', title, body });
  }

  #discardImport() {
    this.#core.discardImport().catch(() => {});
  }

  /**
   * After the user's change made as `journalTransaction`, the toast says
   * `message` and offers Undo, which names it `change` and runs under
   * "Working…" if `slow`. A change that changed nothing has no Undo; a
   * change of the user's ends what Ctrl+Shift+Z redoes.
   */
  #recorded(
    journalTransaction: JournalTransactionId | null,
    message: string,
    change: string,
    slow = false,
  ) {
    if (journalTransaction === null) return;
    this.#changes += 1;
    const target = { kind: 'journalTransaction', id: journalTransaction } as const;
    this.#toast = {
      message,
      action: {
        label: t('common.undo'),
        run: () => void this.undoChange({ target, change, slow }),
      },
      stays: false,
    };
  }

  /** Undoes or redoes the next step of the Undo History, read from the core. */
  async #undoStep(redo: boolean) {
    if (this.#undoing) return;
    this.dismissToast();
    let step;
    try {
      const history = await this.#core.undoHistory();
      step = redo ? history.redo : history.undo;
    } catch (error) {
      this.#toast = { message: t('undo.historyFailed', { error: errorText(error) }), stays: true };
      return;
    }
    if (!step) {
      this.#toast = {
        message: t(redo ? 'undo.nothingToRedo' : 'undo.nothingToUndo'),
        stays: false,
      };
      return;
    }
    await this.#runUndo(stepUndoing(step), redo);
  }

  /**
   * Runs `undoing`, an undo or a redo, asking first if it is a Dangerous
   * Undo, under "Working…" if it is slow; another one meanwhile does nothing.
   */
  async #runUndo(undoing: Undoing, redo: boolean) {
    if (this.#undoing) return;
    this.#undoing = true;
    this.dismissToast();
    const changes = this.#changes;
    try {
      const { change } = undoing;
      const result = await runUndo(this.#core, undoing.target, {
        approve: (danger) =>
          new Promise((resolve) => {
            this.#undoQuestion = {
              danger,
              change,
              redo,
              answer: (approved) => {
                this.#undoQuestion = undefined;
                resolve(approved);
              },
            };
          }),
        running: (run) =>
          undoing.slow
            ? this.#whileWorking(t(redo ? 'working.redoing' : 'working.undoing', { change }), run)
            : run(),
      });
      // A change of the user's while it ran ends redo.
      this.#reportUndo(undoing, redo, result, redo || this.#changes === changes);
    } finally {
      this.#undoing = false;
    }
  }

  /**
   * Says in the toast what the undo or redo `undoing` did, with the Events
   * it skipped, and offers to take it back, if `reversible` and it changed
   * something, or to try again.
   */
  #reportUndo(undoing: Undoing, redo: boolean, result: UndoResult, reversible: boolean) {
    if (result.kind === 'cancelled') return;
    const action = undoing.change;
    if (result.kind === 'failed') {
      const error = errorText(result.error);
      this.#toast = {
        message: t(redo ? 'undo.redoFailed' : 'undo.undoFailed', { action, error }),
        action: result.retryable
          ? {
              label: t(redo ? 'common.redo' : 'common.undo'),
              run: () => void this.#runUndo(undoing, redo),
            }
          : undefined,
        stays: true,
      };
      return;
    }
    const { changed, skipped } = result;
    const calendars = this.#model.directory?.calendars ?? [];
    const done = changed
      ? t(redo ? 'undo.redone' : 'undo.undone', { action })
      : t(redo ? 'undo.nothingRedone' : 'undo.nothingUndone', { action });
    this.#toast = {
      message: [done, ...skipped.map((event) => skippedText(event, calendars))].join(' '),
      // Taking it back is the next step of the Undo History the other way.
      action:
        changed && reversible
          ? {
              label: t(redo ? 'common.undo' : 'common.redo'),
              run: () => void (redo ? this.undo() : this.redo()),
            }
          : undefined,
      stays: !changed || skipped.length > 0,
    };
  }

  /** Runs `run`, saying `message` under "Working…" until it settles. */
  async #whileWorking<T>(message: string, run: () => Promise<T>): Promise<T> {
    this.#working = message;
    try {
      return await run();
    } finally {
      this.#working = undefined;
    }
  }

  /** Opens `next` over the grid in place of what was open, or nothing. */
  #show(next: Open | undefined) {
    // The Window Model reads the Event of the open details only while they are open.
    if (this.#open?.kind === 'details') this.#model.closeDetails();
    this.#open = next;
  }

  /** Deletes as `write` does the Event whose details are open, then closes them. */
  async #deleting(write: () => Promise<{ journalTransaction: JournalTransactionId | null }>) {
    const title = this.#model.opened?.title ?? '';
    try {
      const { journalTransaction } = await write();
      this.close();
      this.#recorded(journalTransaction, t('undo.done.deleted'), actionText('deleted', title));
    } catch (error) {
      this.#failure = t('event.deleteFailed', { error: errorText(error) });
    }
  }

  /**
   * Asks which Occurrences of a Recurring Event a change to its Occurrence
   * `recurrenceId` changes, and whether its Exceptions with values of their
   * own change too. Resolves to `null` when the user cancels. A drop's
   * prompt holds back a "Date changed" while it is open.
   */
  async #askScope(
    change: ScopeChange,
    combinedEventKey: CombinedEventKey,
    recurrenceId: string,
  ): Promise<ScopeChoice | null> {
    // Open while the counts load too, so that a drop is held from the start.
    this.#scopePrompt = { change };
    const release = change === 'move' ? this.#model.hold() : undefined;
    try {
      const counts = await this.#core.exceptionCounts(combinedEventKey, recurrenceId);
      return await new Promise((choose) => {
        this.#chooseScope = choose;
        this.#scopePrompt = { change, counts };
      });
    } finally {
      this.#chooseScope = undefined;
      this.#scopePrompt = undefined;
      release?.();
    }
  }
}
