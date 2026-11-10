<!--
  Creates or edits an Event in a modal dialog. The draft is the editor's own:
  it starts from `initial` and nothing from outside changes it afterwards,
  so a re-fetch while the editor is open never overwrites what the user typed.
  Closing it (✕, Cancel, Escape) asks "Discard changes?" once the draft
  differs from how it started.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { Calendar, EventDraft, Frequency, Repeat, RepeatEnd } from '../core/types';
  import { errorText, t, tCount } from '../i18n';
  import Dialog, { enterSubmits } from './Dialog.svelte';
  import { intervalUnits } from './repeat';
  import { endsBeforeStart, fieldsOf, whenOf } from './when';

  interface Props {
    /** Whether this creates a new Event or edits an existing one. */
    mode: 'create' | 'edit';
    initial: EventDraft;
    /** The Calendars the Event can go into: only writable ones. */
    calendars: Calendar[];
    /** Whether the Event's repeat rule has more than the editor shows. */
    customRepeat?: boolean;
    /** Whether the Event can repeat; an Exception can't. */
    repeatable?: boolean;
    /** Saves the draft. A rejection is shown in the editor, which stays open. */
    onsave: (draft: EventDraft) => Promise<void>;
    oncancel: () => void;
  }

  let {
    mode,
    initial,
    calendars,
    customRepeat = false,
    repeatable = true,
    onsave,
    oncancel,
  }: Props = $props();

  const id = $props.id();
  const start = untrack(() => initial);
  let title = $state(start.title);
  let calendarId = $state(start.calendarId);
  let location = $state(start.location);
  let description = $state(start.description);
  let times = $state(fieldsOf(start.when));
  let failure = $state<string>();
  let saving = $state(false);

  // The repeat's fields keep their values while it is switched off, as the
  // times do while all-day is on.
  const frequencies: Frequency[] = ['daily', 'weekly', 'monthly', 'yearly'];
  const repeatedAt = untrack(() => times.startDate);
  let frequency = $state<Frequency | 'none'>(start.repeat?.frequency ?? 'none');
  let interval = $state<number | null>(start.repeat?.interval ?? 1);
  let endKind = $state<RepeatEnd['kind']>(start.repeat?.end.kind ?? 'never');
  let untilDate = $state(
    start.repeat?.end.kind === 'until'
      ? start.repeat.end.date
      : Temporal.PlainDate.from(repeatedAt).add({ months: 1 }).toString(),
  );
  let count = $state<number | null>(
    start.repeat?.end.kind === 'count' ? start.repeat.end.count : 10,
  );

  /** Every field the user can change, serialized, to tell whether the draft changed. */
  function snapshot() {
    return JSON.stringify({
      title,
      calendarId,
      location,
      description,
      times,
      frequency,
      interval,
      endKind,
      untilDate,
      count,
    });
  }
  const started = untrack(snapshot);
  /** Whether closing is asking "Discard changes?". */
  let discarding = $state(false);

  function close() {
    if (snapshot() === started) oncancel();
    else discarding = true;
  }

  const when = $derived(whenOf(times));
  const repeat = $derived(repeatOf());
  const problem = $derived(
    when === undefined
      ? t('event.incomplete')
      : endsBeforeStart(when)
        ? t('event.endBeforeStart')
        : repeat === undefined
          ? t('repeat.invalid')
          : undefined,
  );

  /** The repeat the fields describe; `undefined` while it has no Occurrence. */
  function repeatOf(): Repeat | null | undefined {
    if (!repeatable || frequency === 'none') return null;
    const atLeastOne = (value: number | null, most: number) =>
      Number.isInteger(value) && value! >= 1 && value! <= most;
    if (!atLeastOne(interval, 999)) return undefined;
    let end: RepeatEnd = { kind: 'never' };
    if (endKind === 'count') {
      if (!atLeastOne(count, 9999)) return undefined;
      end = { kind: 'count', count: count! };
    } else if (endKind === 'until') {
      try {
        if (Temporal.PlainDate.compare(untilDate, times.startDate) < 0) return undefined;
      } catch {
        return undefined;
      }
      end = { kind: 'until', date: untilDate };
    }
    return { frequency, interval: interval!, end };
  }

  async function save(submitted: SubmitEvent) {
    submitted.preventDefault();
    if (!when || repeat === undefined || problem || saving) return;
    saving = true;
    failure = undefined;
    try {
      await onsave({ calendarId, title, when, location, description, repeat });
    } catch (error) {
      failure = t('event.saveFailed', { error: errorText(error) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog title={t(mode === 'create' ? 'event.newTitle' : 'event.editTitle')} onclose={close}>
  <form id="{id}-form" class="form" onsubmit={save}>
    <input
      class="input title"
      aria-label={t('event.title')}
      placeholder={t('event.title')}
      bind:value={title}
      data-initial-focus
      onkeydown={enterSubmits}
    />

    <label class="check">
      <input type="checkbox" bind:checked={times.allDay} />
      {t('event.allDay')}
    </label>

    <div class="times">
      <span class="field-label">{t('event.starts')}</span>
      <input
        class="input"
        type="date"
        aria-label={t('event.startDate')}
        bind:value={times.startDate}
      />
      {#if !times.allDay}
        <input
          class="input"
          type="time"
          aria-label={t('event.startTime')}
          bind:value={times.startTime}
        />
      {/if}
      <span class="field-label">{t('event.ends')}</span>
      <input class="input" type="date" aria-label={t('event.endDate')} bind:value={times.endDate} />
      {#if !times.allDay}
        <input
          class="input"
          type="time"
          aria-label={t('event.endTime')}
          bind:value={times.endTime}
        />
      {/if}
    </div>

    {#if repeatable}
      <label class="field">
        <span class="field-label">{t('event.repeat')}</span>
        <select class="input" bind:value={frequency}>
          <option value="none">{t('repeat.none')}</option>
          {#each frequencies as choice (choice)}
            <option value={choice}>{t(`repeat.${choice}`)}</option>
          {/each}
        </select>
      </label>
      {#if frequency !== 'none'}
        <div class="repeat">
          <span class="field-label">{t('repeat.every')}</span>
          <input
            class="input"
            type="number"
            min="1"
            max="999"
            aria-label={t('repeat.interval')}
            bind:value={interval}
          />
          <span>{tCount(intervalUnits[frequency], interval ?? 1)}</span>
          <span class="field-label">{t('repeat.ends')}</span>
          <select class="input" aria-label={t('repeat.ends')} bind:value={endKind}>
            <option value="never">{t('repeat.endNever')}</option>
            <option value="until">{t('repeat.endUntil')}</option>
            <option value="count">{t('repeat.endCount')}</option>
          </select>
          {#if endKind === 'until'}
            <input
              class="input"
              type="date"
              aria-label={t('repeat.untilDate')}
              bind:value={untilDate}
            />
          {:else if endKind === 'count'}
            <span class="count">
              <input
                class="input"
                type="number"
                min="1"
                max="9999"
                aria-label={t('repeat.count')}
                bind:value={count}
              />
              {tCount('repeat.times', count ?? 1)}
            </span>
          {/if}
        </div>
      {/if}
      {#if customRepeat}
        <p class="note">{t(start.repeat ? 'repeat.custom' : 'repeat.customOnly')}</p>
      {/if}
    {/if}

    <label class="field">
      <span class="field-label">{t('event.calendar')}</span>
      <select class="input" bind:value={calendarId}>
        {#each calendars as calendar (calendar.id)}
          <option value={calendar.id}>{calendar.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span class="field-label">{t('event.location')}</span>
      <input class="input" bind:value={location} />
    </label>

    <label class="field">
      <span class="field-label">{t('event.description')}</span>
      <textarea class="input" rows="4" bind:value={description}></textarea>
    </label>

    {#if problem}
      <p class="problem" role="status">{problem}</p>
    {/if}
    {#if failure}
      <p class="problem" role="alert">{failure}</p>
    {/if}
  </form>

  {#if discarding}
    <Dialog role="alertdialog" title={t('event.discardTitle')} onclose={() => (discarding = false)}>
      <p>{t('event.discardBody')}</p>
      {#snippet actions()}
        <button
          type="button"
          class="button outline"
          data-initial-focus
          onclick={() => (discarding = false)}
        >
          {t('event.keepEditing')}
        </button>
        <button type="button" class="button danger" onclick={oncancel}>
          {t('event.discard')}
        </button>
      {/snippet}
    </Dialog>
  {/if}

  {#snippet actions()}
    <button type="button" class="button outline" onclick={close}>{t('common.cancel')}</button>
    <button
      type="submit"
      class="button solid"
      form="{id}-form"
      disabled={problem !== undefined || saving}
    >
      {t('event.save')}
    </button>
  {/snippet}
</Dialog>

<style>
  .title {
    font-size: var(--text-lg);
  }

  textarea.input {
    padding: var(--space-3) var(--space-4);
    resize: vertical;
  }

  .times {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-4);
  }

  .repeat {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-4);
  }

  .repeat .count {
    display: flex;
    grid-column: span 2;
    align-items: center;
    gap: var(--space-4);
  }

  .note {
    margin: 0;
  }

  /* An all-day Event has no time inputs; its dates take the whole row. */
  .times:not(:has(input[type='time'])) {
    grid-template-columns: auto 1fr;
  }
</style>
