import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { CoreApi } from './CoreApi';
import type { Signal } from './types';

/** The Tauri event on which the shell forwards the core's Signals. */
const SIGNAL_EVENT = 'freecal://signal';

/** The core application interface, reached through the Tauri shell's commands. */
export const tauriCore: CoreApi = {
  subscribe: (onSignal) => listen<Signal>(SIGNAL_EVENT, (message) => onSignal(message.payload)),
  // The shell sends dates as ISO strings (YYYY-MM-DD).
  today: async () => Temporal.PlainDate.from(await invoke<string>('today')),
  lastUsedView: () => invoke('last_used_view'),
  setLastUsedView: (view) => invoke('set_last_used_view', { view }),
  sidebar: () => invoke('sidebar'),
  setSidebar: (sidebar) => invoke('set_sidebar', { sidebar }),
  settings: () => invoke('settings'),
  updateSettings: (update) => invoke('update_settings', { update }),
  listAccounts: () => invoke('list_accounts'),
  listCalendars: () => invoke('list_calendars'),
  createCalendar: (accountId, name, colour) =>
    invoke('create_calendar', { accountId, name, colour }),
  renameCalendar: (id, name) => invoke('rename_calendar', { id, name }),
  recolourCalendar: (id, colour) => invoke('recolour_calendar', { id, colour }),
  deleteCalendar: (id) => invoke('delete_calendar', { id }),
  setCalendarShown: (id, shown) => invoke('set_calendar_shown', { id, shown }),
  showOnlyCalendar: (id) => invoke('show_only_calendar', { id }),
  listOccurrences: (from, to) =>
    invoke('list_occurrences', { from: from.toString(), to: to.toString() }),
  event: (id) => invoke('event', { id }),
  createEvent: (draft) => invoke('create_event', { draft }),
  editEvent: (id, draft) => invoke('edit_event', { id, draft }),
  deleteEvent: (id) => invoke('delete_event', { id }),
  openLink: (url) => invoke('open_link', { url }),
  version: () => invoke('version'),
  showWindow: () => invoke('show_window'),
  quit: () => invoke('quit'),
};
