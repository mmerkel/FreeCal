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
  core.currentDate = '2027-02-14';

  render(App, { core });

  expect(await screen.findByRole('grid', { name: 'February 2027' })).toBeInTheDocument();
  expect(screen.getByRole('gridcell', { name: 'February 14, 2027' })).toHaveAttribute(
    'aria-current',
    'date',
  );
});
