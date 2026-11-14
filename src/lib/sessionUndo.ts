import type { CoreApi } from '../core/CoreApi';
import type {
  Changed,
  DangerousUndo,
  JournalTransactionId,
  SkippedEvent,
  UndoOutcome,
} from '../core/types';

/** A change the user made, as the toasts name it. */
export type UserAction =
  | { kind: 'created' | 'changed' | 'moved' | 'deleted'; title: string }
  | { kind: 'imported'; file: string };

/** A Journal Transaction Ctrl+Z or Ctrl+Shift+Z undoes, with the user action it undoes or redoes. */
interface Entry {
  journalTransaction: JournalTransactionId;
  action: UserAction;
}

/** What an undo or redo of Session Undo did. */
export type SessionUndoResult =
  /** There was nothing to undo or redo. */
  | { kind: 'nothing' }
  /** Another undo or redo was running, so it did nothing. */
  | { kind: 'busy' }
  /** The user didn't approve a Dangerous Undo; it stays to undo or redo. */
  | { kind: 'cancelled' }
  /**
   * It ran. `changed` says whether it changed anything; when it did, it can
   * be redone, or undone again after a redo.
   */
  | { kind: 'done'; action: UserAction; changed: boolean; skipped: SkippedEvent[] }
  /** The core refused; it stays to try again, unless it was undone already. */
  | { kind: 'failed'; action: UserAction; error: unknown; again: boolean };

/** How Session Undo asks the user before a Dangerous Undo, and runs a core undo. */
export interface SessionUndoHooks {
  /** Asks whether to run the Dangerous Undo `danger` of `action`, an undo or a redo. */
  approve: (danger: DangerousUndo, action: UserAction, redo: boolean) => Promise<boolean>;
  /** Runs one undo of the core for `action`, an undo or a redo, such as under "Working…". */
  running: <T>(action: UserAction, redo: boolean, run: () => Promise<T>) => Promise<T>;
}

/**
 * The user's own changes of this session, which Ctrl+Z undoes newest first,
 * and the undos of them, which Ctrl+Shift+Z redoes until the user's next
 * change (Session Undo, new term; spec: US-146). A redo is the undo of an
 * undo's Journal Transaction, so it is undone again like a change. Undos
 * made elsewhere, such as from an Event's History, are never on it, and
 * neither is a sync. One already undone elsewhere is skipped.
 */
export class SessionUndo {
  readonly #core: CoreApi;
  readonly #hooks: SessionUndoHooks;
  /** The changes Ctrl+Z undoes, oldest first. */
  readonly #undoable: Entry[] = [];
  /** The undos Ctrl+Shift+Z redoes, oldest first. */
  #redoable: Entry[] = [];
  /** Whether an undo or redo runs, during which another does nothing. */
  #running = false;

  constructor(core: CoreApi, hooks: SessionUndoHooks) {
    this.#core = core;
    this.#hooks = hooks;
  }

  /** The user made `action` as `journalTransaction`; nothing undone before can be redone now. */
  recorded(journalTransaction: JournalTransactionId, action: UserAction) {
    this.#undoable.push({ journalTransaction, action });
    this.#redoable = [];
  }

  /** Undoes the user's latest change of this session, or the change made as `journalTransaction`. */
  undo(journalTransaction?: JournalTransactionId): Promise<SessionUndoResult> {
    return this.#run(this.#undoable, this.#redoable, false, journalTransaction);
  }

  /** Redoes the latest undo, or the undo made as `journalTransaction`. */
  redo(journalTransaction?: JournalTransactionId): Promise<SessionUndoResult> {
    return this.#run(this.#redoable, this.#undoable, true, journalTransaction);
  }

  /**
   * Undoes the newest Entry of `from`, or the one of `picked`, and puts its
   * undo onto `to`. Without `picked`, an Entry undone already is dropped and
   * the next one tried.
   */
  async #run(
    from: Entry[],
    to: Entry[],
    redo: boolean,
    picked?: JournalTransactionId,
  ): Promise<SessionUndoResult> {
    if (this.#running) return { kind: 'busy' };
    this.#running = true;
    try {
      for (;;) {
        const entry =
          picked === undefined
            ? from.at(-1)
            : from.find((found) => found.journalTransaction === picked);
        if (!entry) return { kind: 'nothing' };
        const result = await this.#undoEntry(entry, redo);
        if (result.kind === 'cancelled') return result;
        if (result.kind === 'failed' && result.again) return result;
        from.splice(from.indexOf(entry), 1);
        if (result.kind === 'done') {
          if (result.undoneAs !== null) {
            to.push({ journalTransaction: result.undoneAs, action: entry.action });
          }
          return {
            kind: 'done',
            action: entry.action,
            changed: result.undoneAs !== null,
            skipped: result.skipped,
          };
        }
        if (picked !== undefined) return result;
      }
    } finally {
      this.#running = false;
    }
  }

  /** Undoes `entry`'s Journal Transaction, asking first if it is a Dangerous Undo. */
  async #undoEntry(
    entry: Entry,
    redo: boolean,
  ): Promise<
    | { kind: 'done'; undoneAs: JournalTransactionId | null; skipped: SkippedEvent[] }
    | { kind: 'cancelled' }
    | Extract<SessionUndoResult, { kind: 'failed' }>
  > {
    const { action } = entry;
    const target = { kind: 'journalTransaction', id: entry.journalTransaction } as const;
    try {
      let approved: DangerousUndo | null = null;
      for (;;) {
        const danger: DangerousUndo | null = approved;
        const { journalTransaction, value }: Changed<UndoOutcome> = await this.#hooks.running(
          action,
          redo,
          () => this.#core.undo(target, danger),
        );
        if (value.kind === 'undone') {
          return { kind: 'done', undoneAs: journalTransaction, skipped: value.skipped };
        }
        // Asked again when it changed since the last approval, such as by a sync.
        if (!(await this.#hooks.approve(value.danger, action, redo))) return { kind: 'cancelled' };
        approved = value.danger;
      }
    } catch (error) {
      return { kind: 'failed', action, error, again: !undoneAlready(error) };
    }
  }
}

/** Whether `error` is the core's refusal of an undo whose changes are all undone already. */
function undoneAlready(error: unknown): boolean {
  return (
    typeof error === 'object' && error !== null && 'code' in error && error.code === 'alreadyUndone'
  );
}
