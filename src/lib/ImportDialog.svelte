<!--
  The import dialog for a picked file: the user chooses a writable Calendar,
  sees the preview for it, and when some of the file's Events are already in
  that Calendar, chooses once what to do with them. The preview is read again
  whenever the Calendar changes.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CoreApi } from '../core/CoreApi';
  import type { Calendar, CalendarId, ExistingEvents, ImportSummary } from '../core/types';
  import { errorText, t, tCount, type MessageKey } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    core: CoreApi;
    /** The picked file's name. */
    file: string;
    /** The Calendars it can go into: the writable ones. */
    calendars: Calendar[];
    initialCalendarId: CalendarId;
    /** Why the last import failed, if it did. */
    failure?: string;
    onimport: (calendarId: CalendarId, existing: ExistingEvents) => void;
    oncancel: () => void;
  }

  let { core, file, calendars, initialCalendarId, failure, onimport, oncancel }: Props = $props();

  const id = $props.id();

  let calendarId = $state(untrack(() => initialCalendarId));
  let summary = $state<ImportSummary>();
  let previewFailure = $state<string>();
  let existing = $state<ExistingEvents>('update');

  const choices: { value: ExistingEvents; label: MessageKey }[] = [
    { value: 'update', label: 'import.update' },
    { value: 'skip', label: 'import.skip' },
    { value: 'copy', label: 'import.copy' },
  ];

  const empty = $derived(summary !== undefined && summary.newEvents + summary.existingEvents === 0);

  // Previews can overtake each other; only the one for the chosen Calendar may land.
  let latestPreview = 0;

  // A Calendar deleted or made Read-only meanwhile is no longer offered.
  $effect(() => {
    if (!calendars.some((calendar) => calendar.id === calendarId) && calendars[0])
      calendarId = calendars[0].id;
  });

  $effect(() => {
    const chosen = calendarId;
    const preview = ++latestPreview;
    summary = undefined;
    previewFailure = undefined;
    core.previewImport(chosen).then(
      (read) => {
        if (preview === latestPreview) summary = read;
      },
      (error) => {
        if (preview === latestPreview)
          previewFailure = t('import.previewFailed', { error: errorText(error) });
      },
    );
  });
</script>

<Dialog title={t('import.title', { file })} onclose={oncancel}>
  <div class="form">
    <label class="field">
      <span class="field-label">{t('import.calendar')}</span>
      <select class="input" bind:value={calendarId} data-initial-focus>
        {#each calendars as calendar (calendar.id)}
          <option value={calendar.id}>{calendar.name}</option>
        {/each}
      </select>
    </label>

    <div class="preview" aria-live="polite">
      {#if previewFailure}
        <p class="problem" role="alert">{previewFailure}</p>
      {:else if !summary}
        <p>{t('import.reading')}</p>
      {:else if empty}
        <p>{t('import.noEvents')}</p>
      {:else}
        <p>{tCount('import.newEvents', summary.newEvents)}</p>
        {#if summary.existingEvents > 0}
          <p>{tCount('import.existingEvents', summary.existingEvents)}</p>
        {/if}
        {#each summary.pinned as pinned (pinned.zone)}
          <p>{tCount('import.pinned', pinned.events, { zone: pinned.zone })}</p>
        {/each}
      {/if}
    </div>

    {#if summary && summary.existingEvents > 0}
      <fieldset class="choices">
        <legend class="field-label">{t('import.existing')}</legend>
        {#each choices as choice (choice.value)}
          <label class="check">
            <input type="radio" name="{id}-existing" value={choice.value} bind:group={existing} />
            {t(choice.label)}
          </label>
        {/each}
      </fieldset>
    {/if}

    {#if failure}
      <p class="problem" role="alert">{failure}</p>
    {/if}
  </div>

  {#snippet actions()}
    <button type="button" class="button outline" onclick={oncancel}>{t('common.cancel')}</button>
    <button
      type="button"
      class="button solid"
      disabled={!summary || empty}
      onclick={() => onimport(calendarId, existing)}
    >
      {t('import.run')}
    </button>
  {/snippet}
</Dialog>

<style>
  .preview {
    display: grid;
    gap: var(--space-2);
  }
</style>
