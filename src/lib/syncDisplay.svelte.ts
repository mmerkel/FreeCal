import type { OverallState, SyncStatus, SyncStatusReport, SyncStatuses } from '../core/types';
import { hourCycle } from './clock.svelte';
import { asDate } from './when';

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

/**
 * When the last sync finished, as the status indicator says it: the time on
 * FreeCal's clock, with the date when it wasn't `today`.
 */
export function lastSyncText(at: string, today: Temporal.PlainDate): string {
  const time = Temporal.PlainDateTime.from(at);
  const format = new Intl.DateTimeFormat(undefined, {
    dateStyle: time.toPlainDate().equals(today) ? undefined : 'medium',
    timeStyle: 'short',
    hourCycle: hourCycle(),
  });
  return format.format(asDate(time));
}
