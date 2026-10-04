<!--
  Creates or edits an Event in a modal dialog. The draft is the editor's own:
  it starts from `initial` and nothing from outside changes it afterwards,
  so a re-fetch while the editor is open never overwrites what the user typed.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { Calendar, EventDraft } from '../core/types';
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';
  import { endsBeforeStart, fieldsOf, whenOf } from './when';

  interface Props {
    /** Whether this creates a new Event or edits an existing one. */
    mode: 'create' | 'edit';
    initial: EventDraft;
    /** The Calendars the Event can go into: only writable ones. */
    calendars: Calendar[];
    /** Saves the draft. A rejection is shown in the editor, which stays open. */
    onsave: (draft: EventDraft) => Promise<void>;
    oncancel: () => void;
  }

  let { mode, initial, calendars, onsave, oncancel }: Props = $props();

  const id = $props.id();
  const start = untrack(() => initial);
  let title = $state(start.title);
  let calendarId = $state(start.calendarId);
  let location = $state(start.location);
  let description = $state(start.description);
  let times = $state(fieldsOf(start.when));
  let failure = $state<string>();
  let saving = $state(false);

  const when = $derived(whenOf(times));
  const problem = $derived(
    when === undefined
      ? t('event.incomplete')
      : endsBeforeStart(when)
        ? t('event.endBeforeStart')
        : undefined,
  );

  async function save(submitted: SubmitEvent) {
    submitted.preventDefault();
    if (!when || problem || saving) return;
    saving = true;
    failure = undefined;
    try {
      await onsave({ calendarId, title, when, location, description });
    } catch (error) {
      failure = t('event.saveFailed', { error: String(error) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog title={t(mode === 'create' ? 'event.newTitle' : 'event.editTitle')} onclose={oncancel}>
  <form id="{id}-form" class="form" onsubmit={save}>
    <input
      class="input title"
      aria-label={t('event.title')}
      placeholder={t('event.title')}
      bind:value={title}
      data-initial-focus
      onkeydown={(key) => {
        // Enter saves. Done here, because the Save button sits outside the
        // form, in the dialog's actions.
        if (key.key === 'Enter') {
          key.preventDefault();
          key.currentTarget.form?.requestSubmit();
        }
      }}
    />

    <label class="check">
      <input type="checkbox" bind:checked={times.allDay} />
      {t('event.allDay')}
    </label>

    <div class="times">
      <span class="label">{t('event.starts')}</span>
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
      <span class="label">{t('event.ends')}</span>
      <input
        class="input"
        type="date"
        aria-label={t('event.endDate')}
        bind:value={times.endDate}
      />
      {#if !times.allDay}
        <input
          class="input"
          type="time"
          aria-label={t('event.endTime')}
          bind:value={times.endTime}
        />
      {/if}
    </div>

    <label class="field">
      <span class="label">{t('event.calendar')}</span>
      <select class="input" bind:value={calendarId}>
        {#each calendars as calendar (calendar.id)}
          <option value={calendar.id}>{calendar.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span class="label">{t('event.location')}</span>
      <input class="input" bind:value={location} />
    </label>

    <label class="field">
      <span class="label">{t('event.description')}</span>
      <textarea class="input" rows="4" bind:value={description}></textarea>
    </label>

    {#if problem}
      <p class="problem" role="status">{problem}</p>
    {/if}
    {#if failure}
      <p class="problem" role="alert">{failure}</p>
    {/if}
  </form>

  {#snippet actions()}
    <button type="button" class="button outline" onclick={oncancel}>{t('common.cancel')}</button>
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
  .form {
    display: grid;
    gap: var(--space-6);
    color: var(--foreground);
  }

  .form .input {
    min-height: var(--control-height);
    padding: 0 var(--space-4);
  }

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

  /* An all-day Event has no time inputs; its dates take the whole row. */
  .times:not(:has(input[type='time'])) {
    grid-template-columns: auto 1fr;
  }

  .field {
    display: grid;
    gap: var(--space-2);
  }

  .label {
    color: var(--foreground-muted);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .problem {
    color: var(--danger);
  }

  .button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
