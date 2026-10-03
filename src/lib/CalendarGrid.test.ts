import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import CalendarGrid from './CalendarGrid.svelte';

test('shows the month view of today with today highlighted', async () => {
  render(CalendarGrid, { today: '2026-10-03' });

  expect(screen.getByRole('grid', { name: 'October 2026' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'October 3, 2026' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});
