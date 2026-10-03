// Mirrors the types of the Rust core's application interface.

export type AccountId = number;

/** A kind of calendar service FreeCal can talk to. */
export type Provider = 'local';

export interface Account {
  id: AccountId;
  provider: Provider;
}

/**
 * A message from the core saying that something changed. Never call these
 * "events" or "notifications". The v1 Signals are added by the tickets that
 * first send them.
 */
export type Signal = never;
