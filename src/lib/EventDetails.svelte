<!--
  An Event's details: its title, time, repeat, location, description and
  Calendar, with Edit and Delete for Events of writable Calendars; the others
  say they are read-only and why. Deleting an Occurrence of a Recurring Event
  asks whether to delete it alone or all.
-->
<script lang="ts">
  import Calendar from '@lucide/svelte/icons/calendar';
  import Clock from '@lucide/svelte/icons/clock';
  import Lock from '@lucide/svelte/icons/lock';
  import MapPin from '@lucide/svelte/icons/map-pin';
  import Repeat from '@lucide/svelte/icons/repeat';
  import Text from '@lucide/svelte/icons/text';
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
  const repeat = $derived(describeRepeat(event.repeat, event.customRepeat));
  /** Whether Delete asks first: for an Event that repeats or is an Exception. */
  const recurring = $derived(
    repeat !== undefined || event.recurrenceId !== null || ondeleteoccurrence !== undefined,
  );
  /** Whether Delete asks which Occurrences to delete. */
  let asking = $state(false);

  function startDeleting() {
    if (recurring) asking = true;
    else ondelete();
  }
</script>

<Dialog title={event.title || t('event.untitled')} {onclose}>
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
  {#if asking}
    <p class="question">{t('event.deleteWhich')}</p>
  {/if}
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#snippet actions()}
    {#if asking}
      <button
        type="button"
        class="button outline"
        onclick={() => (asking = false)}
        {@attach (button) => button.focus()}
      >
        {t('common.cancel')}
      </button>
      <button type="button" class="button danger" onclick={ondelete}>
        {t('event.deleteAll')}
      </button>
      {#if ondeleteoccurrence}
        <button type="button" class="button danger" onclick={ondeleteoccurrence}>
          {t('event.deleteThis')}
        </button>
      {/if}
    {:else}
      {#if writable}
        <button type="button" class="button outline danger-text" onclick={startDeleting}>
          {t('event.delete')}
        </button>
        <button type="button" class="button outline" onclick={onedit}>{t('event.edit')}</button>
      {/if}
      <button type="button" class="button solid" data-initial-focus onclick={onclose}>
        {t('event.close')}
      </button>
    {/if}
  {/snippet}
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

  .failure,
  .question {
    margin: var(--space-6) 0 0;
  }

  .failure {
    color: var(--danger);
  }

  .danger-text {
    margin-inline-end: auto;
    color: var(--danger);
  }
</style>
