import type {
  AccountId,
  Calendar,
  Occurrence,
  SendFailure,
  SyncStatus,
  SyncStatuses,
  UnsentChange,
} from '../core/types';
import { errorText, t, type MessageKey } from '../i18n';

/**
 * How long an Unsent Change waits before its Occurrence gets the dashed
 * outline, so that the outline doesn't flash on every edit sent at once
 * (spec: "Event chips stay sparse").
 */
export const OUTLINE_DELAY_MS = 5000;

/**
 * When the Occurrence's dashed outline is due, in epoch milliseconds: at
 * once while its Account is Offline (`offline`), otherwise once its Unsent
 * Change waited {@link OUTLINE_DELAY_MS}. Undefined when it has none.
 */
export function outlineDue(occurrence: Occurrence, offline: boolean): number | undefined {
  if (occurrence.unsentSince === null) return undefined;
  if (offline) return Number.NEGATIVE_INFINITY;
  return Temporal.Instant.from(occurrence.unsentSince).epochMilliseconds + OUTLINE_DELAY_MS;
}

/** The server Accounts that are Offline now. */
export function offlineAccounts(statuses: SyncStatuses | undefined): Set<AccountId> {
  return new Set(
    statuses?.accounts
      .filter((account) => account.report.status.kind === 'offline')
      .map((account) => account.accountId),
  );
}

/** A Sync Status's own reason for holding Unsent Changes back, if it has one. */
function reasonOf(status: SyncStatus | undefined): string | undefined {
  switch (status?.kind) {
    case 'offline':
      return t('unsent.offline');
    case 'syncError':
      return errorText(status.error);
    default:
      return undefined;
  }
}

/**
 * Why the Unsent Changes of `calendar` wait, as its Account's and its own
 * Sync Status say: the Account Offline or with a Sync Error, the Calendar
 * with a Sync Error, or its First Sync unfinished, which sends nothing.
 * The Account's comes first. Undefined when none says.
 */
export function heldBackBy(
  statuses: SyncStatuses | undefined,
  calendar: Calendar | undefined,
): string | undefined {
  if (!statuses || !calendar) return undefined;
  const account = statuses.accounts.find((status) => status.accountId === calendar.accountId);
  const own = statuses.calendars.find((status) => status.calendarId === calendar.id);
  return (
    reasonOf(account?.report.status) ??
    reasonOf(own?.report.status) ??
    (own?.firstSyncUnfinished ? t('unsent.firstSyncUnfinished') : undefined)
  );
}

const sendFailureMessages: Record<Exclude<SendFailure['kind'], 'failed'>, MessageKey> = {
  serverChanged: 'unsent.serverChanged',
  addressTaken: 'unsent.addressTaken',
  waitingForServerVersion: 'unsent.waitingForServerVersion',
  unbuildable: 'unsent.unbuildable',
};

/** A Send Failure in words. */
function sendFailureText(failure: SendFailure): string {
  return failure.kind === 'failed'
    ? errorText(failure.error)
    : t(sendFailureMessages[failure.kind]);
}

/**
 * What an Event's details say about its Unsent Change (spec: "Event
 * details"): "Not yet synced: <reason>", the reason being its Account's or
 * Calendar's (see {@link heldBackBy}), which a stored Send Failure may be
 * older than, or else that Send Failure; just "Not yet synced" when neither
 * says why.
 */
export function notYetSyncedText(
  unsent: UnsentChange,
  statuses: SyncStatuses | undefined,
  calendar: Calendar | undefined,
): string {
  const failure = unsent.sendFailure;
  const reason =
    heldBackBy(statuses, calendar) ?? (failure === null ? undefined : sendFailureText(failure));
  return reason === undefined
    ? t('unsent.notYetSynced')
    : t('unsent.notYetSyncedBecause', { reason });
}

/** A Calendar whose Unsent Changes are still waiting at quit, with why. */
export interface UnsentAtQuit {
  calendar: Calendar;
  count: number;
  reason: string;
}

/**
 * The Calendars of `calendars` whose Unsent Changes still wait, as
 * `statuses` count them, each with why (see {@link heldBackBy}); one still
 * syncing is "still sending", any other "not sent yet".
 */
export function unsentAtQuit(statuses: SyncStatuses, calendars: Calendar[]): UnsentAtQuit[] {
  return statuses.calendars.flatMap((status) => {
    const count = status.report.unsentChanges;
    const calendar = calendars.find((calendar) => calendar.id === status.calendarId);
    if (count === 0 || !calendar) return [];
    const reason =
      heldBackBy(statuses, calendar) ??
      t(status.report.status.kind === 'syncing' ? 'unsent.stillSending' : 'unsent.notSent');
    return [{ calendar, count, reason }];
  });
}

/** The Unsent Changes of every Calendar, as `statuses` count them. */
export function unsentCount(statuses: SyncStatuses): number {
  return statuses.calendars.reduce((sum, status) => sum + status.report.unsentChanges, 0);
}
