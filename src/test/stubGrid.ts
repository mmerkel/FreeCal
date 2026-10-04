import type { ComponentProps } from 'svelte';
import type CalendarGrid from '../lib/CalendarGrid.svelte';

/**
 * The props of the mounted `StubGrid`, so that a test can report what the
 * user did in the grid, such as selecting a slot, as FullCalendar would.
 */
export const stubGrid: { props?: ComponentProps<typeof CalendarGrid> } = {};
