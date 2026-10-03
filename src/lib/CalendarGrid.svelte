<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state.
-->
<script lang="ts">
  import { Calendar } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import classicThemePlugin from 'fullcalendar/themes/classic';
  import 'fullcalendar/skeleton.css';
  import 'fullcalendar/themes/classic/theme.css';
  import 'fullcalendar/themes/classic/palette.css';

  interface Props {
    /** The local date, as YYYY-MM-DD, that the view opens on and highlights. */
    today: string;
  }

  let { today }: Props = $props();

  let element: HTMLDivElement;

  $effect(() => {
    const calendar = new Calendar(element, {
      plugins: [dayGridPlugin, classicThemePlugin],
      initialView: 'dayGridMonth',
      initialDate: today,
      now: today,
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
