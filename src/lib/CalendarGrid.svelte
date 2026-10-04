<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state, including which Event is
  selected.
-->
<script lang="ts">
  import { Calendar as FullCalendar, type EventInput } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import interactionPlugin from 'fullcalendar/interaction';
  import breezyThemePlugin from 'fullcalendar/themes/breezy';
  import 'fullcalendar/skeleton.css';
  // The palette has light and dark values and switches on `data-color-scheme`
  // like the rest of the window (spec: "Visual style").
  import 'fullcalendar/themes/breezy/theme.css';
  import 'fullcalendar/themes/breezy/palettes/indigo.css';
  import { untrack } from 'svelte';
  import type { Calendar, EventId, Occurrence, When } from '../core/types';
  import { t } from '../i18n';
  import { plainDateOf, whenOfSelection, type VisibleRange } from './when';

  interface Props {
    /** The local date that the view opens on and highlights. */
    today: Temporal.PlainDate;
    occurrences: Occurrence[];
    /** For the colours of the Occurrences. */
    calendars: Calendar[];
    /** The Event that is open, which the grid marks. */
    selectedEventId?: EventId;
    /** The view now shows these dates. */
    onrangechange?: (range: VisibleRange) => void;
    /** The user clicked an empty slot or dragged across a range. */
    onselectslot?: (when: When) => void;
    /** The user clicked an Occurrence. */
    onopenevent?: (id: EventId) => void;
  }

  let {
    today,
    occurrences,
    calendars,
    selectedEventId,
    onrangechange,
    onselectslot,
    onopenevent,
  }: Props = $props();

  let element: HTMLDivElement;
  let grid = $state<FullCalendar>();

  $effect(() => {
    const date = today.toString();
    const calendar = new FullCalendar(element, {
      plugins: [dayGridPlugin, interactionPlugin, breezyThemePlugin],
      initialView: 'dayGridMonth',
      initialDate: date,
      now: date,
      headerToolbar: false,
      height: '100%',
      selectable: true,
      editable: false,
      datesSet: (range) =>
        onrangechange?.({ from: plainDateOf(range.start), to: plainDateOf(range.end) }),
      select: (selection) => {
        calendar.unselect();
        onselectslot?.(whenOfSelection(selection.start, selection.end, selection.allDay));
      },
      eventClick: (clicked) => {
        clicked.jsEvent.preventDefault();
        onopenevent?.(Number(clicked.event.id));
      },
    });
    // FullCalendar reports the visible range while it renders. The owner's
    // handler must not become a dependency of this effect.
    untrack(() => calendar.render());
    grid = calendar;
    return () => {
      grid = undefined;
      calendar.destroy();
    };
  });

  $effect(() => {
    const colours = new Map(calendars.map((calendar) => [calendar.id, calendar.colour]));
    const events: EventInput[] = occurrences.map((occurrence) => ({
      id: String(occurrence.eventId),
      // Titles are untrusted; FullCalendar draws them as text, never as HTML.
      title: occurrence.title || t('event.untitled'),
      start: occurrence.when.start,
      end: occurrence.when.end,
      allDay: occurrence.when.kind === 'allDay',
      color: colours.get(occurrence.calendarId),
      className: occurrence.eventId === selectedEventId ? 'selected-event' : undefined,
    }));
    const shown = grid;
    untrack(() => shown?.setOption('events', events));
  });
</script>

<div class="calendar-grid" bind:this={element}></div>

<style>
  .calendar-grid {
    height: 100%;
  }

  .calendar-grid :global(.selected-event) {
    outline: 2px solid var(--foreground);
    outline-offset: 1px;
  }
</style>
