import { describe, expect, test } from 'vitest';
import { samples } from '../core/samples.generated';
import { tauriCore } from '../core/tauriCore';
import type { Calendar, EventDraft, Signal } from '../core/types';
import { accountId, calendarId, eventId, journalTransactionId } from './ids';
import { failure, FakeCore, first, settled } from './fakeCore';

const draft = first<EventDraft>(samples.EventDraft);

describe('a call nobody scripted', () => {
  test('answers with the generated samples of its type', async () => {
    const core = new FakeCore();

    expect(await core.listCalendars()).toEqual(samples.Calendar);
    expect(await core.event(eventId(3))).toEqual(samples.Event[0]);
    expect(await core.settings()).toEqual(samples.Settings[0]);
    expect(await core.createEvent(draft)).toEqual(samples.Changed[0]);
    expect(await core.editOccurrence(eventId(3), 'r', 'this', false, draft)).toEqual({
      journalTransaction: samples.Changed[0].journalTransaction,
      value: samples.Occurrence[0],
    });
  });

  test('answers with nothing where the answer can be nothing', async () => {
    const core = new FakeCore();

    expect(await core.startupFailure()).toBeNull();
    expect(await core.pickImportFile('Import', 'Calendar files')).toBeNull();
    expect(await core.setCalendarShown(calendarId(2), false)).toBeUndefined();
  });

  test('answers today with the date of the sample "Date changed"', async () => {
    const core = new FakeCore();
    const dateChanged = samples.Signal.find((signal) => signal.kind === 'dateChanged');

    const today = await core.today();

    expect(today).toBeInstanceOf(Temporal.PlainDate);
    expect(dateChanged && today.equals(dateChanged.date)).toBe(true);
  });

  test('answers each call with a copy, so that changing one changes no other', async () => {
    const core = new FakeCore();

    (await core.listCalendars())[0].name = 'Changed';

    expect((await core.listCalendars())[0].name).toBe(samples.Calendar[0].name);
    expect(samples.Calendar[0].name).not.toBe('Changed');
  });

  test('exists for every command of the core interface', () => {
    const core = new FakeCore();

    for (const method of Object.keys(tauriCore)) {
      expect(core[method as keyof typeof tauriCore], method).toBeTypeOf('function');
    }
  });
});

describe('a scripted answer', () => {
  test('answers every later call with its value', async () => {
    const core = new FakeCore();
    core.answer('listCalendars', []);

    expect(await core.listCalendars()).toEqual([]);
    expect(await core.listCalendars()).toEqual([]);
  });

  test('rejects with its error code', async () => {
    const core = new FakeCore();
    core.answer('event', failure({ code: 'eventNotFound' }));

    await expect(core.event(eventId(3))).rejects.toEqual({ code: 'eventNotFound' });
  });

  test('tells an error code answered as a value from one to fail with', async () => {
    const core = new FakeCore();
    core.answer('startupFailure', { code: 'localStoreInUse' });

    expect(await core.startupFailure()).toEqual({ code: 'localStoreInUse' });
  });

  test('answers each call with a copy, so that changing one changes no other', async () => {
    const core = new FakeCore();
    const calendars = [{ ...samples.Calendar[0], name: 'Home' }] as Calendar[];
    core.answer('listCalendars', calendars);

    (await core.listCalendars())[0].name = 'Changed';

    expect((await core.listCalendars())[0].name).toBe('Home');
    expect(calendars[0].name).toBe('Home');
  });

  test('gives way to the next one', async () => {
    const core = new FakeCore();
    core.answer('undoJournalTransaction', failure({ code: 'localStoreFailed' }));
    await expect(core.undoJournalTransaction(journalTransactionId(1))).rejects.toEqual({
      code: 'localStoreFailed',
    });

    core.answer('undoJournalTransaction', { journalTransaction: null, value: null });

    expect(await core.undoJournalTransaction(journalTransactionId(1))).toEqual({
      journalTransaction: null,
      value: null,
    });
  });
});

describe('a pending answer', () => {
  test('answers the next call when the test gives it', async () => {
    const core = new FakeCore();
    const answer = core.pending('listCalendars');
    let calendars: unknown;

    void core.listCalendars().then((value) => (calendars = value));
    await settled();
    expect(calendars).toBeUndefined();

    answer.resolve([]);
    await settled();
    expect(calendars).toEqual([]);
  });

  test('belong to the calls in the order they come, and settle in the order they are given', async () => {
    const core = new FakeCore();
    const first = core.pending('listOccurrences');
    const second = core.pending('listOccurrences');
    const order: string[] = [];
    const from = Temporal.PlainDate.from('2026-10-01');
    const to = Temporal.PlainDate.from('2026-11-01');

    void core.listOccurrences(from, to).then(() => order.push('first'));
    void core.listOccurrences(from, to).then(() => order.push('second'));
    second.resolve([]);
    await settled();
    first.resolve([]);
    await settled();

    expect(order).toEqual(['second', 'first']);
  });

  test('can be given before the call comes', async () => {
    const core = new FakeCore();
    core.pending('version').resolve('9.9.9');

    expect(await core.version()).toBe('9.9.9');
  });

  test('can fail with an error code', async () => {
    const core = new FakeCore();
    const answer = core.pending('runImport');

    const run = core.runImport(calendarId(2), 'update');
    answer.fail({ code: 'calendarReadOnly' });

    await expect(run).rejects.toEqual({ code: 'calendarReadOnly' });
  });

  test('answers with the default when given no value', async () => {
    const core = new FakeCore();
    const answer = core.pending('listCalendars');

    const calendars = core.listCalendars();
    answer.resolve();

    expect(await calendars).toEqual(samples.Calendar);
  });

  test('comes before a scripted answer, which answers the calls after it', async () => {
    const core = new FakeCore();
    core.answer('version', '1.0.0');
    core.pending('version').resolve('2.0.0');

    expect(await core.version()).toBe('2.0.0');
    expect(await core.version()).toBe('1.0.0');
  });
});

describe('the recorded calls', () => {
  test('list every call with its arguments, in order', async () => {
    const core = new FakeCore();

    await core.renameCalendar(calendarId(2), 'Family');
    await core.listCalendars();
    await core.setCalendarShown(calendarId(2), false);

    expect(core.calls).toEqual([
      { method: 'renameCalendar', args: [calendarId(2), 'Family'] },
      { method: 'listCalendars', args: [] },
      { method: 'setCalendarShown', args: [calendarId(2), false] },
    ]);
  });

  test('are read for one method the same way for every method', async () => {
    const core = new FakeCore();

    await core.setSidebar({ shown: false, width: 300 });
    await core.listCalendars();
    await core.setSidebar({ shown: true, width: 300 });

    expect(core.callsOf('setSidebar')).toEqual([
      [{ shown: false, width: 300 }],
      [{ shown: true, width: 300 }],
    ]);
    expect(core.callsOf('listCalendars')).toEqual([[]]);
    expect(core.callsOf('quit')).toEqual([]);
  });

  test('keep the arguments as they were at the call', async () => {
    const core = new FakeCore();
    const sidebar = { shown: true, width: 300 };

    await core.setSidebar(sidebar);
    sidebar.width = 400;

    expect(core.callsOf('setSidebar')).toEqual([[{ shown: true, width: 300 }]]);
  });

  test('include a call still waiting for its answer', () => {
    const core = new FakeCore();
    core.pending('event');

    void core.event(eventId(3));

    expect(core.callsOf('event')).toEqual([[eventId(3)]]);
  });
});

describe('Signals', () => {
  test('reach every subscriber until it unsubscribes, and only when the test emits them', async () => {
    const core = new FakeCore();
    const received: Signal[] = [];
    const unsubscribe = await core.subscribe((signal) => received.push(signal));

    await core.createCalendar(accountId(1), 'Home', '#3366cc');
    await core.createEvent(draft);
    core.emit({ kind: 'calendarsChanged' });
    unsubscribe();
    core.emit({ kind: 'occurrencesChanged', calendarIds: [calendarId(2)] });

    expect(received).toEqual([{ kind: 'calendarsChanged' }]);
  });

  test('reach a subscriber only once its subscribe has answered', async () => {
    const core = new FakeCore();
    const received: Signal[] = [];
    const answer = core.pending('subscribe');

    const subscribed = core.subscribe((signal) => received.push(signal));
    core.emit({ kind: 'calendarsChanged' });
    answer.resolve();
    await subscribed;
    core.emit({ kind: 'occurrencesChanged', calendarIds: [] });

    expect(received).toEqual([{ kind: 'occurrencesChanged', calendarIds: [] }]);
  });
});
