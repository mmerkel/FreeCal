import { describe, expect, test } from 'vitest';
import { samples } from '../core/samples.generated';
import type {
  Calendar,
  Changed,
  Event,
  EventDraft,
  Occurrence,
  Repeat,
  Signal,
} from '../core/types';
import { dateChanged } from '../test/answers';
import { calendarId, eventId } from '../test/ids';
import { branded, failure, FakeCore, first, settled } from '../test/fakeCore';
import { WindowDialogs } from './windowDialogs.svelte';
import { WindowModel } from './windowModel.svelte';

const sampleEvent = first<Event>(samples.Event);
/** The sample Occurrence of a Recurring Event. */
const recurring = branded<Occurrence>(
  samples.Occurrence.find((occurrence) => occurrence.recurrenceId !== null)!,
);
const recurrenceId = recurring.recurrenceId!;
const weekly = first<Repeat>(samples.Repeat);
const when = branded<Event['when']>(samples.When[1]);
const changedEvent = first<Changed<Event>>(samples.Changed);
const file = '/home/user/a.ics';

/** Window Dialogs on a started Window Model, which shows the sample Occurrences. */
async function started(core = new FakeCore()) {
  const model = new WindowModel(core);
  model.start();
  await settled();
  model.showRange({
    from: Temporal.PlainDate.from('2026-10-05'),
    to: Temporal.PlainDate.from('2026-10-12'),
  });
  await settled();
  const dialogs = new WindowDialogs(core, model);
  core.calls.splice(0);
  return { core, model, dialogs };
}

/** Opens the editor for the Occurrence `recurrenceId` of a Recurring Event repeating `repeat`. */
async function editingOccurrence(repeat: Repeat | null = weekly) {
  const opened = started();
  const { core, dialogs } = await opened;
  core.answer('event', { ...sampleEvent, repeat });
  dialogs.openDetails(eventId(3), recurrenceId);
  await settled();
  dialogs.startEditing();
  core.calls.splice(0);
  return opened;
}

/** The draft the open editor started from. */
const draftOf = (dialogs: WindowDialogs): EventDraft => {
  const open = dialogs.open;
  if (open?.kind !== 'editing' && open?.kind !== 'creating') throw new Error('No editor is open');
  return open.draft;
};

describe('the Scope Prompt of an edit', () => {
  test('asks once the Exceptions are counted, and saves for the scope chosen', async () => {
    const { core, dialogs } = await editingOccurrence();
    const draft = { ...draftOf(dialogs), title: 'Changed' };
    const counts = core.pending('exceptionCounts');
    core.answer('editOccurrence', { ...changedEvent, value: recurring });

    const saved = dialogs.save(draft);
    await settled();

    expect(dialogs.scopePrompt).toEqual({ change: 'edit' });
    expect(dialogs.isOpen).toBe(true);

    counts.resolve({ thisAndFollowing: 4, all: 5 });
    await settled();

    expect(dialogs.scopePrompt).toEqual({
      change: 'edit',
      counts: { thisAndFollowing: 4, all: 5 },
    });

    dialogs.chooseScope({ scope: 'thisAndFollowing', exceptionsToo: true });
    await saved;

    expect(core.callsOf('exceptionCounts')).toEqual([[eventId(3), recurrenceId]]);
    expect(core.callsOf('editOccurrence')).toEqual([
      [eventId(3), recurrenceId, 'thisAndFollowing', true, draft],
    ]);
    expect(dialogs.scopePrompt).toBeUndefined();
    expect(dialogs.open).toEqual({
      kind: 'details',
      id: eventId(recurring.eventId),
      recurrenceId,
    });
  });

  test('Cancel leaves the editor open and saves nothing', async () => {
    const { core, dialogs } = await editingOccurrence();
    const editor = dialogs.open;

    const saved = dialogs.save(draftOf(dialogs));
    await settled();
    dialogs.chooseScope(null);
    await saved;

    expect(core.callsOf('editOccurrence')).toEqual([]);
    expect(dialogs.scopePrompt).toBeUndefined();
    expect(dialogs.open).toEqual(editor);
  });

  test('an edit of a single Event asks nothing', async () => {
    const { core, dialogs } = await started();
    dialogs.openDetails(eventId(3));
    await settled();
    dialogs.startEditing();
    const draft = draftOf(dialogs);

    await dialogs.save(draft);

    expect(core.callsOf('exceptionCounts')).toEqual([]);
    expect(core.callsOf('editEvent')).toEqual([[eventId(3), draft]]);
    expect(dialogs.open?.kind).toBe('details');
  });
});

describe('the Scope Prompt of a changed repeat', () => {
  test('asks about the repeat, and saves for the scope chosen', async () => {
    const { core, dialogs } = await editingOccurrence();
    const draft = { ...draftOf(dialogs), repeat: { ...weekly, interval: 3 } };

    const saved = dialogs.save(draft);
    await settled();

    expect(dialogs.scopePrompt?.change).toBe('editRepeat');

    dialogs.chooseScope({ scope: 'all', exceptionsToo: false });
    await saved;

    expect(core.callsOf('editOccurrence')).toEqual([
      [eventId(3), recurrenceId, 'all', false, draft],
    ]);
  });

  test('Cancel leaves the editor open and saves nothing', async () => {
    const { core, dialogs } = await editingOccurrence();

    const saved = dialogs.save({ ...draftOf(dialogs), repeat: null });
    await settled();

    expect(dialogs.scopePrompt?.change).toBe('editRepeat');

    dialogs.chooseScope(null);
    await saved;

    expect(core.callsOf('editOccurrence')).toEqual([]);
    expect(dialogs.open?.kind).toBe('editing');
  });
});

describe('the Scope Prompt of a drop', () => {
  test('asks which Occurrences move, and moves those chosen', async () => {
    const { core, dialogs } = await started();

    const moved = dialogs.reschedule(eventId(3), recurrenceId, when);
    await settled();

    expect(dialogs.scopePrompt?.change).toBe('move');
    expect(dialogs.dropHeld).toBe(true);

    dialogs.chooseScope({ scope: 'this', exceptionsToo: false });
    await moved;

    expect(core.callsOf('rescheduleOccurrence')).toEqual([
      [eventId(3), recurrenceId, 'this', false, when],
    ]);
    expect(dialogs.dropHeld).toBe(false);
  });

  test('Cancel rejects, so that the grid puts the Occurrence back, and moves nothing', async () => {
    const { core, dialogs } = await started();

    const moved = dialogs.reschedule(eventId(3), recurrenceId, when);
    await settled();
    dialogs.chooseScope(null);

    await expect(moved).rejects.toThrow();
    expect(core.callsOf('rescheduleOccurrence')).toEqual([]);
    expect(dialogs.scopePrompt).toBeUndefined();
  });

  test('holds a "Date changed" from the start until it is answered', async () => {
    const { core, model, dialogs } = await started();
    const counts = core.pending('exceptionCounts');

    const moved = dialogs.reschedule(eventId(3), recurrenceId, when);
    dateChanged(core, '2026-10-06');
    counts.resolve();
    await settled();

    expect(model.navigation?.date.toString()).toBe('2026-10-04');

    dialogs.chooseScope(null);
    await expect(moved).rejects.toThrow();

    expect(model.navigation?.date.toString()).toBe('2026-10-06');
  });

  test('a single Event moves without asking', async () => {
    const { core, dialogs } = await started();

    await dialogs.reschedule(eventId(3), null, when);

    expect(core.callsOf('exceptionCounts')).toEqual([]);
    expect(core.callsOf('rescheduleEvent')).toEqual([[eventId(3), when]]);
  });

  test('a failed move says why and rejects, until a move succeeds', async () => {
    const { core, dialogs } = await started();
    core.answer('rescheduleEvent', failure({ code: 'localStoreFailed' }));

    await expect(dialogs.reschedule(eventId(3), null, when)).rejects.toBeDefined();

    expect(dialogs.rescheduleFailure).toBeTruthy();

    core.answer('rescheduleEvent', changedEvent);
    await dialogs.reschedule(eventId(3), null, when);

    expect(dialogs.rescheduleFailure).toBeUndefined();
  });
});

describe('one dialog at a time', () => {
  test('opening one closes the one open, and the Window Model stops reading closed details', async () => {
    const { model, dialogs } = await started();
    dialogs.openDetails(eventId(3));
    await settled();

    expect(model.opened).toBeDefined();

    dialogs.openSettings();

    expect(dialogs.open).toEqual({ kind: 'settings' });
    expect(model.opened).toBeUndefined();
    expect(dialogs.selectedEventId).toBeUndefined();
  });

  test('a file picked while something else opened is let go', async () => {
    const { core, dialogs } = await started();
    const picked = core.pending('pickImportFile');

    const importing = dialogs.startImport();
    await settled();
    dialogs.openAbout();
    picked.resolve(file);
    await importing;

    expect(dialogs.open).toEqual({ kind: 'about' });
    expect(core.callsOf('discardImport')).toEqual([[]]);
  });

  test('so is one picked while a drop’s Scope Prompt opened', async () => {
    const { core, dialogs } = await started();
    const picked = core.pending('pickImportFile');

    const importing = dialogs.startImport();
    await settled();
    void dialogs.reschedule(eventId(3), recurrenceId, when).catch(() => {});
    await settled();
    picked.resolve(file);
    await importing;

    expect(dialogs.open).toBeUndefined();
    expect(core.callsOf('discardImport')).toEqual([[]]);

    dialogs.chooseScope(null);
  });

  test('details close once their Event is gone', async () => {
    const { core, dialogs } = await started();
    dialogs.openDetails(eventId(3));
    await settled();

    core.answer('event', failure({ code: 'localStoreFailed' }));
    core.emit(samples.Signal.find((found) => found.kind === 'occurrencesChanged') as Signal);
    await settled();

    expect(dialogs.open).toBeUndefined();
  });
});

describe('writes', () => {
  test('a new Event goes into the first shown writable Calendar', async () => {
    const { core, dialogs } = await started();
    dialogs.startCreating(when);
    const draft = draftOf(dialogs);

    expect(draft.calendarId).toBe(calendarId(2));

    await dialogs.save(draft);

    expect(core.callsOf('createEvent')).toEqual([[draft]]);
    expect(dialogs.open).toBeUndefined();
  });

  test('without a writable Calendar, a notice says where to create one', async () => {
    const core = new FakeCore();
    core.answer('listCalendars', []);
    const { dialogs } = await started(core);

    dialogs.startCreating(when);

    expect(dialogs.open?.kind).toBe('notice');
  });

  test('a failed delete says why and leaves the details open', async () => {
    const { core, dialogs } = await started();
    core.answer('deleteOccurrence', failure({ code: 'localStoreFailed' }));
    dialogs.openDetails(eventId(3), recurrenceId);
    await settled();

    await dialogs.removeOccurrence();

    expect(core.callsOf('deleteOccurrence')).toEqual([[eventId(3), recurrenceId]]);
    expect(dialogs.failure).toBeTruthy();
    expect(dialogs.open?.kind).toBe('details');
  });

  test('a refused drag says the Calendar is read-only in the toast', async () => {
    const core = new FakeCore();
    const calendar = first<Calendar>(samples.Calendar);
    core.answer('listCalendars', [{ ...calendar, readOnly: true }]);
    const { dialogs } = await started(core);

    dialogs.refuseDrag(recurring.eventId, recurrenceId);

    expect(dialogs.toast?.message).toContain(`its calendar “⁨${calendar.name}⁩” is read-only`);
    dialogs.dismissToast();
    expect(dialogs.toast).toBeUndefined();
  });

  test('a refused drag of an Event with guests in a writable Calendar says so', async () => {
    const { dialogs } = await started();
    expect(recurring.hasGuests).toBe(true);

    dialogs.refuseDrag(recurring.eventId, recurrenceId);

    expect(dialogs.toast?.message).toContain('has guests');
  });
});

describe('the import flow', () => {
  /** Window Dialogs with the import dialog open for `file`. */
  async function importing() {
    const opened = await started();
    opened.core.answer('pickImportFile', file);
    await opened.dialogs.startImport();
    opened.core.calls.splice(0);
    return opened;
  }

  test('the file picked opens the import dialog, into the first shown writable Calendar', async () => {
    const { core, dialogs } = await started();
    core.answer('pickImportFile', file);

    await dialogs.startImport();

    expect(dialogs.open).toEqual({ kind: 'importing', file, calendarId: calendarId(2) });
    expect(dialogs.isOpen).toBe(true);
  });

  test('no file picked opens nothing', async () => {
    const { core, dialogs } = await started();

    await dialogs.startImport();

    expect(dialogs.open).toBeUndefined();
    expect(core.callsOf('discardImport')).toEqual([]);
  });

  test('a file that can’t be picked says why', async () => {
    const { core, dialogs } = await started();
    core.answer('pickImportFile', failure({ code: 'localStoreFailed' }));

    await dialogs.startImport();

    expect(dialogs.open?.kind).toBe('notice');
  });

  test('without a writable Calendar, a notice says where to create one, and no file is asked for', async () => {
    const core = new FakeCore();
    core.answer('listCalendars', []);
    const { dialogs } = await started(core);

    await dialogs.startImport();

    expect(dialogs.open?.kind).toBe('notice');
    expect(core.callsOf('pickImportFile')).toEqual([]);
  });

  test('says "Working…" while it runs, then closes and offers Undo in the toast', async () => {
    const { core, dialogs } = await importing();
    const ran = core.pending('runImport');

    const run = dialogs.runImport(calendarId(2), 'update');
    await settled();

    expect(dialogs.working).toContain('a.ics');
    // Nothing closes the dialog while its import runs.
    dialogs.cancelImport();
    expect(dialogs.open?.kind).toBe('importing');
    expect(core.callsOf('discardImport')).toEqual([]);

    ran.resolve();
    await run;

    expect(core.callsOf('runImport')).toEqual([[calendarId(2), 'update']]);
    expect(dialogs.working).toBeUndefined();
    expect(dialogs.open).toBeUndefined();
    expect(dialogs.toast?.message).toContain('a.ics');
    expect(dialogs.toast?.undo).toBeDefined();
  });

  test('an import that changed nothing offers no Undo', async () => {
    const { core, dialogs } = await importing();
    core.answer('runImport', { journalTransaction: null, imported: 0, updated: 0, skipped: 3 });

    await dialogs.runImport(calendarId(2), 'skip');

    expect(dialogs.toast).toBeDefined();
    expect(dialogs.toast?.undo).toBeUndefined();
  });

  test('a failed import says why in the dialog, which stays open', async () => {
    const { core, dialogs } = await importing();
    core.answer('runImport', failure({ code: 'localStoreFailed' }));

    await dialogs.runImport(calendarId(2), 'update');

    expect(dialogs.importFailure).toBeTruthy();
    expect(dialogs.working).toBeUndefined();
    expect(dialogs.open?.kind).toBe('importing');
    expect(dialogs.toast).toBeUndefined();
  });

  test('Cancel discards the import and closes the dialog', async () => {
    const { core, dialogs } = await importing();

    dialogs.cancelImport();

    expect(core.callsOf('discardImport')).toEqual([[]]);
    expect(dialogs.open).toBeUndefined();
  });

  test('Undo says "Working…", then that the import was undone', async () => {
    const { core, dialogs } = await importing();
    await dialogs.runImport(calendarId(2), 'update');
    const undone = core.pending('undoJournalTransaction');

    dialogs.toast?.undo?.();
    await settled();

    expect(dialogs.working).toContain('a.ics');
    expect(dialogs.toast).toBeUndefined();

    undone.resolve();
    await settled();

    expect(core.callsOf('undoJournalTransaction')).toEqual([[4]]);
    expect(dialogs.working).toBeUndefined();
    expect(dialogs.toast?.message).toContain('a.ics');
    expect(dialogs.toast?.undo).toBeUndefined();
  });

  test('a failed Undo says why and can be retried', async () => {
    const { core, dialogs } = await importing();
    await dialogs.runImport(calendarId(2), 'update');
    core.answer('undoJournalTransaction', failure({ code: 'localStoreFailed' }));

    dialogs.toast?.undo?.();
    await settled();

    expect(dialogs.working).toBeUndefined();
    const retry = dialogs.toast?.undo;
    expect(retry).toBeDefined();

    core.answer('undoJournalTransaction', { ...changedEvent, value: null });
    retry?.();
    await settled();

    expect(core.callsOf('undoJournalTransaction')).toEqual([[4], [4]]);
    expect(dialogs.toast?.undo).toBeUndefined();
  });
});
