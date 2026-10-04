<!--
  Wraps FullCalendar (ADR 0001). It only displays the Occurrences it is given
  and reports user actions; FreeCal owns all state.
-->
<script lang="ts">
  import { Calendar } from 'fullcalendar';
  import dayGridPlugin from 'fullcalendar/daygrid';
  import classicThemePlugin from 'fullcalendar/themes/classic';
  import pulseThemePlugin from 'fullcalendar/themes/pulse';
  import monarchThemePlugin from 'fullcalendar/themes/monarch';
  import breezyThemePlugin from 'fullcalendar/themes/breezy';
  import 'fullcalendar/skeleton.css';
  import 'fullcalendar/themes/classic/theme.css';
  import 'fullcalendar/themes/classic/palette.css';
  // PROTOTYPE: themes paired with the sidebar variants.
  import 'fullcalendar/themes/pulse/theme.css';
  import 'fullcalendar/themes/pulse/palettes/blue.css';
  import 'fullcalendar/themes/monarch/theme.css';
  import 'fullcalendar/themes/monarch/palettes/blue.css';
  import 'fullcalendar/themes/breezy/theme.css';
  import 'fullcalendar/themes/breezy/palettes/indigo.css';

  const THEME_PLUGINS = { classic: classicThemePlugin, pulse: pulseThemePlugin, monarch: monarchThemePlugin, breezy: breezyThemePlugin };

  interface Props {
    /** The local date that the view opens on and highlights. */
    today: Temporal.PlainDate;
    theme?: keyof typeof THEME_PLUGINS;
  }

  let { today, theme = 'classic' }: Props = $props();

  let element: HTMLDivElement;

  $effect(() => {
    const calendar = new Calendar(element, {
      plugins: [dayGridPlugin, THEME_PLUGINS[theme]],
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
