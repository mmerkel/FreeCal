<!--
  An Event's details: its title, time, place, description and Calendar, with
  Edit and Delete for Events of writable Calendars.
-->
<script lang="ts">
  import Calendar from '@lucide/svelte/icons/calendar';
  import Clock from '@lucide/svelte/icons/clock';
  import MapPin from '@lucide/svelte/icons/map-pin';
  import Text from '@lucide/svelte/icons/text';
  import type { Calendar as CalendarType, Event } from '../core/types';
  import { t } from '../i18n';
  import DescriptionView from './DescriptionView.svelte';
  import Dialog from './Dialog.svelte';
  import { describeWhen } from './when';

  interface Props {
    event: Event;
    /** The Event's Calendar, if it is still there. */
    calendar: CalendarType | undefined;
    /** A failure to report, such as a delete the core refused. */
    failure?: string;
    onedit: () => void;
    ondelete: () => void;
    onclose: () => void;
    onopenlink: (url: string) => void;
  }

  let { event, calendar, failure, onedit, ondelete, onclose, onopenlink }: Props = $props();

  const writable = $derived(calendar !== undefined && !calendar.readOnly);
</script>

<Dialog title={event.title || t('event.untitled')} {onclose}>
  <dl class="details">
    <div class="row">
      <dt><Clock size={14} aria-label={t('event.starts')} /></dt>
      <dd>{describeWhen(event.when)}</dd>
    </div>
    {#if event.location}
      <div class="row">
        <dt><MapPin size={14} aria-label={t('event.location')} /></dt>
        <dd class="text">{event.location}</dd>
      </div>
    {/if}
    {#if event.description.paragraphs.length > 0}
      <div class="row">
        <dt><Text size={14} aria-label={t('event.description')} /></dt>
        <dd><DescriptionView description={event.description} {onopenlink} /></dd>
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
    {/if}
  </dl>
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#snippet actions()}
    {#if writable}
      <button type="button" class="button outline danger-text" onclick={ondelete}>
        {t('event.delete')}
      </button>
      <button type="button" class="button outline" onclick={onedit}>{t('event.edit')}</button>
    {/if}
    <button type="button" class="button solid" data-initial-focus onclick={onclose}>
      {t('event.close')}
    </button>
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

  .danger-text {
    margin-inline-end: auto;
    color: var(--danger);
  }
</style>
