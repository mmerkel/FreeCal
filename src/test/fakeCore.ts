import type { CoreApi } from '../core/CoreApi';
import { samples } from '../core/samples.generated';
import type { Changed, Event, ErrorCode, Occurrence, Signal, Unbranded } from '../core/types';

type Method = keyof CoreApi;
type Args<M extends Method> = Parameters<CoreApi[M]>;
type Answer<M extends Method> = Awaited<ReturnType<CoreApi[M]>>;

/** One recorded call of the core interface: its method and its arguments. */
export type Call = { [M in Method]: { method: M; args: Args<M> } }[Method];

/** An error code a scripted answer rejects with, as the core's does when a method fails. */
export class Failure {
  constructor(readonly error: ErrorCode) {}
}

/** Scripts an answer that rejects with `error`. */
export const failure = (error: ErrorCode) => new Failure(error);

/** An answer the test gives later, once it has checked what the window does meanwhile. */
export interface PendingAnswer<M extends Method> {
  /** Answers with `value`, or with the default answer when given none. */
  resolve(value?: Answer<M>): void;
  fail(error: ErrorCode): void;
}

/** How a call is answered: with a value, with its default answer, or by rejecting. */
type Outcome<M extends Method> =
  { kind: 'value'; value: Answer<M> } | { kind: 'default' } | { kind: 'failure'; error: ErrorCode };

/**
 * A copy of `value` as it is now, as the real core gets or gives one in a
 * message: its arrays and plain objects copied all the way down, such as the
 * window's reactive state, and anything else, such as a `Temporal` date or a
 * function, as it is.
 */
const copyAsSent = <T>(value: T): T => {
  if (Array.isArray(value)) return value.map(copyAsSent) as T;
  if (value === null || typeof value !== 'object') return value;
  const prototype: unknown = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [key, copyAsSent(item)]),
  ) as T;
};

/** A copy of a sample of the core interface's types, with its IDs branded as the core sends them. */
const branded = <T>(sample: Unbranded<NoInfer<T>>): T => copyAsSent(sample) as unknown as T;
/** The first sample of a type. */
const first = <T>(samplesOfType: readonly Unbranded<NoInfer<T>>[]): T =>
  branded<T>(samplesOfType[0]);
/** Every sample of a type. */
const every = <T>(samplesOfType: readonly Unbranded<NoInfer<T>>[]): T[] =>
  samplesOfType.map((sample) => branded<T>(sample));

/** A `Changed` with `value` and the Journal Transaction of the first sample `Changed<Event>`. */
const changed = <T>(value: T): Changed<T> => ({
  journalTransaction: first<Changed<Event>>(samples.Changed).journalTransaction,
  value,
});

/**
 * The fake core interface of the frontend tests (spec: "Testing Decisions",
 * Seam 2), which reimplements nothing of the core.
 * It records every call with its arguments and answers it as the test
 * scripted with `answer` or `pending`; a call nobody scripted gets its
 * default answer, built from the generated samples. It sends no Signal of its
 * own: tests `emit` the Signals the core would send.
 */
export class FakeCore {
  /**
   * Every call so far, in order, those still waiting for their answer too,
   * each with a copy of its arguments as they were at the call.
   */
  readonly calls: Call[] = [];
  private readonly subscribers = new Set<(signal: Signal) => void>();
  private readonly scripted = new Map<Method, Outcome<Method>>();
  private readonly pendingAnswers = new Map<Method, Promise<Outcome<Method>>[]>();

  /**
   * The answer to a call nobody scripted: the first sample of its type, every
   * sample for a list, and nothing where the answer can be nothing. A method
   * added to `CoreApi` needs one here, which makes it a method of this fake.
   */
  private readonly defaults: { [M in Method]: (...args: Args<M>) => Answer<M> } = {
    startupFailure: () => null,
    subscribe: (onSignal) => {
      this.subscribers.add(onSignal);
      return () => void this.subscribers.delete(onSignal);
    },
    today: () => {
      const dateChanged = samples.Signal.find((signal) => signal.kind === 'dateChanged');
      if (!dateChanged) throw new Error('The samples have no "Date changed" Signal');
      return Temporal.PlainDate.from(dateChanged.date);
    },
    lastUsedView: () => first(samples.View),
    setLastUsedView: () => undefined,
    sidebar: () => first(samples.SidebarState),
    setSidebar: () => undefined,
    settings: () => first(samples.Settings),
    updateSettings: () => first(samples.Settings),
    listAccounts: () => every(samples.Account),
    addCalDavAccount: () => first(samples.AddedAccount),
    availableCalendars: () => every(samples.AvailableCalendar),
    addCalendars: () => every(samples.Calendar),
    removeAccount: () => undefined,
    listCalendars: () => every(samples.Calendar),
    syncStatuses: () => first(samples.SyncStatuses),
    syncNow: () => undefined,
    createCalendar: () => first(samples.Calendar),
    renameCalendar: () => undefined,
    recolourCalendar: () => undefined,
    deleteCalendar: () => undefined,
    removeCalendar: () => undefined,
    setCalendarShown: () => undefined,
    showOnlyCalendar: () => undefined,
    listOccurrences: () => every(samples.Occurrence),
    event: () => first(samples.Event),
    createEvent: () => first(samples.Changed),
    editEvent: () => first(samples.Changed),
    rescheduleEvent: () => first(samples.Changed),
    editOccurrence: () => changed(first<Occurrence>(samples.Occurrence)),
    rescheduleOccurrence: () => changed(first<Occurrence>(samples.Occurrence)),
    exceptionCounts: () => first(samples.ExceptionCounts),
    deleteEvent: () => changed(null),
    deleteOccurrence: () => changed(null),
    pickImportFile: () => null,
    previewImport: () => first(samples.ImportSummary),
    runImport: () => first(samples.ImportOutcome),
    discardImport: () => undefined,
    undoJournalTransaction: () => changed(null),
    openLink: () => undefined,
    // A version is no type of the interface, so it has no sample.
    version: () => '0.0.0',
    showWindow: () => undefined,
    quit: () => undefined,
  };

  constructor() {
    for (const method of Object.keys(this.defaults) as Method[]) {
      Object.assign(this, { [method]: (...args: Args<Method>) => this.call(method, args) });
    }
  }

  /**
   * Answers every later call of `method` with a copy of `value`, or rejects
   * them with a `failure`. An answer to `subscribe` replaces its default, which
   * is what lets `emit` reach the subscriber.
   */
  answer<M extends Method>(method: M, value: Answer<M> | Failure) {
    this.scripted.set(
      method,
      value instanceof Failure ? { kind: 'failure', error: value.error } : { kind: 'value', value },
    );
  }

  /**
   * Holds the answer to the next call of `method` until the test gives it.
   * Pending answers come before a scripted one, in the order they were made.
   */
  pending<M extends Method>(method: M): PendingAnswer<M> {
    let settle!: (outcome: Outcome<M>) => void;
    const outcome = new Promise<Outcome<M>>((resolve) => (settle = resolve));
    const queue = this.pendingAnswers.get(method) ?? [];
    queue.push(outcome as Promise<Outcome<Method>>);
    this.pendingAnswers.set(method, queue);
    return {
      resolve: (...value: [Answer<M>?]) =>
        settle(
          value.length ? { kind: 'value', value: value[0] as Answer<M> } : { kind: 'default' },
        ),
      fail: (error) => settle({ kind: 'failure', error }),
    };
  }

  /** The arguments of every call of `method` so far, in order. */
  callsOf<M extends Method>(method: M): Args<M>[] {
    return this.calls.filter((call) => call.method === method).map((call) => call.args as Args<M>);
  }

  /** Sends `signal` to every subscriber, as the core would. */
  emit(signal: Signal) {
    for (const onSignal of [...this.subscribers]) onSignal(signal);
  }

  private async call<M extends Method>(method: M, args: Args<M>): Promise<Answer<M>> {
    this.calls.push({ method, args: copyAsSent(args) } as Call);
    const outcome = (await (this.pendingAnswers.get(method)?.shift() ??
      this.scripted.get(method) ?? { kind: 'default' })) as Outcome<M>;
    switch (outcome.kind) {
      case 'value':
        return copyAsSent(outcome.value);
      case 'failure':
        throw outcome.error;
      case 'default':
        return (this.defaults[method] as (...args: Args<M>) => Answer<M>)(...args);
    }
  }
}

// The methods of `CoreApi`, which the constructor adds from `defaults`.
export interface FakeCore extends CoreApi {}
