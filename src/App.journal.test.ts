/**
 * The Change Journal list and an Event's History (ticket 19): what they
 * show, and the undos they run through the core.
 */
import { screen, waitFor, within } from '@testing-library/svelte';
import type userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../fixtures/hostile-event.json';
import type {
  DangerousUndo,
  ListedJournalEventModification,
  ListedJournalTransaction,
  When,
} from './core/types';
import { calendar, event, firstLaunch } from './test/answers';
import { combinedEventKey, journalTransactionId } from './test/ids';
import { failure, type FakeCore } from './test/fakeCore';
import { showApp } from './test/showApp';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

type User = ReturnType<typeof userEvent.setup>;

const monday: When = { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' };
const home = calendar({ name: 'Home' });
const work = calendar({ name: 'Work' });
const holidays = calendar({ name: 'Holidays', readOnly: true });

/** A Journal Transaction as the Change Journal list shows it, a user's move of Dentist unless `fields` say otherwise. */
function listed(
  id: number,
  fields: Partial<ListedJournalTransaction> = {},
): ListedJournalTransaction {
  return {
    id: journalTransactionId(id),
    origin: { kind: 'userAction', action: 'moved' },
    madeAt: 1791100700,
    undoes: null,
    events: 1,
    undoneEvents: 0,
    calendarIds: [home.id],
    titles: ['Dentist'],
    undonePerEvent: false,
    undoChangedNothing: false,
    ...fields,
  };
}

const move = listed(4);
const sync = listed(5, {
  origin: { kind: 'incomingSync' },
  madeAt: 1791100800,
  events: 3,
  undoneEvents: 1,
  calendarIds: [home.id, work.id],
  titles: ['Standup', 'Yoga'],
  undonePerEvent: true,
});

/**
 * A Journal Event Modification of Journal Transaction `of`, as listed, of the
 * Event with key `key`.
 */
function modification(
  of: ListedJournalTransaction,
  fields: Partial<ListedJournalEventModification> = {},
  key = combinedEventKey('event-30'),
): ListedJournalEventModification {
  return {
    journalTransaction: of.id,
    origin: of.origin,
    madeAt: of.madeAt,
    title: 'Standup',
    calendarId: home.id,
    kind: 'edited',
    undone: false,
    undo: of.undonePerEvent
      ? { kind: 'event', journalTransaction: of.id, combinedEventKey: key }
      : { kind: 'journalTransaction', id: of.id },
    ...fields,
  };
}

/** The window with Home, Work and the Read-only Holidays, and a Change Journal of `journal`. */
async function withJournal(journal: ListedJournalTransaction[] = [sync, move]) {
  const core = firstLaunch({ calendars: [home, work, holidays] });
  core.answer('changeJournal', journal);
  return showApp(core);
}

/** Opens the Change Journal from the main menu. */
async function openJournal(user: User) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Change Journal…' }));
  const dialog = screen.getByRole('dialog', { name: 'Change Journal' });
  await within(dialog).findByRole('list', { name: 'Changes, newest first' });
  return dialog;
}

const entry = (dialog: HTMLElement, name: string) => within(dialog).getByRole('listitem', { name });

test('the main menu opens the Change Journal, newest first, each with how much is undone', async () => {
  const { user } = await withJournal();

  const dialog = await openJournal(user);

  const list = within(dialog).getByRole('list', { name: 'Changes, newest first' });
  const entries = within(list)
    .getAllByRole('listitem')
    .filter((item) => item.parentElement === list);
  expect(entries.map((item) => item.getAttribute('aria-label'))).toEqual([
    'Changed on the server: 3 events',
    'Moved “⁨Dentist⁩”',
  ]);
  expect(entries[0]).toHaveTextContent('Home, Work');
  expect(entries[0]).toHaveTextContent('1 of 3 undone');
  expect(entries[0]).toHaveTextContent('“\u2068Standup\u2069”, “\u2068Yoga\u2069”…');
  expect(within(entries[0]).getByRole('button', { name: 'Undo all' })).toBeInTheDocument();
  expect(within(entries[1]).getByRole('button', { name: 'Undo' })).toBeInTheDocument();
});

test('a Journal Transaction wholly undone offers no Undo', async () => {
  const { user } = await withJournal([listed(4, { undoneEvents: 1 })]);

  const dialog = await openJournal(user);

  const moved = entry(dialog, 'Moved “⁨Dentist⁩”');
  expect(moved).toHaveTextContent('Undone');
  expect(within(moved).queryByRole('button', { name: 'Undo' })).toBeNull();
});

test('a user action expands into its Events, undone only whole', async () => {
  const { core, user } = await withJournal();
  core.answer('journalTransactionEvents', [modification(move, { title: 'Dentist' })]);
  const dialog = await openJournal(user);
  const moved = entry(dialog, 'Moved “⁨Dentist⁩”');

  await user.click(within(moved).getByRole('button', { name: 'Events' }));

  const events = await within(moved).findByRole('list', {
    name: 'Events of Moved “⁨Dentist⁩”',
  });
  expect(core.callsOf('journalTransactionEvents')).toEqual([[move.id]]);
  expect(events).toHaveTextContent('Changed “\u2068Dentist\u2069”');
  expect(within(events).queryByRole('button', { name: 'Undo' })).toBeNull();
});

test('a sync’s Event is undone alone, and the list is read again', async () => {
  const { core, user } = await withJournal();
  const standup = modification(sync, {}, combinedEventKey('event-31'));
  core.answer('journalTransactionEvents', [
    standup,
    modification(sync, { title: 'Yoga', undone: true, undo: null }),
  ]);
  const dialog = await openJournal(user);
  const synced = entry(dialog, 'Changed on the server: 3 events');
  await user.click(within(synced).getByRole('button', { name: 'Events' }));
  const events = await within(synced).findByRole('list');
  expect(events).toHaveTextContent('Undone');

  await user.click(within(events).getByRole('button', { name: 'Undo' }));

  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent('Undone: the changes to “⁨Standup⁩”.'),
  );
  expect(core.callsOf('undo')).toEqual([[standup.undo, null]]);
  await waitFor(() => expect(core.callsOf('changeJournal')).toHaveLength(2));
  expect(core.callsOf('journalTransactionEvents')).toHaveLength(2);
});

test('“Undo all” undoes the rest of a sync under “Working…”', async () => {
  const { core, user } = await withJournal();
  const dialog = await openJournal(user);
  const undone = core.pending('undo');

  await user.click(
    within(entry(dialog, 'Changed on the server: 3 events')).getByRole('button', {
      name: 'Undo all',
    }),
  );

  expect(
    await screen.findByText('Undoing the changes on the server to 3 events…'),
  ).toBeInTheDocument();
  undone.resolve();
  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent(
      'Undone: the changes on the server to 3 events.',
    ),
  );
  expect(core.callsOf('undo')).toEqual([[{ kind: 'journalTransaction', id: sync.id }, null]]);
});

test('a Dangerous Undo from the list asks first', async () => {
  const { core, user } = await withJournal();
  const danger: DangerousUndo = {
    laterJournalTransactions: [],
    serverChanges: 1,
    exceedsMassChangeGuard: false,
    events: 1,
  };
  core.pending('undo').resolve({
    journalTransaction: null,
    value: { kind: 'dangerous', danger, skipped: [] },
  });
  const dialog = await openJournal(user);

  await user.click(
    within(entry(dialog, 'Moved “⁨Dentist⁩”')).getByRole('button', { name: 'Undo' }),
  );

  const question = await screen.findByRole('alertdialog', {
    name: 'Undo moving “⁨Dentist⁩”?',
  });
  await user.click(within(question).getByRole('button', { name: 'Undo' }));
  await waitFor(() =>
    expect(core.callsOf('undo')).toEqual([
      [{ kind: 'journalTransaction', id: move.id }, null],
      [{ kind: 'journalTransaction', id: move.id }, danger],
    ]),
  );
});

test('undoing a Calendar since a time undoes from then on in that Calendar', async () => {
  const { core, user } = await withJournal();
  const dialog = await openJournal(user);
  const calendars = within(dialog).getByRole('combobox', { name: 'Calendar' });
  expect(
    within(calendars)
      .getAllByRole('option')
      .map((option) => option.textContent),
  ).toEqual(['Home', 'Work']);

  await user.selectOptions(calendars, 'Work');
  const since = within(dialog).getByLabelText('Since');
  await user.clear(since);
  await user.type(since, '2026-10-05T09:30');
  await user.click(within(dialog).getByRole('button', { name: 'Undo since' }));

  const seconds =
    Temporal.PlainDateTime.from('2026-10-05T09:30').toZonedDateTime(Temporal.Now.timeZoneId())
      .epochMilliseconds / 1000;
  await waitFor(() =>
    expect(core.callsOf('undo')).toEqual([
      [{ kind: 'calendarSince', calendarId: work.id, since: seconds }, null],
    ]),
  );
  await waitFor(() =>
    expect(screen.getByRole('status')).toHaveTextContent('Undone: the changes to “⁨Work⁩” since'),
  );
});

test('a Change Journal that can’t be read says why', async () => {
  const core = firstLaunch({ calendars: [home] });
  core.answer('changeJournal', failure({ code: 'localStoreFailed' }));
  const { user } = await showApp(core);

  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Change Journal…' }));

  expect(await screen.findByRole('alert')).toHaveTextContent(/^Couldn’t read the Change Journal/);
});

test('the list names a hostile Event as text', async () => {
  const { user } = await withJournal([listed(4, { titles: [hostile.title] })]);

  const dialog = await openJournal(user);

  expect(dialog).toHaveTextContent('<img src=x onerror=alert(1)>');
  expect(document.querySelector('img, script')).toBeNull();
});

/** Opens the details of `opened`, Dentist in Home, with its History `history`. */
async function openHistory(core: FakeCore, user: User, history: ListedJournalEventModification[]) {
  const dentist = event({
    combinedEventKey: combinedEventKey('event-30'),
    calendarId: home.id,
    title: 'Dentist',
    when: monday,
  });
  core.answer('event', dentist);
  core.answer('eventHistory', history);
  const grid = screen.getByRole('list', { name: 'Grid' });
  await user.click(await within(grid).findByRole('button', { name: 'Dentist' }));
  const details = await screen.findByRole('dialog', { name: 'Dentist' });
  await user.click(within(details).getByRole('button', { name: 'History' }));
  return within(details).findByRole('list', { name: 'History of “⁨Dentist⁩”' });
}

test('the History in an Event’s details lists its changes, each with Undo', async () => {
  const core = firstLaunch({
    calendars: [home],
    events: [
      event({
        combinedEventKey: combinedEventKey('event-30'),
        calendarId: home.id,
        title: 'Dentist',
        when: monday,
      }),
    ],
  });
  const { user } = await showApp(core);
  const changes = [
    modification(sync, { title: 'Dentist' }),
    modification(move, { title: 'Dentist', kind: 'created', undone: true, undo: null }),
  ];

  const history = await openHistory(core, user, changes);

  expect(core.callsOf('eventHistory')).toEqual([[combinedEventKey('event-30')]]);
  const rows = within(history).getAllByRole('listitem');
  expect(rows[0]).toHaveTextContent('Changed “\u2068Dentist\u2069”');
  expect(rows[0]).toHaveTextContent('on the server');
  expect(rows[1]).toHaveTextContent('Created “\u2068Dentist\u2069”');
  expect(rows[1]).toHaveTextContent('by you');
  expect(rows[1]).toHaveTextContent('Undone');
  expect(within(rows[1]).queryByRole('button', { name: 'Undo' })).toBeNull();

  await user.click(within(rows[0]).getByRole('button', { name: 'Undo' }));

  await waitFor(() => expect(core.callsOf('undo')).toEqual([[changes[0].undo, null]]));
  await waitFor(() => expect(core.callsOf('eventHistory')).toHaveLength(2));
  expect(screen.getByRole('status')).toHaveTextContent('Undone: the changes to “⁨Dentist⁩”.');
});

test('a user action in the History undoes its whole Journal Transaction', async () => {
  const core = firstLaunch({
    calendars: [home],
    events: [
      event({
        combinedEventKey: combinedEventKey('event-30'),
        calendarId: home.id,
        title: 'Dentist',
        when: monday,
      }),
    ],
  });
  const { user } = await showApp(core);

  const history = await openHistory(core, user, [modification(move, { title: 'Dentist' })]);
  await user.click(within(history).getByRole('button', { name: 'Undo' }));

  await waitFor(() =>
    expect(core.callsOf('undo')).toEqual([[{ kind: 'journalTransaction', id: move.id }, null]]),
  );
  expect(screen.getByRole('status')).toHaveTextContent('Undone: moving “⁨Dentist⁩”.');
});
