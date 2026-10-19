<!--
  An Event's details: its title, time, repeat, location, description and
  Calendar, with Edit and Delete icons for Events of writable Calendars; the
  others say they are read-only and why. Delete asks first, and for an
  Occurrence of a Recurring Event whether to delete it alone or all. A click
  on the backdrop closes the details.
-->
<script lang="ts">
  import Calendar from '@lucide/svelte/icons/calendar';
  import Clock from '@lucide/svelte/icons/clock';
  import Lock from '@lucide/svelte/icons/lock';
  import MapPin from '@lucide/svelte/icons/map-pin';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Repeat from '@lucide/svelte/icons/repeat';
  import Text from '@lucide/svelte/icons/text';
  import Trash from '@lucide/svelte/icons/trash-2';
  import type { Calendar as CalendarType, Event, When } from '../core/types';
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';
  import { describeRepeat } from './repeat';
  import StructuredTextView from './StructuredTextView.svelte';
  import { describeWhen } from './when';

  interface Props {
    event: Event;
    /** The Event's Calendar, if it is still there. */
    calendar: CalendarType | undefined;
    /** When the Occurrence that was opened takes place, if it isn't the Event's first. */
    when?: When;
    /** A failure to report, such as a delete the core refused. */
    failure?: string;
    onedit: () => void;
    /** Deletes the Event; for a Recurring Event, all its Occurrences. */
    ondelete: () => void;
    /** Deletes only the Occurrence that was opened, if it is one of a Recurring Event. */
    ondeleteoccurrence?: () => void;
    onclose: () => void;
    onopenlink: (url: string) => void;
  }

  let {
    event,
    calendar,
    when = event.when,
    failure,
    onedit,
    ondelete,
    ondeleteoccurrence,
    onclose,
    onopenlink,
  }: Props = $props();

  const writable = $derived(calendar !== undefined && !calendar.readOnly);
  const title = $derived(event.title || t('event.untitled'));
  const repeat = $derived(describeRepeat(event.repeat, event.customRepeat));
  /** Whether Delete asks which Occurrences to delete: for an Event that repeats or is an Exception. */
  const recurring = $derived(
    repeat !== undefined || event.recurrenceId !== null || ondeleteoccurrence !== undefined,
  );
  /** Whether Delete is asking for confirmation. */
  let asking = $state(false);

  /** Closes the question, then deletes as answered. */
  function answer(deleting: () => void) {
    asking = false;
    deleting();
  }
</script>

<Dialog {title} {onclose} backdropCloses>
  {#snippet tools()}
    {#if writable}
      <button
        type="button"
        class="icon-button"
        aria-label={t('event.edit')}
        title={t('event.edit')}
        onclick={onedit}
      >
        <Pencil size={16} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="icon-button"
        aria-label={t('event.delete')}
        title={t('event.delete')}
        onclick={() => (asking = true)}
      >
        <Trash size={16} aria-hidden="true" />
      </button>
    {/if}
  {/snippet}

  <dl class="details">
    <div class="row">
      <dt><Clock size={14} aria-label={t('event.starts')} /></dt>
      <dd>
        {describeWhen(when)}
        {#if event.pinnedZone !== null}
          <p class="note">{t('event.pinnedZone', { zone: event.pinnedZone })}</p>
        {/if}
      </dd>
    </div>
    {#if repeat}
      <div class="row">
        <dt><Repeat size={14} aria-label={t('event.repeat')} /></dt>
        <dd>{repeat}</dd>
      </div>
    {/if}
    {#if event.location.paragraphs.length > 0}
      <div class="row">
        <dt><MapPin size={14} aria-label={t('event.location')} /></dt>
        <dd><StructuredTextView content={event.location} {onopenlink} /></dd>
      </div>
    {/if}
    {#if event.description.paragraphs.length > 0}
      <div class="row">
        <dt><Text size={14} aria-label={t('event.description')} /></dt>
        <dd><StructuredTextView content={event.description} {onopenlink} /></dd>
      </div>
    {/if}
    {#if calendar}
      <div class="row">
        <dt><Calendar size={14} aria-label={t('event.calendar')} /></dt>
        <dd class="calendar">
          <span class="dot" style:--colour={calendar.colour} aria-hidden="true"></span>
          <span class="text">{calendar.name}</span>
        </dd>
      </div>
      {#if calendar.readOnly}
        <div class="row">
          <dt><Lock size={14} aria-label={t('event.readOnly')} /></dt>
          <dd>{t('event.readOnlyCalendar')}</dd>
        </div>
      {/if}
    {/if}
  </dl>
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#if asking}
    <Dialog role="alertdialog" title={t('event.deleteTitle')} onclose={() => (asking = false)}>
      {#if recurring}
        <p>{t('event.deleteRecurring', { title })}</p>
        <p>{t('event.deleteWhich')}</p>
      {:else}
        <p>{t('event.deleteBody', { title })}</p>
      {/if}
      {#snippet actions()}
        <button
          type="button"
          class="button outline"
          data-initial-focus
          onclick={() => (asking = false)}
        >
          {t('common.cancel')}
        </button>
        {#if recurring}
          <button type="button" class="button danger" onclick={() => answer(ondelete)}>
            {t('event.deleteAll')}
          </button>
          {#if ondeleteoccurrence}
            {@const deleteOccurrence = ondeleteoccurrence}
            <button type="button" class="button danger" onclick={() => answer(deleteOccurrence)}>
              {t('event.deleteThis')}
            </button>
          {/if}
        {:else}
          <button type="button" class="button danger" onclick={() => answer(ondelete)}>
            {t('event.delete')}
          </button>
        {/if}
      {/snippet}
    </Dialog>
  {/if}
</Dialog>

<style>
  .details {
    display: grid;
    gap: var(--space-5);
    margin: 0;
    color: var(--foreground);
  }

  .row {
    display: grid;
    grid-template-columns: 1rem 1fr;
    gap: var(--space-5);
    align-items: start;
  }

  dt {
    display: flex;
    padding-top: 0.15em;
    color: var(--foreground-muted);
  }

  dd {
    min-width: 0;
    margin: 0;
  }

  .note {
    margin: var(--space-2) 0 0;
    overflow-wrap: anywhere;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .calendar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .dot {
    flex: none;
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    background: var(--colour);
  }

  .failure {
    margin: var(--space-6) 0 0;
    color: var(--danger);
  }

  p + p {
    margin-top: var(--space-4);
  }
</style>
