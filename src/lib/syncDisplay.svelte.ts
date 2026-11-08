import type {
  OverallState,
  OverallStatus,
  SyncStatus,
  SyncStatusReport,
  SyncStatuses,
} from '../core/types';
import { LOCALE, t, type MessageKey } from '../i18n';

/**
 * How long a sync FreeCal started itself runs before the window shows it as
 * Syncing, so that routine polls don't make the display flicker (spec:
 * "Overall status"). A sync the user started shows at once.
 */
export const SYNCING_DELAY_MS = 1000;

/**
 * Holds back the Syncing of a sync FreeCal started until it has run for
 * `SYNCING_DELAY_MS`, for the overall status and each server Account and
 * Calendar alike. Only the display waits; the Signals came straight away.
 */
export class SyncingDelay {
  /** Each Syncing held back, or come due, by `key`. */
  private waits = new Map<string, { timer: ReturnType<typeof setTimeout>; due: boolean }>();
  /** Counts held-back Syncing states coming due, so that what reads `shown` reads it again then. */
  private dues = $state(0);

  /**
   * `statuses` as the window shows them now: a Syncing still held back
   * shows as the overall status Synced, and on an Account or Calendar as
   * its last sync ended.
   */
  shown(statuses: SyncStatuses): SyncStatuses {
    void this.dues;
    const syncing = new Set<string>();
    const overall = this.held('overall', statuses.overall.state, syncing)
      ? { ...statuses.overall, state: { kind: 'synced' } satisfies OverallState }
      : statuses.overall;
    const report = (key: string, report: SyncStatusReport): SyncStatusReport =>
      this.held(key, report.status, syncing)
        ? { ...report, status: report.lastSync?.outcome ?? { kind: 'synced' } }
        : report;
    const shown = {
      overall,
      accounts: statuses.accounts.map((account) => ({
        ...account,
        report: report(`account ${account.accountId}`, account.report),
      })),
      calendars: statuses.calendars.map((calendar) => ({
        ...calendar,
        report: report(`calendar ${calendar.calendarId}`, calendar.report),
      })),
    };
    for (const key of [...this.waits.keys()]) {
      if (!syncing.has(key)) this.forget(key);
    }
    return shown;
  }

  /** Stops waiting for any Syncing. */
  stop() {
    for (const key of [...this.waits.keys()]) this.forget(key);
  }

  /** Whether `key`, which is in `state`, is Syncing still held back; notes it in `syncing`. */
  private held(key: string, state: SyncStatus | OverallState, syncing: Set<string>): boolean {
    if (state.kind !== 'syncing') return false;
    syncing.add(key);
    if (state.byUser) return false;
    const wait = this.waits.get(key);
    if (wait) return !wait.due;
    const started = {
      due: false,
      timer: setTimeout(() => {
        started.due = true;
        this.dues++;
      }, SYNCING_DELAY_MS),
    };
    this.waits.set(key, started);
    return true;
  }

  private forget(key: string) {
    clearTimeout(this.waits.get(key)?.timer);
    this.waits.delete(key);
  }
}

/** Each overall state in words. */
const overallStateLabels: Record<OverallState['kind'], MessageKey> = {
  conflict: 'sync.conflict',
  syncError: 'sync.errorShort',
  offline: 'sync.offline',
  syncing: 'sync.syncing',
  synced: 'sync.synced',
};

/**
 * What the status indicator says: the overall state, with how long ago
 * the last sync finished, seen at `now`. Under a minute it says the state
 * alone, so that no time counts up every second.
 */
export function overallSyncText(overall: OverallStatus, now: Temporal.Instant): string {
  const state = t(overallStateLabels[overall.state.kind]);
  if (!overall.lastSync) return state;
  const lastSync = Temporal.Instant.from(overall.lastSync);
  if (now.epochMilliseconds - lastSync.epochMilliseconds < 60_000) return state;
  const ago = timeAgo(lastSync, now);
  return overall.state.kind === 'synced'
    ? t('sync.syncedAgo', { ago })
    : t('sync.lastSync', { state, ago });
}

/** The units the time ago is in, smallest first, with their length in seconds. */
const units = [
  { unit: 'second', seconds: 1 },
  { unit: 'minute', seconds: 60 },
  { unit: 'hour', seconds: 60 * 60 },
  { unit: 'day', seconds: 24 * 60 * 60 },
] as const;

/**
 * How long before `now` the instant `since` was, as "3 minutes ago": in
 * the largest unit it is at least one whole of, rounded half up, and in
 * the next unit when it rounds up to that. Counting instants, not wall
 * times, keeps a DST change out of it. A `since` after `now`, from a clock
 * that went back, is 0 seconds ago.
 */
export function timeAgo(since: Temporal.Instant, now: Temporal.Instant): string {
  const seconds = Math.max(0, now.epochMilliseconds - since.epochMilliseconds) / 1000;
  let index = 0;
  while (index + 1 < units.length && seconds >= units[index + 1].seconds) index++;
  let count = Math.floor(seconds / units[index].seconds + 0.5);
  const next = units[index + 1];
  if (next && count * units[index].seconds >= next.seconds) {
    index++;
    count = Math.floor(seconds / next.seconds + 0.5);
  }
  return new Intl.RelativeTimeFormat(LOCALE, { numeric: 'always' }).format(
    -count,
    units[index].unit,
  );
}
