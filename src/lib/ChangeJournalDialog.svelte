<!--
  The Change Journal list (spec: "Ways to undo"): every Journal Transaction
  as one timeline, newest first, with when it was made, its Calendars and
  how much of it is undone. Each expands into its Events and has "Undo all"
  for what isn't undone yet. Above the list, the user undoes everything in a
  Calendar since a given time. After each undo the list is read again.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CoreApi } from '../core/CoreApi';
  import type {
    Calendar,
    CalendarId,
    JournalTransactionId,
    ListedJournalEventModification,
    ListedJournalTransaction,
  } from '../core/types';
  import { errorText, t, tCount } from '../i18n';
  import Dialog from './Dialog.svelte';
  import JournalEventList from './JournalEventList.svelte';
  import { calendarSinceUndoing, journalTimeText, transactionUndoing, type Undoing } from './undo';

  interface Props {
    core: CoreApi;
    /** Every Calendar, to name those of the Journal Transactions. */
    calendars: Calendar[];
    /** The Calendars an undo since a given time can change: those that aren't Read-only. */
    writableCalendars: Calendar[];
    /** Runs an undo, asking first if it is a Dangerous Undo; resolves once it is done. */
    onundo: (undoing: Undoing) => Promise<void>;
    onclose: () => void;
  }

  let { core, calendars, writableCalendars, onundo, onclose }: Props = $props();

  let listed = $state<ListedJournalTransaction[]>();
  let readFailure = $state<string>();
  /** The Events of each expanded Journal Transaction, once read, or why that failed. */
  let expanded = $state(
    new Map<JournalTransactionId, ListedJournalEventModification[] | { failure: string } | null>(),
  );

  let sinceCalendarId = $state<CalendarId | undefined>(untrack(() => writableCalendars[0]?.id));
  /** When "since" starts, as the `datetime-local` field writes it: an hour ago at first. */
  let sinceText = $state(
    Temporal.Now.plainDateTimeISO().subtract({ hours: 1 }).toString({ smallestUnit: 'minute' }),
  );
  /** "Since", in Unix seconds, if the field holds a time. */
  const since = $derived.by(() => {
    try {
      const zoned = Temporal.PlainDateTime.from(sinceText).toZonedDateTime(
        Temporal.Now.timeZoneId(),
      );
      return Math.floor(zoned.epochMilliseconds / 1000);
    } catch {
      return undefined;
    }
  });
  const sinceCalendar = $derived(writableCalendars.find(({ id }) => id === sinceCalendarId));

  untrack(() => void read());

  /** Reads the list, and the Events of each expanded Journal Transaction, again. */
  async function read() {
    try {
      listed = await core.changeJournal();
      readFailure = undefined;
    } catch (error) {
      readFailure = t('journal.readFailed', { error: errorText(error) });
      listed ??= [];
    }
    await Promise.all([...expanded.keys()].map(readEvents));
  }

  async function readEvents(id: JournalTransactionId) {
    let events: ListedJournalEventModification[] | { failure: string };
    try {
      events = await core.journalTransactionEvents(id);
    } catch (error) {
      events = { failure: t('journal.readFailed', { error: errorText(error) }) };
    }
    // Collapsed meanwhile, it stays collapsed.
    if (expanded.has(id)) expanded = new Map(expanded).set(id, events);
  }

  function toggle(id: JournalTransactionId) {
    const next = new Map(expanded);
    if (next.delete(id)) {
      expanded = next;
      return;
    }
    expanded = next.set(id, null);
    void readEvents(id);
  }

  async function undo(undoing: Undoing) {
    await onundo(undoing);
    await read();
  }

  const calendarNames = (ids: CalendarId[]) =>
    ids
      .map((id) => calendars.find((calendar) => calendar.id === id)?.name)
      .filter((name) => name !== undefined)
      .join(', ');

  /** The headline of a Journal Transaction: a user action by its title, the others by their number of Events. */
  function headline(transaction: ListedJournalTransaction): string {
    const { origin, events } = transaction;
    switch (origin.kind) {
      case 'userAction':
        return t(`journal.action.${origin.action}`, {
          title: transaction.titles[0] || t('event.untitled'),
        });
      case 'import':
        return tCount('journal.imported', events);
      case 'incomingSync':
        return tCount('journal.incomingSync', events);
      case 'undo':
        return tCount('journal.undo', events);
      case 'firstSync':
        return tCount('journal.firstSync', events);
    }
  }

  /** The first titles of a Journal Transaction's Events, with "…" if it changed more. */
  function titlesText({ titles, events }: ListedJournalTransaction): string {
    const quoted = titles
      .map((title) => t('journal.quotedTitle', { title: title || t('event.untitled') }))
      .join(', ');
    return events > titles.length ? `${quoted}…` : quoted;
  }

  /** How much of a Journal Transaction is undone, if any. */
  function undoneText({ events, undoneEvents }: ListedJournalTransaction): string | undefined {
    if (undoneEvents === 0) return undefined;
    if (undoneEvents >= events) return t('journal.undone');
    return t('journal.partlyUndone', { undone: String(undoneEvents), events: String(events) });
  }
</script>

<Dialog title={t('journal.title')} {onclose}>
  <div class="form">
    <form
      class="since"
      onsubmit={(submitted) => {
        submitted.preventDefault();
        if (sinceCalendar && since !== undefined)
          void undo(calendarSinceUndoing(sinceCalendar, since));
      }}
    >
      <fieldset class="choices">
        <legend class="field-label">{t('journal.since.legend')}</legend>
        <div class="since-fields">
          <label class="field">
            <span class="field-label">{t('journal.since.calendar')}</span>
            <select class="input" bind:value={sinceCalendarId}>
              {#each writableCalendars as calendar (calendar.id)}
                <option value={calendar.id}>{calendar.name}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            <span class="field-label">{t('journal.since.time')}</span>
            <input class="input" type="datetime-local" bind:value={sinceText} />
          </label>
          <button
            type="submit"
            class="button outline"
            disabled={!sinceCalendar || since === undefined}
          >
            {t('journal.since.undo')}
          </button>
        </div>
      </fieldset>
    </form>

    {#if readFailure}
      <p class="problem" role="alert">{readFailure}</p>
    {/if}
    {#if !listed}
      <p aria-busy="true">{t('journal.reading')}</p>
    {:else if listed.length === 0 && !readFailure}
      <p>{t('journal.empty')}</p>
    {:else}
      <ul class="transactions" aria-label={t('journal.list')}>
        {#each listed as transaction (transaction.id)}
          {@const title = headline(transaction)}
          {@const events = expanded.get(transaction.id)}
          {@const undone = undoneText(transaction)}
          <li class="transaction" aria-label={title}>
            <div class="summary">
              <div class="text">
                <span>{title}</span>
                <span class="note">
                  {journalTimeText(transaction.madeAt)}
                  {#if transaction.calendarIds.length > 0}
                    · <bdi>{calendarNames(transaction.calendarIds)}</bdi>
                  {/if}
                  {#if undone}
                    · {undone}
                  {/if}
                </span>
                {#if transaction.origin.kind !== 'userAction' && transaction.titles.length > 0}
                  <span class="note">{titlesText(transaction)}</span>
                {/if}
              </div>
              <div class="buttons">
                <button
                  type="button"
                  class="button outline"
                  aria-expanded={events !== undefined}
                  onclick={() => toggle(transaction.id)}
                >
                  {t('journal.events')}
                </button>
                {#if transaction.undoneEvents < transaction.events}
                  <button
                    type="button"
                    class="button outline"
                    onclick={() => void undo(transactionUndoing(transaction))}
                  >
                    {t(transaction.events > 1 ? 'journal.undoAll' : 'common.undo')}
                  </button>
                {/if}
              </div>
            </div>
            {#if events === null}
              <p class="note" aria-busy="true">{t('journal.reading')}</p>
            {:else if events && 'failure' in events}
              <p class="problem" role="alert">{events.failure}</p>
            {:else if events}
              <div class="events">
                <JournalEventList
                  modifications={events}
                  {calendars}
                  history={false}
                  undoEach={transaction.undonePerEvent}
                  label={t('journal.eventsOf', { title })}
                  onundo={(undoing) => void undo(undoing)}
                />
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#snippet actions()}
    <button type="button" class="button solid" onclick={onclose}>{t('common.close')}</button>
  {/snippet}
</Dialog>

<style>
  .since-fields {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: var(--space-6);
  }

  .transactions {
    display: grid;
    gap: var(--space-6);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .transaction {
    display: grid;
    gap: var(--space-4);
    padding-bottom: var(--space-6);
    border-bottom: 1px solid var(--border);
  }

  .summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-6);
  }

  .text {
    display: grid;
    gap: var(--space-1);
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .events {
    padding-inline-start: var(--space-6);
    border-inline-start: 2px solid var(--border);
  }

  .buttons {
    display: flex;
    flex-shrink: 0;
    gap: var(--space-4);
  }
</style>
