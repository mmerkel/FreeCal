import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import App from '../App.svelte';
import { firstLaunch } from './answers';
import type { FakeCore } from './fakeCore';

/**
 * Renders the whole window on `core` and waits until it has started, as the
 * toolbar's title shows; the grid and the sidebar show with it.
 */
export async function showApp(core: FakeCore = firstLaunch()) {
  render(App, { core });
  await screen.findByRole('heading', { level: 1 });
  return { core, user: userEvent.setup() };
}
