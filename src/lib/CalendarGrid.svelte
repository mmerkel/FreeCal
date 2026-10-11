<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state, including which Event is
  selected.
-->
<script lang="ts">
  import { Calendar as FullCalendar, type EventInput } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import interactionPlugin from 'fullcalendar/interaction';
  import classicThemePlugin from 'fullcalendar/themes/classic';
  import 'fullcalendar/skeleton.css';
  // `app.css` points the palette's variables at FreeCal's tokens, so the grid
  // has the window's light and dark colours (spec: "Visual style").
  import 'fullcalendar/themes/classic/theme.css';
  import 'fullcalendar/themes/classic/palette.css';
  import { untrack } from 'svelte';
  import type { Calendar, EventId, Occurrence, When } from '../core/types';
  import { t } from '../i18n';
  import { gridTimeFormat } from './clock.svelte';
  import { textColourOn } from './palette';
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
      plugins: [dayGridPlugin, interactionPlugin, classicThemePlugin],
      // In addition to the theme's classes, which cut titles off without "…".
      eventTitleClass: 'event-title',
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
        // FullCalendar keeps the ID as a string; it came from an Occurrence.
        onopenevent?.(Number(clicked.event.id) as EventId);
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
    const format = gridTimeFormat();
    const shown = grid;
    untrack(() => {
      shown?.setOption('eventTimeFormat', format);
      shown?.setOption('slotHeaderFormat', format);
    });
  });

  $effect(() => {
    const colours = new Map(calendars.map((calendar) => [calendar.id, calendar.colour]));
    const events: EventInput[] = occurrences.map((occurrence) => {
      const colour = colours.get(occurrence.calendarId);
      return {
        id: String(occurrence.eventId),
        // Titles are untrusted; FullCalendar draws them as text, never as HTML.
        title: occurrence.title || t('event.untitled'),
        start: occurrence.when.start,
        end: occurrence.when.end,
        allDay: occurrence.when.kind === 'allDay',
        color: colour,
        contrastColor: colour && textColourOn(colour),
        className: occurrence.eventId === selectedEventId ? 'selected-event' : undefined,
      };
    });
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

  .calendar-grid :global(.event-title) {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
