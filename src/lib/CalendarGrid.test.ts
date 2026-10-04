import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar, Occurrence } from '../core/types';
import CalendarGrid from './CalendarGrid.svelte';

const today = Temporal.PlainDate.from('2026-10-03');

const home: Calendar = {
  id: 1,
  accountId: 1,
  name: 'Home',
  colour: '#3366cc',
  shown: true,
  readOnly: false,
};

function occurrence(fields: Partial<Occurrence> & Pick<Occurrence, 'title'>): Occurrence {
  return {
    eventId: 1,
    calendarId: home.id,
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
    ...fields,
  };
}

/** An Occurrence's chip in the grid, found by its title. */
function chip(title: string): HTMLElement {
  const found = screen.getByText(title).closest<HTMLElement>('[role="button"]');
  if (!found) throw new Error(`no chip for ${title}`);
  return found;
}

test('shows the month view of today with today highlighted', async () => {
  render(CalendarGrid, { today, occurrences: [], calendars: [] });

  expect(screen.getByRole('grid', { name: 'October 2026' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'October 3, 2026' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});

test('reports the dates it shows', async () => {
  const onrangechange = vi.fn();

  render(CalendarGrid, { today, occurrences: [], calendars: [], onrangechange });

  await waitFor(() => expect(onrangechange).toHaveBeenCalled());
  const range = onrangechange.mock.lastCall![0];
  expect(range.from.toString()).toBe('2026-09-27');
  expect(range.to.toString()).toBe('2026-11-08');
});

test('shows the Occurrences it is given in their Calendar’s colour', async () => {
  render(CalendarGrid, {
    today,
    occurrences: [
      occurrence({ eventId: 1, title: 'Dentist' }),
      occurrence({
        eventId: 2,
        title: 'Holiday',
        when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' },
      }),
      occurrence({ eventId: 3, title: '' }),
    ],
    calendars: [home],
  });

  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());
  expect(screen.getByText('Holiday')).toBeInTheDocument();
  expect(screen.getByText('(No title)')).toBeInTheDocument();
  expect(chip('Dentist').getAttribute('style')).toContain('#3366cc');
});

test('shows new Occurrences when it is given them', async () => {
  const shown = render(CalendarGrid, { today, occurrences: [], calendars: [home] });

  await shown.rerender({ occurrences: [occurrence({ title: 'Later' })] });

  await waitFor(() => expect(screen.getByText('Later')).toBeInTheDocument());
});

test('reports a click on an Occurrence, also with the keyboard', async () => {
  const user = userEvent.setup();
  const onopenevent = vi.fn();
  render(CalendarGrid, {
    today,
    occurrences: [occurrence({ eventId: 7, title: 'Dentist' })],
    calendars: [home],
    onopenevent,
  });
  await waitFor(() => expect(screen.getByText('Dentist')).toBeInTheDocument());

  await user.click(chip('Dentist'));
  chip('Dentist').focus();
  await user.keyboard('{Enter}');

  expect(onopenevent.mock.calls).toEqual([[7], [7]]);
});

test('marks the selected Event', async () => {
  render(CalendarGrid, {
    today,
    occurrences: [
      occurrence({ eventId: 1, title: 'Chosen' }),
      occurrence({ eventId: 2, title: 'Other' }),
    ],
    calendars: [home],
    selectedEventId: 1,
  });

  await waitFor(() => expect(chip('Chosen')).toHaveClass('selected-event'));
  expect(chip('Other')).not.toHaveClass('selected-event');
});

test('draws the hostile title as text, never as markup', async () => {
  render(CalendarGrid, {
    today,
    occurrences: [occurrence({ title: hostile.title })],
    calendars: [home],
  });

  await waitFor(() =>
    expect(screen.getByText(hostile.title.replace(/\s+/g, ' ').trim())).toBeInTheDocument(),
  );
  expect(document.querySelector('img, script:not([src]), b')).toBeNull();
});
