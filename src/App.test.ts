import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import App from './App.svelte';
import { calendar, event, firstLaunch, scriptWindow } from './test/answers';

test('the sidebar shows the Local Account', async () => {
  render(App, { core: firstLaunch() });

  const sidebar = await screen.findByRole('navigation', { name: 'Calendars' });
  expect(sidebar).toHaveTextContent('Local');
});

test('opens an empty month view on the core’s today', async () => {
  const core = firstLaunch();
  core.answer('today', Temporal.PlainDate.from('2027-02-14'));

  render(App, { core });

  expect(await screen.findByRole('grid', { name: 'February 2027' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'February 14, 2027' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});

test('the grid shows the core’s Occurrences and follows “Occurrences changed”', async () => {
  const core = firstLaunch();
  const home = calendar({ name: 'Home' });
  const dentist = event({
    calendarId: home.id,
    title: 'Dentist',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
  });
  const holiday = event({
    calendarId: home.id,
    title: 'Holiday',
    when: { kind: 'allDay', start: '2026-10-07', end: '2026-10-08' },
  });
  scriptWindow(core, { calendars: [home], events: [dentist] });

  render(App, { core });

  expect(await screen.findByText('Dentist')).toBeInTheDocument();
  scriptWindow(core, { calendars: [home], events: [dentist, holiday] });
  core.emit({ kind: 'occurrencesChanged', calendarIds: [home.id] });
  expect(await screen.findByText('Holiday')).toBeInTheDocument();
});
