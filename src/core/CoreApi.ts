import type { Account, Signal } from './types';

/** The core application interface. The frontend talks only to this. */
export interface CoreApi {
  /** Starts receiving Signals. Subscribe before reading state, so that nothing falls in the gap. */
  subscribe(onSignal: (signal: Signal) => void): Promise<() => void>;
  /** The current local date. */
  today(): Promise<Temporal.PlainDate>;
  listAccounts(): Promise<Account[]>;
}
