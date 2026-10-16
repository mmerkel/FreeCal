// The main menu, Settings and About, run against the fake core with a
// stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import App from './App.svelte';
import { expectAnchoredTo } from './test/anchored';
import { systemDarkQuery } from './test/darkQuery';
import { FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

async function showApp(core = new FakeCore()) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
  return { core, user: userEvent.setup() };
}

function colourScheme() {
  return document.documentElement.dataset.colorScheme;
}

async function openMainMenu(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  return screen.getByRole('menu', { name: 'Main menu' });
}

test('the main menu offers Settings, About FreeCal and Quit', async () => {
  const { user } = await showApp();

  const menu = await openMainMenu(user);

  const items = within(menu).getAllByRole('menuitem');
  expect(items).toHaveLength(3);
  expect(items[0]).toHaveAccessibleName('Settings…');
  expect(items[1]).toHaveAccessibleName('About FreeCal');
  expect(items[2]).toHaveAccessibleName('Quit');

  await user.click(within(menu).getByRole('menuitem', { name: 'Settings…' }));
  expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

test('the main menu is a popover anchored to its button', async () => {
  const { user } = await showApp();

  const menu = await openMainMenu(user);

  expect(menu).toHaveAttribute('popover', 'manual');
  expectAnchoredTo(menu, screen.getByRole('button', { name: 'Main menu' }));
});

test('Ctrl+, opens Settings, also while typing, but not over another dialog', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const { user } = await showApp(core);

  await user.keyboard('{Control>},{/Control}');
  const settings = screen.getByRole('dialog', { name: 'Settings' });
  await user.click(within(settings).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

  await user.keyboard('c');
  const title = await screen.findByRole('textbox', { name: 'Title' });
  await user.type(title, 'x{Control>},{/Control}');
  expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
  expect(title).toHaveValue('x');
});

test('choosing an appearance writes it once and applies it at once', async () => {
  const { core, user } = await showApp();
  expect(colourScheme()).toBe('light');

  await user.keyboard('{Control>},{/Control}');
  const settings = screen.getByRole('dialog', { name: 'Settings' });
  const appearance = within(settings).getByRole('group', { name: 'Appearance' });
  expect(within(appearance).getByRole('radio', { name: 'System' })).toBeChecked();
  expect(within(appearance).getByRole('radio', { name: 'System' })).toHaveFocus();

  await user.click(within(appearance).getByRole('radio', { name: 'Dark' }));

  expect(colourScheme()).toBe('dark');
  expect(within(appearance).getByRole('radio', { name: 'Dark' })).toBeChecked();
  await waitFor(() => expect(core.settingsUpdates).toEqual([{ appearance: 'dark' }]));
  expect(core.storedSettings.appearance).toBe('dark');
  // Every change is saved at once, so there is nothing to save or cancel.
  expect(within(settings).queryByRole('button', { name: /save|cancel/i })).not.toBeInTheDocument();

  await user.click(within(appearance).getByRole('radio', { name: 'System' }));
  expect(colourScheme()).toBe('light');
  systemDarkQuery.flip(true);
  expect(colourScheme()).toBe('dark');
});

test('a setting that can’t be saved is reported and goes back to the stored one', async () => {
  const { core, user } = await showApp();
  vi.spyOn(core, 'updateSettings').mockRejectedValue(new Error('disk full'));

  await user.keyboard('{Control>},{/Control}');
  await user.click(screen.getByRole('radio', { name: 'Dark' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('disk full');
  await waitFor(() => expect(colourScheme()).toBe('light'));
  expect(screen.getByRole('radio', { name: 'System' })).toBeChecked();
});

test('opens with the stored appearance and shows the window once it is known', async () => {
  const core = new FakeCore();
  core.storedSettings = { appearance: 'dark' };
  let schemeWhenShown: string | undefined;
  const show = core.showWindow.bind(core);
  core.showWindow = () => {
    schemeWhenShown = colourScheme();
    return show();
  };

  const { user } = await showApp(core);

  expect(colourScheme()).toBe('dark');
  expect(core.windowShown).toBe(true);
  await user.keyboard('{Control>},{/Control}');
  expect(screen.getByRole('radio', { name: 'Dark' })).toHaveFocus();
  expect(core.calls.indexOf('settings')).toBeLessThan(core.calls.indexOf('showWindow'));
  expect(schemeWhenShown).toBe('dark');
});

test('a startup failure is drawn with the frontend’s default settings', async () => {
  const core = new FakeCore();
  core.storedSettings = { appearance: 'dark' };
  core.startupFailure = 'LocalStoreInUse';

  render(App, { core });

  expect(await screen.findByRole('alert')).toHaveTextContent('LocalStoreInUse');
  expect(colourScheme()).toBe('light');
  expect(core.windowShown).toBe(true);
});

test('About FreeCal shows the version and the licence, and opens the source through the shell', async () => {
  const { core, user } = await showApp();

  await user.click(
    within(await openMainMenu(user)).getByRole('menuitem', { name: 'About FreeCal' }),
  );

  const about = screen.getByRole('dialog', { name: 'About FreeCal' });
  expect(about).toHaveTextContent('FreeCal');
  expect(await within(about).findByText('Version 0.1.0')).toBeInTheDocument();
  expect(about).toHaveTextContent('Licensed under the GNU GPL, version 3 or later');
  expect(about).toHaveTextContent('Copyright ©');

  await user.click(within(about).getByRole('button', { name: 'Source code' }));
  expect(core.openedLinks).toEqual(['https://github.com/mmerkel/FreeCal']);
  expect(within(about).queryByRole('link')).not.toBeInTheDocument();
});

test('Quit ends FreeCal', async () => {
  const { core, user } = await showApp();

  await user.click(within(await openMainMenu(user)).getByRole('menuitem', { name: 'Quit' }));

  expect(core.hasQuit).toBe(true);
});
