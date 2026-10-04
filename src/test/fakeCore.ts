import type { CoreApi } from '../core/CoreApi';
import type { Account, Signal } from '../core/types';

/** A fake core interface for component tests. It can also emit Signals. */
export class FakeCore implements CoreApi {
  accounts: Account[] = [{ id: 1, provider: 'local' }];
  currentDate = Temporal.PlainDate.from('2026-10-03');
  private subscribers = new Set<(signal: Signal) => void>();

  async subscribe(onSignal: (signal: Signal) => void) {
    this.subscribers.add(onSignal);
    return () => {
      this.subscribers.delete(onSignal);
    };
  }

  emit(signal: Signal) {
    for (const subscriber of this.subscribers) subscriber(signal);
  }

  async today() {
    return this.currentDate;
  }

  async listAccounts() {
    return this.accounts;
  }
}
