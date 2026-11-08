// The main menu, Settings and About, run against the fake core
// with a stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import App from './App.svelte';
import { expectAnchoredTo } from './test/anchored';
import { systemDarkQuery } from './test/darkQuery';
import { calendar, firstLaunch } from './test/answers';
import { failure } from './test/fakeCore';
import { showApp } from './test/showApp';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

function colourScheme() {
  return document.documentElement.dataset.colorScheme;
}

async function openMainMenu(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Main menu' }));
  return screen.getByRole('menu', { name: 'Main menu' });
}

test('the main menu offers Sync now, Import, Settings, About FreeCal and Quit', async () => {
  const { user } = await showApp();

  const menu = await openMainMenu(user);

  const items = within(menu).getAllByRole('menuitem');
  expect(items).toHaveLength(5);
  expect(items[0]).toHaveAccessibleName('Sync now');
  expect(items[1]).toHaveAccessibleName('Import…');
  expect(items[2]).toHaveAccessibleName('Settings…');
  expect(items[3]).toHaveAccessibleName('About FreeCal');
  expect(items[4]).toHaveAccessibleName('Quit');

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
  const core = firstLaunch();
  core.answer('listCalendars', [calendar({ name: 'Home' })]);
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

  core.answer('updateSettings', { appearance: 'dark' });
  await user.click(within(appearance).getByRole('radio', { name: 'Dark' }));

  expect(colourScheme()).toBe('dark');
  expect(within(appearance).getByRole('radio', { name: 'Dark' })).toBeChecked();
  expect(core.callsOf('updateSettings')).toEqual([[{ appearance: 'dark' }]]);
  // Every change is saved at once, so there is nothing to save or cancel.
  expect(within(settings).queryByRole('button', { name: /save|cancel/i })).not.toBeInTheDocument();

  core.answer('updateSettings', { appearance: 'system' });
  await user.click(within(appearance).getByRole('radio', { name: 'System' }));
  expect(colourScheme()).toBe('light');
  systemDarkQuery.flip(true);
  expect(colourScheme()).toBe('dark');
});

test('a setting that can’t be saved is reported in words and goes back to the stored one', async () => {
  const { core, user } = await showApp();
  core.answer('updateSettings', failure({ code: 'localStoreFailed' }));

  await user.keyboard('{Control>},{/Control}');
  await user.click(screen.getByRole('radio', { name: 'Dark' }));

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The setting couldn’t be saved: FreeCal’s database couldn’t be read or written',
  );
  await waitFor(() => expect(colourScheme()).toBe('light'));
  expect(screen.getByRole('radio', { name: 'System' })).toBeChecked();
});

test('opens with the stored appearance and shows the window once it is known', async () => {
  const core = firstLaunch();
  core.answer('settings', { appearance: 'dark' });
  let schemeWhenShown: string | undefined;
  const show = core.showWindow;
  core.showWindow = () => {
    schemeWhenShown = colourScheme();
    return show();
  };

  const { user } = await showApp(core);

  expect(colourScheme()).toBe('dark');
  expect(schemeWhenShown).toBe('dark');
  await user.keyboard('{Control>},{/Control}');
  expect(screen.getByRole('radio', { name: 'Dark' })).toHaveFocus();
});

test('a startup failure is drawn with the frontend’s default settings', async () => {
  const core = firstLaunch();
  core.answer('settings', { appearance: 'dark' });
  core.answer('startupFailure', { code: 'localStoreInUse' });

  render(App, { core });

  expect(await screen.findByRole('heading', { name: 'FreeCal couldn’t start' })).toBeVisible();
  expect(colourScheme()).toBe('light');
  expect(core.callsOf('showWindow')).toEqual([[]]);
});

test.each([
  ['localStoreInUse', 'FreeCal is already running, but it couldn’t be brought to the front.'],
  ['newerLocalStore', 'Your calendar data was saved by a newer version of FreeCal.'],
  ['localStoreFailed', 'FreeCal couldn’t read its database. It may be damaged.'],
] as const)('the startup error screen says why FreeCal couldn’t start: %s', async (code, why) => {
  const core = firstLaunch();
  core.answer('startupFailure', { code });

  render(App, { core });

  expect(await screen.findByRole('alert')).toHaveTextContent(why);
  expect(core.callsOf('settings')).toEqual([]);
});

test('the startup error screen quits FreeCal', async () => {
  const core = firstLaunch();
  core.answer('startupFailure', { code: 'newerLocalStore' });
  render(App, { core });
  const user = userEvent.setup();

  await user.click(await screen.findByRole('button', { name: 'Quit FreeCal' }));

  expect(core.callsOf('quit')).toEqual([[]]);
});

test('About FreeCal shows the version and the licence, and opens the source through the shell', async () => {
  const core = firstLaunch();
  core.answer('version', '0.1.0');
  const { user } = await showApp(core);

  await user.click(
    within(await openMainMenu(user)).getByRole('menuitem', { name: 'About FreeCal' }),
  );

  const about = screen.getByRole('dialog', { name: 'About FreeCal' });
  expect(about).toHaveTextContent('FreeCal');
  expect(await within(about).findByText('Version 0.1.0')).toBeInTheDocument();
  expect(about).toHaveTextContent('Licensed under the GNU GPL, version 3 or later');
  expect(about).toHaveTextContent('Copyright ©');

  await user.click(within(about).getByRole('button', { name: 'Source code' }));
  expect(core.callsOf('openLink')).toEqual([['https://github.com/mmerkel/FreeCal']]);
  expect(within(about).queryByRole('link')).not.toBeInTheDocument();
});

test('Settings and About close only through their ✕, which About focuses', async () => {
  const { user } = await showApp();

  await user.keyboard('{Control>},{/Control}');
  const settings = screen.getByRole('dialog', { name: 'Settings' });
  const closeSettings = within(settings).getByRole('button', { name: 'Close' });
  expect(within(settings).getAllByRole('button')).toEqual([closeSettings]);
  expect(closeSettings.querySelector('svg')).not.toBeNull();
  await user.click(closeSettings);
  await user.click(
    within(await openMainMenu(user)).getByRole('menuitem', { name: 'About FreeCal' }),
  );

  const about = screen.getByRole('dialog', { name: 'About FreeCal' });
  const closeAbout = within(about).getByRole('button', { name: 'Close' });
  expect(closeAbout).toHaveFocus();
  expect(closeAbout.querySelector('svg')).not.toBeNull();
  expect(about.querySelector('.actions')).toBeNull();
  await user.click(closeAbout);
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

test('Quit ends FreeCal', async () => {
  const { core, user } = await showApp();

  await user.click(within(await openMainMenu(user)).getByRole('menuitem', { name: 'Quit' }));

  expect(core.callsOf('quit')).toEqual([[]]);
});
