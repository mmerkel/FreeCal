import { describe, expect, test, vi } from 'vitest';
import { samples } from '../core/samples.generated';
import type { Account, Event, Occurrence, Signal, SyncStatuses } from '../core/types';
import { calendar, dateChanged } from '../test/answers';
import { eventId } from '../test/ids';
import { branded, failure, FakeCore, first, settled } from '../test/fakeCore';
import { whyReadOnly, WindowModel } from './windowModel.svelte';

const date = (iso: string) => Temporal.PlainDate.from(iso);
/** The sample Signal of `kind`. */
const signal = (kind: Signal['kind']) =>
  branded<Signal>(samples.Signal.find((found) => found.kind === kind)!);
const range = { from: date('2026-10-05'), to: date('2026-10-12') };
const sampleEvent = first<Event>(samples.Event);
const sampleOccurrence = first<Occurrence>(samples.Occurrence);
const sampleAccount = first<Account>(samples.Account);
const sampleStatuses = first<SyncStatuses>(samples.SyncStatuses);

/** A Window Model on `core`, started and done with its startup. */
async function started(core = new FakeCore()) {
  const model = new WindowModel(core);
  const stop = model.start();
  await settled();
  return { core, model, stop };
}

describe('startup', () => {
  test('subscribes before its first read, and shows the window once the settings are read', async () => {
    const { core } = await started();
    const methods = core.calls.map((call) => call.method);

    expect(methods.slice(0, 4)).toEqual(['startupFailure', 'subscribe', 'settings', 'showWindow']);
    expect(methods.slice(4).sort()).toEqual(
      ['today', 'lastUsedView', 'sidebar', 'listAccounts', 'listCalendars', 'syncStatuses'].sort(),
    );
  });

  test('shows what it read, opening the last-used view on today', async () => {
    const { model } = await started();

    expect(model.settings).toEqual(samples.Settings[0]);
    expect(model.today?.toString()).toBe('2026-10-04');
    expect(model.navigation).toEqual({ view: 'day', date: date('2026-10-04'), tracking: true });
    expect(model.sidebar).toEqual(samples.SidebarState[0]);
    expect(model.directory).toEqual({ accounts: samples.Account, calendars: samples.Calendar });
    expect(model.syncStatuses).toEqual(samples.SyncStatuses[0]);
    expect(model.startupReason).toBeUndefined();
  });

  test('on a startup failure, says why and shows the window without subscribing', async () => {
    const core = new FakeCore();
    core.answer('startupFailure', { code: 'localStoreInUse' });

    const { model } = await started(core);

    expect(model.startupReason).toBeTruthy();
    expect(core.calls.map((call) => call.method)).toEqual(['startupFailure', 'showWindow']);
  });

  test('when a first read fails, says why FreeCal could not load', async () => {
    const core = new FakeCore();
    core.answer('listCalendars', failure({ code: 'localStoreFailed' }));

    const { model } = await started(core);

    expect(model.startupReason).toBeTruthy();
  });

  test('a "Date changed" during startup is newer than the date read', async () => {
    const core = new FakeCore();
    const today = core.pending('today');
    const model = new WindowModel(core);
    model.start();
    await settled();

    dateChanged(core, '2026-10-06');
    today.resolve();
    await settled();

    expect(model.navigation?.date.toString()).toBe('2026-10-06');
  });

  test('stopped, it hears no more Signals', async () => {
    const { core, model, stop } = await started();
    stop();
    core.calls.splice(0);

    core.emit(signal('calendarsChanged'));
    dateChanged(core, '2026-10-06');
    await settled();

    expect(core.calls).toEqual([]);
    expect(model.today?.toString()).toBe('2026-10-04');
  });
});

describe('an older read never overwrites a newer one', () => {
  test('of the Accounts and Calendars', async () => {
    const { core, model } = await started();
    const older = core.pending('listAccounts');
    const newer = core.pending('listAccounts');
    core.emit(signal('calendarsChanged'));
    core.emit(signal('calendarsChanged'));

    newer.resolve([]);
    await settled();
    older.resolve([sampleAccount]);
    await settled();

    expect(model.directory?.accounts).toEqual([]);
  });

  test('of the Sync Statuses', async () => {
    const { core, model } = await started();
    const older = core.pending('syncStatuses');
    const newer = core.pending('syncStatuses');
    core.emit(signal('accountSyncStatusChanged'));
    core.emit(signal('calendarSyncStatusChanged'));

    newer.resolve({ ...sampleStatuses, accounts: [] });
    await settled();
    older.resolve(sampleStatuses);
    await settled();

    expect(model.syncStatuses?.accounts).toEqual([]);
  });

  test('of the Occurrences of the visible range', async () => {
    const { core, model } = await started();
    const older = core.pending('listOccurrences');
    const newer = core.pending('listOccurrences');
    model.showRange(range);
    model.showRange({ ...range, to: date('2026-10-13') });

    newer.resolve([]);
    await settled();
    older.resolve([sampleOccurrence]);
    await settled();

    expect(model.occurrences).toEqual([]);
  });

  test('of the Event whose details are open', async () => {
    const { core, model } = await started();
    const older = core.pending('event');
    const newer = core.pending('event');
    model.openDetails(eventId(3), () => {});
    core.emit(signal('occurrencesChanged'));

    newer.resolve({ ...sampleEvent, title: 'Newer' });
    await settled();
    older.resolve({ ...sampleEvent, title: 'Older' });
    await settled();

    expect(model.opened?.title).toBe('Newer');
  });

  test('nor does an older read’s failure', async () => {
    const { core, model } = await started();
    const older = core.pending('listOccurrences');
    const newer = core.pending('listOccurrences');
    model.showRange(range);
    model.showRange(range);

    newer.resolve([]);
    await settled();
    older.fail({ code: 'localStoreFailed' });
    await settled();

    expect(model.readFailure).toBeUndefined();
  });

  test('the newest read’s failure shows, until the Accounts and Calendars read again', async () => {
    const { core, model } = await started();
    core.answer('listOccurrences', failure({ code: 'localStoreFailed' }));
    model.showRange(range);
    await settled();

    expect(model.readFailure).toBeTruthy();

    core.answer('listOccurrences', []);
    core.emit(signal('calendarsChanged'));
    await settled();

    expect(model.readFailure).toBeUndefined();
  });
});

describe('Signals', () => {
  test.each([
    {
      kind: 'calendarsChanged',
      reads: ['event', 'listAccounts', 'listCalendars', 'listOccurrences', 'syncStatuses'],
    },
    { kind: 'occurrencesChanged', reads: ['event', 'listOccurrences'] },
    { kind: 'accountSyncStatusChanged', reads: ['syncStatuses'] },
    { kind: 'calendarSyncStatusChanged', reads: ['syncStatuses'] },
    { kind: 'dateChanged', reads: [] },
  ] as const)('$kind re-reads $reads', async ({ kind, reads }) => {
    const { core, model } = await started();
    model.showRange(range);
    model.openDetails(eventId(3), () => {});
    await settled();
    core.calls.splice(0);

    core.emit(signal(kind));
    await settled();

    expect(core.calls.map((call) => call.method).sort()).toEqual(reads);
  });

  test('the Event of closed details is not read again', async () => {
    const { core, model } = await started();
    model.openDetails(eventId(3), () => {});
    model.closeDetails();
    await settled();
    core.calls.splice(0);

    core.emit(signal('occurrencesChanged'));
    await settled();

    expect(core.callsOf('event')).toEqual([]);
    expect(model.opened).toBeUndefined();
  });

  test('an Event that cannot be read any more says its details are gone', async () => {
    const { core, model } = await started();
    const ongone = vi.fn();
    model.openDetails(eventId(3), ongone);
    await settled();

    core.answer('event', failure({ code: 'eventNotFound' }));
    core.emit(signal('occurrencesChanged'));
    await settled();

    expect(ongone).toHaveBeenCalledOnce();
  });
});

describe('a "Date changed"', () => {
  test('moves a view that tracks today at once', async () => {
    const { core, model } = await started();

    dateChanged(core, '2026-10-05');

    expect(model.today?.toString()).toBe('2026-10-05');
    expect(model.navigation?.date.toString()).toBe('2026-10-05');
  });

  test('held by a press and by a drop’s scope prompt moves the view once the last hold is released', async () => {
    const { core, model } = await started();
    const releasePress = model.hold();
    const releaseScopePrompt = model.hold();

    dateChanged(core, '2026-10-05');
    releasePress();
    releasePress();

    expect(model.today?.toString()).toBe('2026-10-05');
    expect(model.navigation?.date.toString()).toBe('2026-10-04');

    releaseScopePrompt();

    expect(model.navigation?.date.toString()).toBe('2026-10-05');
  });

  test('released without a "Date changed" leaves the view where it is', async () => {
    const { model } = await started();
    const navigation = model.navigation;

    model.hold()();

    expect(model.navigation).toBe(navigation);
  });
});

describe('the window’s own state', () => {
  test('a changed view shows at once and is remembered; the same view is not written again', async () => {
    const { core, model } = await started();
    const shown = model.navigation!;

    model.navigate({ ...shown, view: 'week' });
    model.navigate({ ...shown, view: 'week', date: date('2026-10-11'), tracking: false });

    expect(model.navigation?.view).toBe('week');
    expect(core.callsOf('setLastUsedView')).toEqual([['week']]);
  });

  test('a view that could not be remembered says so', async () => {
    const { core, model } = await started();
    core.answer('setLastUsedView', failure({ code: 'localStoreFailed' }));

    model.navigate({ ...model.navigation!, view: 'month' });
    await settled();

    expect(model.navigation?.view).toBe('month');
    expect(model.viewSaveFailure).toBeTruthy();
  });

  test('the sidebar shows at once and is remembered', async () => {
    const { core, model } = await started();
    const write = core.pending('setSidebar');

    model.toggleSidebar();

    expect(model.sidebar?.shown).toBe(false);
    expect(core.callsOf('setSidebar')).toEqual([[{ shown: false, width: null }]]);

    write.fail({ code: 'localStoreFailed' });
    await settled();

    expect(model.sidebarSaveFailure).toBeTruthy();
  });

  test('a setting takes effect before it is written', async () => {
    const { core, model } = await started();
    core.pending('updateSettings');

    model.changeSettings({ appearance: 'light' });

    expect(model.settings.appearance).toBe('light');
    expect(core.callsOf('updateSettings')).toEqual([[{ appearance: 'light' }]]);
  });

  test('a failed settings write puts back the stored settings', async () => {
    const { core, model } = await started();
    core.answer('updateSettings', failure({ code: 'localStoreFailed' }));
    core.answer('settings', { appearance: 'system' });

    model.changeSettings({ appearance: 'light' });
    await settled();

    expect(model.settings).toEqual({ appearance: 'system' });
    expect(model.settingsFailure).toBeTruthy();

    model.forgetSettingsFailure();

    expect(model.settingsFailure).toBeUndefined();
  });
});

describe('whyReadOnly', () => {
  test('lets an Event without guests in a writable Calendar be edited', () => {
    expect(whyReadOnly(calendar({ name: 'Work' }), false)).toBeUndefined();
  });

  test('refuses an Event in a Read-only Calendar, whatever made it read-only', () => {
    expect(whyReadOnly(calendar({ name: 'Work', readOnly: true }), false)).toBe('calendarReadOnly');
    expect(
      whyReadOnly(calendar({ name: 'Work', readOnly: true, serverReadOnly: true }), false),
    ).toBe('calendarReadOnly');
  });

  test('refuses an Event with guests, even before its Calendar is read', () => {
    expect(whyReadOnly(calendar({ name: 'Work' }), true)).toBe('eventHasGuests');
    expect(whyReadOnly(undefined, true)).toBe('eventHasGuests');
  });

  test('names a Read-only Calendar before guests, as the user can change that', () => {
    expect(whyReadOnly(calendar({ name: 'Work', readOnly: true }), true)).toBe('calendarReadOnly');
  });
});
