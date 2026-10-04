import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import App from './App.svelte';
import { FakeCore } from './test/fakeCore';

test('the sidebar shows the Local Account', async () => {
  render(App, { core: new FakeCore() });

  const sidebar = await screen.findByRole('navigation', { name: 'Calendars' });
  expect(sidebar).toHaveTextContent('Local');
});

test('opens an empty month view on the core’s today', async () => {
  const core = new FakeCore();
  core.currentDate = Temporal.PlainDate.from('2027-02-14');

  render(App, { core });

  expect(await screen.findByRole('grid', { name: 'February 2027' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'February 14, 2027' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});

test('the grid shows the core’s Occurrences and follows “Occurrences changed”', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  core.addEvent({
    calendarId: home.id,
    title: 'Dentist',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
  });

  render(App, { core });

  expect(await screen.findByText('Dentist')).toBeInTheDocument();
  core.addEvent({
    calendarId: home.id,
    title: 'Holiday',
    when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' },
  });
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });
  expect(await screen.findByText('Holiday')).toBeInTheDocument();
});
