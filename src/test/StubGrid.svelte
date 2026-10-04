<!--
  Stands in for CalendarGrid in tests of the App's flows, which jsdom can't
  drive through FullCalendar's drag and select. It lists the Occurrences as
  buttons that open them, reports a month around today as the visible range,
  and leaves its props in `stubGrid` for a test to report a selection.
-->
<script lang="ts">
  import type { ComponentProps } from 'svelte';
  import { untrack } from 'svelte';
  import type CalendarGrid from '../lib/CalendarGrid.svelte';
  import { stubGrid } from './stubGrid';

  let props: ComponentProps<typeof CalendarGrid> = $props();

  $effect.pre(() => {
    stubGrid.props = props;
    return () => {
      if (stubGrid.props === props) stubGrid.props = undefined;
    };
  });

  $effect(() => {
    untrack(() => {
      const from = props.today.with({ day: 1 }).subtract({ days: 7 });
      props.onrangechange?.({ from, to: from.add({ days: 42 }) });
    });
  });
</script>

<ul aria-label="Grid">
  {#each props.occurrences as occurrence (occurrence.eventId)}
    <li>
      <button
        type="button"
        aria-current={occurrence.eventId === props.selectedEventId}
        onclick={() => props.onopenevent?.(occurrence.eventId)}
      >
        {occurrence.title}
      </button>
    </li>
  {/each}
</ul>
