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
  listAccounts: () => invoke('list_accounts'),
};
