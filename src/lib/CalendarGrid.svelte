<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state.
-->
<script lang="ts">
  import { Calendar } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import breezyThemePlugin from 'fullcalendar/themes/breezy';
  import 'fullcalendar/skeleton.css';
  // The palette has light and dark values and switches on `data-color-scheme`
  // like the rest of the window (spec: "Visual style").
  import 'fullcalendar/themes/breezy/theme.css';
  import 'fullcalendar/themes/breezy/palettes/indigo.css';

  interface Props {
    /** The local date that the view opens on and highlights. */
    today: Temporal.PlainDate;
  }

  let { today }: Props = $props();

  let element: HTMLDivElement;

  $effect(() => {
    const calendar = new Calendar(element, {
      plugins: [dayGridPlugin, breezyThemePlugin],
      initialView: 'dayGridMonth',
      initialDate: today.toString(),
      now: today.toString(),
      headerToolbar: false,
      height: '100%',
    });
    calendar.render();
    return () => calendar.destroy();
  });
</script>

<div class="calendar-grid" bind:this={element}></div>

<style>
  .calendar-grid {
    height: 100%;
  }
</style>
