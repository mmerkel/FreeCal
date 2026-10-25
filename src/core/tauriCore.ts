import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { CoreApi } from './CoreApi';
import type { Signal } from './types';

/** The Tauri event on which the shell forwards the core's Signals. */
const SIGNAL_EVENT = 'freecal://signal';

/** The core application interface, reached through the Tauri shell's commands. */
export const tauriCore: CoreApi = {
  startupFailure: () => invoke('startup_failure'),
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
  addCalDavAccount: (url, username, password) =>
    invoke('add_caldav_account', { login: { url, username, password } }),
  availableCalendars: (accountId) => invoke('available_calendars', { accountId }),
  addCalendars: (accountId, urls) => invoke('add_calendars', { accountId, urls }),
  removeAccount: (id) => invoke('remove_account', { id }),
  listCalendars: () => invoke('list_calendars'),
  createCalendar: (accountId, name, colour) =>
    invoke('create_calendar', { accountId, name, colour }),
  renameCalendar: (id, name) => invoke('rename_calendar', { id, name }),
  recolourCalendar: (id, colour) => invoke('recolour_calendar', { id, colour }),
  deleteCalendar: (id) => invoke('delete_calendar', { id }),
  removeCalendar: (id) => invoke('remove_calendar', { id }),
  setCalendarShown: (id, shown) => invoke('set_calendar_shown', { id, shown }),
  showOnlyCalendar: (id) => invoke('show_only_calendar', { id }),
  listOccurrences: (from, to) =>
    invoke('list_occurrences', { from: from.toString(), to: to.toString() }),
  event: (id) => invoke('event', { id }),
  createEvent: (draft) => invoke('create_event', { draft }),
  editEvent: (id, draft) => invoke('edit_event', { id, draft }),
  rescheduleEvent: (id, when) => invoke('reschedule_event', { id, when }),
  editOccurrence: (id, recurrenceId, scope, exceptionsToo, draft) =>
    invoke('edit_occurrence', { id, recurrenceId, scope, exceptionsToo, draft }),
  rescheduleOccurrence: (id, recurrenceId, scope, exceptionsToo, when) =>
    invoke('reschedule_occurrence', { id, recurrenceId, scope, exceptionsToo, when }),
  exceptionCounts: (id, recurrenceId) => invoke('exception_counts', { id, recurrenceId }),
  deleteEvent: (id) => invoke('delete_event', { id }),
  deleteOccurrence: (id, recurrenceId) => invoke('delete_occurrence', { id, recurrenceId }),
  pickImportFile: (title, filter) => invoke('pick_import_file', { title, filter }),
  previewImport: (calendarId) => invoke('preview_import', { calendarId }),
  runImport: (calendarId, existing) => invoke('run_import', { calendarId, existing }),
  discardImport: () => invoke('discard_import'),
  undoUnit: (id) => invoke('undo_unit', { id }),
  openLink: (url) => invoke('open_link', { url }),
  version: () => invoke('version'),
  showWindow: () => invoke('show_window'),
  quit: () => invoke('quit'),
};
