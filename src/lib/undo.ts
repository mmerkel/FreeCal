import type { CoreApi } from '../core/CoreApi';
import type {
  Calendar,
  Changed,
  DangerousUndo,
  ListedJournalEventModification,
  ListedJournalTransaction,
  SkippedEvent,
  TransactionOrigin,
  UndoHistoryStep,
  UndoOutcome,
  UndoTarget,
  UserAction,
} from '../core/types';
import { LOCALE, t, tCount } from '../i18n';
import { hourCycle } from './clock.svelte';

/**
 * An undo to run (spec: US-146): what it undoes, how the toast and the
 * question before a Dangerous Undo name it, and whether it may take a while.
 */
export interface Undoing {
  target: UndoTarget;
  /** The change it undoes, or redoes, as a phrase: `deleting “Standup”`. */
  change: string;
  /**
   * Whether it runs under "Working…": every undo but that of one user
   * action or of one Event, which are quick.
   */
  slow: boolean;
}

/** What an undo did. */
export type UndoResult =
  /** The user didn't approve a Dangerous Undo; nothing changed. */
  | { kind: 'cancelled' }
  /** It ran; `changed` says whether it changed anything. */
  | { kind: 'done'; changed: boolean; skipped: SkippedEvent[] }
  /** The core refused it; it can be tried again unless it was undone already. */
  | { kind: 'failed'; error: unknown; retryable: boolean };

/** How an undo asks the user before a Dangerous Undo, and runs a core undo. */
export interface UndoHooks {
  /** Asks whether to run the Dangerous Undo `danger`. */
  approve: (danger: DangerousUndo) => Promise<boolean>;
  /** Runs one undo of the core, such as under "Working…". */
  running: <T>(run: () => Promise<T>) => Promise<T>;
}

/**
 * Undoes `target`, asking first if it is a Dangerous Undo, and again if it
 * changed since the last approval, such as by a sync.
 */
export async function runUndo(
  core: CoreApi,
  target: UndoTarget,
  hooks: UndoHooks,
): Promise<UndoResult> {
  try {
    let approved: DangerousUndo | null = null;
    for (;;) {
      const danger: DangerousUndo | null = approved;
      const { journalTransaction, value }: Changed<UndoOutcome> = await hooks.running(() =>
        core.undo(target, danger),
      );
      if (value.kind === 'undone') {
        return { kind: 'done', changed: journalTransaction !== null, skipped: value.skipped };
      }
      if (!(await hooks.approve(value.danger))) return { kind: 'cancelled' };
      approved = value.danger;
    }
  } catch (error) {
    return { kind: 'failed', error, retryable: !undoneAlready(error) };
  }
}

/** Whether `error` is the core's refusal of an undo whose changes are all undone already. */
function undoneAlready(error: unknown): boolean {
  return (
    typeof error === 'object' && error !== null && 'code' in error && error.code === 'alreadyUndone'
  );
}

const titled = (title: string) => ({ title: title || t('event.untitled') });

/** How a toast names the user action `action` of the Event titled `title`. */
export function actionText(action: UserAction, title: string): string {
  return t(`undo.action.${action}`, titled(title));
}

/**
 * How a toast names a Journal Transaction from `origin` that changed
 * `events` Events, the first of them titled `titles`: a user action or an
 * undo by its first title, the others by their number of Events.
 */
function transactionText(origin: TransactionOrigin, titles: string[], events: number): string {
  const title = titles[0] ?? '';
  switch (origin.kind) {
    case 'userAction':
      return actionText(origin.action, title);
    case 'undo':
      return t('undo.action.undo', titled(title));
    case 'import':
      return tCount('undo.action.imported', events);
    case 'incomingSync':
      return tCount('undo.action.incomingSync', events);
    case 'firstSync':
      return tCount('undo.action.firstSync', events);
  }
}

/** Undoing the whole Journal Transaction `id` from `origin`, as `transactionText` names it. */
function wholeUndoing(
  id: ListedJournalTransaction['id'],
  origin: TransactionOrigin,
  titles: string[],
  events: number,
): Undoing {
  return {
    target: { kind: 'journalTransaction', id },
    change: transactionText(origin, titles, events),
    slow: origin.kind !== 'userAction',
  };
}

/** Undoing what is left of the listed Journal Transaction `listed` ("Undo all"). */
export function transactionUndoing(listed: ListedJournalTransaction): Undoing {
  return wholeUndoing(listed.id, listed.origin, listed.titles, listed.events);
}

/**
 * Undoing the Undo History Step `step`, named after the user change it
 * undoes or redoes; one about none, such as a redo of an undo from the
 * Change Journal list, is a change from the Change Journal.
 */
export function stepUndoing(step: UndoHistoryStep): Undoing {
  const { userChange } = step;
  const target = { kind: 'journalTransaction', id: step.journalTransaction } as const;
  if (!userChange) return { target, change: t('undo.action.unnamed'), slow: true };
  return { ...transactionUndoing(userChange), target };
}

/**
 * What the Undo of the listed Journal Event Modification `modification`
 * runs, none once it is undone: its Event alone, named by what happened to
 * it, or its whole Journal Transaction.
 */
export function modificationUndoing(modification: ListedJournalEventModification): Undoing | null {
  const { undo: target, title, origin } = modification;
  if (!target) return null;
  if (target.kind === 'event') {
    return { target, change: t(`undo.action.${modification.kind}`, titled(title)), slow: false };
  }
  if (target.kind === 'journalTransaction') return wholeUndoing(target.id, origin, [title], 1);
  return { target, change: t('undo.action.unnamed'), slow: true };
}

/** Undoing everything in `calendar` since `since`, in Unix seconds. */
export function calendarSinceUndoing(calendar: Calendar, since: number): Undoing {
  return {
    target: { kind: 'calendarSince', calendarId: calendar.id, since },
    change: t('undo.action.calendarSince', {
      calendar: calendar.name,
      time: journalTimeText(since),
    }),
    slow: true,
  };
}

/** When a Journal Transaction was made, `madeAt` in Unix seconds, as the user reads it. */
export function journalTimeText(madeAt: number): string {
  return new Intl.DateTimeFormat(LOCALE, {
    dateStyle: 'medium',
    timeStyle: 'short',
    hourCycle: hourCycle(),
  }).format(madeAt * 1000);
}
