<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state, including which Event is
  selected.
-->
<script lang="ts">
  import { Calendar as FullCalendar, type EventInput } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import interactionPlugin from 'fullcalendar/interaction';
  import timeGridPlugin from 'fullcalendar/timegrid';
  import classicThemePlugin from 'fullcalendar/themes/classic';
  import 'fullcalendar/skeleton.css';
  // `app.css` points the palette's variables at FreeCal's tokens, so the grid
  // has the window's light and dark colours (spec: "Visual style").
  import 'fullcalendar/themes/classic/theme.css';
  import 'fullcalendar/themes/classic/palette.css';
  import { onDestroy, untrack } from 'svelte';
  import type { Calendar, EventId, Occurrence, View, When } from '../core/types';
  import { t } from '../i18n';
  import { gridTimeFormat } from './clock.svelte';
  import { textColourOn } from './palette';
  import { firstWeekdayNumber } from './week.svelte';
  import {
    gridId,
    idOf,
    recurrenceIdOf,
    rescheduleOfDrop,
    rescheduleOfResize,
    type Reschedule,
  } from './gridDrag';
  import { allDaySpan, plainDateOf, whenOfSelection, type VisibleRange } from './when';

  interface Props {
    view: View;
    /** The date whose day, week or month is shown. */
    date: Temporal.PlainDate;
    /** The local date that the grid highlights. */
    today: Temporal.PlainDate;
    occurrences: Occurrence[];
    /** For the colours of the Occurrences. */
    calendars: Calendar[];
    /** The Event that is open, which the grid marks. */
    selectedEventId?: EventId;
    /**
     * Which Occurrence of the open Event is open, if it is one of a Recurring
     * Event; without it, all its Occurrences are marked.
     */
    selectedRecurrenceId?: string | null;
    /** The view now shows these dates. */
    onrangechange?: (range: VisibleRange) => void;
    /** The user clicked an empty slot or dragged across a range. */
    onselectslot?: (when: When) => void;
    /** The user clicked an Occurrence: its Event and its `recurrenceId`. */
    onopenevent?: (id: EventId, recurrenceId: string | null) => void;
    /** The user clicked a day number or a day's heading. */
    onopenday?: (date: Temporal.PlainDate) => void;
    /**
     * The user dragged an Occurrence to another time or dragged its edge. If
     * the promise rejects, the Occurrence goes back to where it was.
     */
    onreschedule?: (id: EventId, when: When) => Promise<void>;
    /**
     * The user started (`true`) or ended (`false`) pressing on the grid, as
     * for a drag, a resize or a drag-to-create. Meanwhile the grid holds back
     * a new today, and its owner should hold back moving the view.
     */
    oninteract?: (interacting: boolean) => void;
  }

  /** FullCalendar's names for FreeCal's views. */
  const viewNames = { day: 'timeGridDay', week: 'timeGridWeek', month: 'dayGridMonth' } as const;

  let {
    view,
    date,
    today,
    occurrences,
    calendars,
    selectedEventId,
    selectedRecurrenceId,
    onrangechange,
    onselectslot,
    onopenevent,
    onopenday,
    onreschedule,
    oninteract,
  }: Props = $props();

  let element: HTMLDivElement;
  let grid = $state<FullCalendar>();

  /**
   * Whether the user is pressing on the grid. A new grid would drop a drag,
   * a resize or a drag-to-create without a word, so a new today waits.
   */
  let interacting = $state(false);
  let shownToday = $state(untrack(() => today));
  let interactionEnd: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const next = today;
    if (!interacting) shownToday = next;
  });

  function startInteracting() {
    clearTimeout(interactionEnd);
    interactionEnd = undefined;
    if (interacting) return;
    interacting = true;
    oninteract?.(true);
  }

  function endInteracting() {
    if (!interacting || interactionEnd !== undefined) return;
    // After FullCalendar has handled the same release, so that the drop or
    // selection lands on the grid it started on.
    interactionEnd = setTimeout(() => {
      interactionEnd = undefined;
      interacting = false;
      oninteract?.(false);
    });
  }

  onDestroy(() => clearTimeout(interactionEnd));

  /** Counts drags the owner refused, so that the Occurrences are drawn again. */
  let redraws = $state(0);

  function reschedule({ id, when }: Reschedule) {
    // The grid may be a new one by the time the core answers, so the
    // Occurrences are drawn again rather than the drag reverted.
    onreschedule?.(id, when).catch(() => redraws++);
  }

  // FullCalendar reads `now` only when it is created, so a new today, once a
  // day, builds a new grid.
  $effect(() => {
    const now = shownToday.toString();
    // The effects below follow changes of these; reading them here only
    // saves the first render a wrong range.
    const initial = untrack(() => ({
      view: viewNames[view],
      date: date.toString(),
      firstDay: firstWeekdayNumber(),
    }));
    const calendar = new FullCalendar(element, {
      plugins: [dayGridPlugin, timeGridPlugin, interactionPlugin, classicThemePlugin],
      // In addition to the theme's classes, which cut titles off without "…".
      eventTitleClass: 'event-title',
      eventClass: 'event-chip',
      // Bars and time-grid chips, filled with their Calendar colour; not the
      // month view's dots.
      blockEventClass: 'event-block',
      initialView: initial.view,
      initialDate: initial.date,
      firstDay: initial.firstDay,
      // A function, because a fixed `now` would run on with the system
      // clock; the core says when the date changes.
      now: () => now,
      headerToolbar: false,
      // Day numbers and day headings open their day; FreeCal decides what
      // that does to the view (spec: "Tracking today").
      navLinks: true,
      navLinkDayClick: (day) => onopenday?.(plainDateOf(day)),
      height: '100%',
      selectable: true,
      // Read-only Occurrences say otherwise (spec: "Event chips stay sparse").
      editable: true,
      eventResizableFromStart: true,
      forceEventDuration: true,
      // Overlapping Occurrences side by side, none hidden behind another.
      slotEventOverlap: false,
      datesSet: (range) =>
        onrangechange?.({ from: plainDateOf(range.start), to: plainDateOf(range.end) }),
      select: (selection) => {
        calendar.unselect();
        onselectslot?.(whenOfSelection(selection.start, selection.end, selection.allDay));
      },
      eventClick: (clicked) => {
        clicked.jsEvent.preventDefault();
        onopenevent?.(idOf(clicked.event), recurrenceIdOf(clicked.event));
      },
      eventDrop: (dropped) => reschedule(rescheduleOfDrop(dropped)),
      eventResize: (resized) => reschedule(rescheduleOfResize(resized)),
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
    const name = viewNames[view];
    const shown = date.toString();
    const calendar = grid;
    untrack(() => calendar?.changeView(name, shown));
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
    const firstDay = firstWeekdayNumber();
    const shown = grid;
    untrack(() => shown?.setOption('firstDay', firstDay));
  });

  $effect(() => {
    // Draws the Occurrences again after a refused drag.
    redraws;
    // Without a handler there is nothing to tell about a drag.
    const movable = onreschedule !== undefined;
    const calendarsById = new Map(calendars.map((calendar) => [calendar.id, calendar]));
    const timeGrid = view !== 'month';
    const events: EventInput[] = occurrences.map((occurrence) => {
      const calendar = calendarsById.get(occurrence.calendarId);
      const colour = calendar?.colour;
      // A timed Occurrence of a day or more goes to the all-day area, so that
      // it doesn't cover the time grid.
      const span = timeGrid ? allDaySpan(occurrence.when) : undefined;
      const selected =
        occurrence.eventId === selectedEventId &&
        (selectedRecurrenceId === undefined || occurrence.recurrenceId === selectedRecurrenceId);
      return {
        id: gridId(occurrence),
        // Titles are untrusted; FullCalendar draws them as text, never as HTML.
        title: occurrence.title || t('event.untitled'),
        start: span?.start ?? occurrence.when.start,
        end: span?.end ?? occurrence.when.end,
        allDay: span !== undefined || occurrence.when.kind === 'allDay',
        // Moving an Occurrence of a Recurring Event asks for the recurrence
        // scope first (ticket 08).
        editable:
          movable &&
          calendar !== undefined &&
          !calendar.readOnly &&
          occurrence.recurrenceId === null,
        extendedProps: { when: occurrence.when, recurrenceId: occurrence.recurrenceId },
        color: colour,
        contrastColor: colour && textColourOn(colour),
        className: selected ? 'selected-event' : undefined,
      };
    });
    const shown = grid;
    untrack(() => shown?.setOption('events', events));
  });
</script>

<svelte:window
  onmouseup={endInteracting}
  ontouchend={endInteracting}
  ontouchcancel={endInteracting}
/>

<!-- The same mouse and touch events FullCalendar starts and ends its drags
     with. While a button is held the window keeps the pointer, so the release
     arrives even outside it. -->
<div
  class="calendar-grid"
  bind:this={element}
  onmousedowncapture={startInteracting}
  ontouchstartcapture={startInteracting}
></div>

<style>
  .calendar-grid {
    flex: 1;
    min-height: 0;
  }

  /* Every Event opens its details, so the pointer is a hand over the whole
     chip and its text can't be selected (spec: "Event chips stay sparse"). */
  .calendar-grid :global(.event-chip) {
    cursor: pointer;
    user-select: none;
  }

  /* Filled chips darken on hover, as the theme greys the dots. An overlay on
     the background, not the grey `--hover`, works on any Calendar colour and
     leaves the text and the outlines alone (spec: "Event chips stay
     sparse"). */
  @media (hover: hover) {
    .calendar-grid :global(.event-block:hover) {
      background-image: linear-gradient(rgb(0 0 0 / 15%), rgb(0 0 0 / 15%));
    }
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
