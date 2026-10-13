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
  import { untrack } from 'svelte';
  import type { Calendar, EventId, Occurrence, View, When } from '../core/types';
  import { t } from '../i18n';
  import { gridTimeFormat } from './clock.svelte';
  import { textColourOn } from './palette';
  import { firstWeekdayNumber } from './week.svelte';
  import { plainDateOf, whenOfSelection, type VisibleRange } from './when';

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
    /** The view now shows these dates. */
    onrangechange?: (range: VisibleRange) => void;
    /** The user clicked an empty slot or dragged across a range. */
    onselectslot?: (when: When) => void;
    /** The user clicked an Occurrence. */
    onopenevent?: (id: EventId) => void;
    /** The user clicked a day number or a day's heading. */
    onopenday?: (date: Temporal.PlainDate) => void;
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
    onrangechange,
    onselectslot,
    onopenevent,
    onopenday,
  }: Props = $props();

  let element: HTMLDivElement;
  let grid = $state<FullCalendar>();

  // FullCalendar reads `now` only when it is created, so a new today, once a
  // day, builds a new grid.
  $effect(() => {
    const now = today.toString();
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
    flex: 1;
    min-height: 0;
  }

  /* Every Event opens its details, so the pointer is a hand over the whole
     chip and its text can't be selected (spec: "Event chips stay sparse"). */
  .calendar-grid :global(.event-chip) {
    cursor: pointer;
    user-select: none;
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
