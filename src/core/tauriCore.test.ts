import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, expect, test } from 'vitest';
import { accountId, calendarId, eventId } from '../test/ids';
import { tauriCore } from './tauriCore';

afterEach(() => clearMocks());

test('today turns the shell’s ISO date into a PlainDate', async () => {
  mockIPC((command) => (command === 'today' ? '2026-10-04' : undefined));

  const today = await tauriCore.today();

  expect(today).toBeInstanceOf(Temporal.PlainDate);
  expect(today.equals(Temporal.PlainDate.from('2026-10-04'))).toBe(true);
});

test('listAccounts returns the shell’s Accounts', async () => {
  mockIPC((command) => (command === 'list_accounts' ? [{ id: 1, provider: 'local' }] : undefined));

  expect(await tauriCore.listAccounts()).toEqual([{ id: 1, provider: 'local' }]);
});

test('subscribe receives the Signals the shell forwards until unsubscribed', async () => {
  mockIPC(() => undefined, { shouldMockEvents: true });
  const received: unknown[] = [];

  const unsubscribe = await tauriCore.subscribe((signal) => received.push(signal));
  await emit('freecal://signal', { kind: 'first' });
  unsubscribe();
  await emit('freecal://signal', { kind: 'second' });
  await emit('some-other-event', { kind: 'other' });

  expect(received).toEqual([{ kind: 'first' }]);
});

test('the Calendar commands pass their arguments under the names the shell expects', async () => {
  const invoked: [string, unknown][] = [];
  mockIPC((command, args) => {
    invoked.push([command, args]);
    return command === 'list_calendars' ? [] : undefined;
  });

  await tauriCore.listCalendars();
  await tauriCore.createCalendar(accountId(1), 'Home', '#3366cc');
  await tauriCore.renameCalendar(calendarId(2), 'Family');
  await tauriCore.recolourCalendar(calendarId(2), '#ff8800');
  await tauriCore.setCalendarShown(calendarId(2), false);
  await tauriCore.showOnlyCalendar(calendarId(2));
  await tauriCore.deleteCalendar(calendarId(2));

  expect(invoked).toEqual([
    ['list_calendars', {}],
    ['create_calendar', { accountId: 1, name: 'Home', colour: '#3366cc' }],
    ['rename_calendar', { id: 2, name: 'Family' }],
    ['recolour_calendar', { id: 2, colour: '#ff8800' }],
    ['set_calendar_shown', { id: 2, shown: false }],
    ['show_only_calendar', { id: 2 }],
    ['delete_calendar', { id: 2 }],
  ]);
});

test('the Event commands pass their arguments under the names the shell expects', async () => {
  const invoked: [string, unknown][] = [];
  mockIPC((command, args) => {
    invoked.push([command, args]);
    return command === 'list_occurrences' ? [] : undefined;
  });
  const draft = {
    calendarId: calendarId(2),
    title: 'Dentist',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
    location: '',
    description: '',
  } as const;

  await tauriCore.listOccurrences(
    Temporal.PlainDate.from('2026-09-28'),
    Temporal.PlainDate.from('2026-11-09'),
  );
  await tauriCore.event(eventId(7));
  await tauriCore.createEvent(draft);
  await tauriCore.editEvent(eventId(7), draft);
  await tauriCore.deleteEvent(eventId(7));
  await tauriCore.openLink('https://example.com');

  expect(invoked).toEqual([
    ['list_occurrences', { from: '2026-09-28', to: '2026-11-09' }],
    ['event', { id: 7 }],
    ['create_event', { draft }],
    ['edit_event', { id: 7, draft }],
    ['delete_event', { id: 7 }],
    ['open_link', { url: 'https://example.com' }],
  ]);
});

test('the app commands pass their arguments under the names the shell expects', async () => {
  const invoked: [string, unknown][] = [];
  mockIPC((command, args) => {
    invoked.push([command, args]);
    return command === 'version' ? '0.1.0' : { appearance: 'dark' };
  });

  await tauriCore.settings();
  await tauriCore.updateSettings({ appearance: 'dark' });
  expect(await tauriCore.version()).toBe('0.1.0');
  await tauriCore.showWindow();
  await tauriCore.quit();

  expect(invoked).toEqual([
    ['settings', {}],
    ['update_settings', { update: { appearance: 'dark' } }],
    ['version', {}],
    ['show_window', {}],
    ['quit', {}],
  ]);
});

test('the sidebar commands pass their arguments under the names the shell expects', async () => {
  const invoked: [string, unknown][] = [];
  mockIPC((command, args) => {
    invoked.push([command, args]);
    return command === 'sidebar' ? { shown: true, width: null } : undefined;
  });

  await tauriCore.sidebar();
  await tauriCore.setSidebar({ shown: false, width: 18.5 });

  expect(invoked).toEqual([
    ['sidebar', {}],
    ['set_sidebar', { sidebar: { shown: false, width: 18.5 } }],
  ]);
});
