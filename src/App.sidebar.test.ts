// Hiding the sidebar and changing its width, run against the fake core with a
// stand-in grid. jsdom has no layout: widths come from the state FreeCal
// keeps, never from the page, and dragging is also checked in the real app.
import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import App from './App.svelte';
import { FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

/** jsdom's window is 1024px wide with 16px to the rem: the maximum is 40%, 25.6rem. */
const MAX = '25.6';

async function showApp(core = new FakeCore()) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
  return { core, user: userEvent.setup() };
}

function sidebar() {
  return screen.queryByRole('navigation', { name: 'Calendars' });
}

function handle() {
  return screen.getByRole('separator', { name: 'Sidebar width' });
}

function width() {
  return handle().getAttribute('aria-valuenow');
}

/** Drags the handle by `by` pixels, as a mouse would. */
async function drag(by: number) {
  await fireEvent.pointerDown(handle(), { button: 0, clientX: 300 });
  await fireEvent.pointerMove(window, { clientX: 300 + by / 2 });
  await fireEvent.pointerMove(window, { clientX: 300 + by });
  await fireEvent.pointerUp(window, { clientX: 300 + by });
}

test('on first launch the sidebar shows at the default width', async () => {
  await showApp();

  expect(sidebar()).toBeInTheDocument();
  const toggle = screen.getByRole('button', { name: 'Hide sidebar' });
  expect(toggle).toHaveAttribute('aria-expanded', 'true');
  expect(toggle).toHaveAttribute('title', 'Hide sidebar (F9)');
  expect(width()).toBe('14');
  expect(handle()).toHaveAttribute('aria-valuemin', '10');
  expect(handle()).toHaveAttribute('aria-valuemax', MAX);
  expect(handle()).toHaveAttribute('aria-orientation', 'vertical');
  expect(handle()).toHaveAttribute('tabindex', '0');
});

test('opens hidden when it was left hidden, and shows again at its last width', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: false, width: 20 };
  const { user } = await showApp(core);

  expect(sidebar()).not.toBeInTheDocument();
  expect(screen.queryByRole('separator')).not.toBeInTheDocument();
  const toggle = screen.getByRole('button', { name: 'Show sidebar' });
  expect(toggle).toHaveAttribute('aria-expanded', 'false');

  await user.click(toggle);

  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('20');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 20 }]);
  expect(screen.getByRole('button', { name: 'Hide sidebar' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
});

test('the button hides the sidebar and remembers that', async () => {
  const { core, user } = await showApp();

  await user.click(screen.getByRole('button', { name: 'Hide sidebar' }));

  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([{ shown: false, width: null }]);
});

test('F9 hides and shows the sidebar, but not while typing', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 18 };
  const { user } = await showApp(core);

  await user.keyboard('{F9}');
  expect(sidebar()).not.toBeInTheDocument();

  await user.keyboard('{F9}');
  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('18');
  expect(core.sidebarSaves).toEqual([
    { shown: false, width: 18 },
    { shown: true, width: 18 },
  ]);

  await user.click(screen.getByRole('button', { name: 'New calendar' }));
  await user.keyboard('{F9}');
  expect(sidebar()).toBeInTheDocument();
});

test('the arrow keys change the width within its limits and save each press', async () => {
  const { core, user } = await showApp();
  handle().focus();

  await user.keyboard('{ArrowRight}');
  expect(width()).toBe('15');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 15 }]);

  await user.keyboard('{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}');
  expect(width()).toBe('10');
  expect(sidebar()).toBeInTheDocument();
  expect(core.sidebarSaves.at(-1)).toEqual({ shown: true, width: 10 });

  for (let press = 0; press < 20; press++) await user.keyboard('{ArrowRight}');
  expect(width()).toBe(MAX);
  expect(core.sidebarSaves.at(-1)).toEqual({ shown: true, width: 25.6 });
});

test('a stored width beyond the limits shows within them', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 80 };
  await showApp(core);

  expect(width()).toBe(MAX);
});

test('dragging changes the width and saves once when it ends', async () => {
  const { core } = await showApp();

  await drag(64);

  expect(width()).toBe('18');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 18 }]);
});

test('a drag that leaves the width as shown keeps a stored width beyond the limits', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 80 };
  await showApp(core);

  await drag(16);

  expect(width()).toBe(MAX);
  expect(core.sidebarSaves).toEqual([]);
});

test('dragging below the minimum hides the sidebar; showing it restores the width from before', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 16 };
  const { user } = await showApp(core);

  await drag(-160);

  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([{ shown: false, width: 16 }]);

  await user.click(screen.getByRole('button', { name: 'Show sidebar' }));
  expect(width()).toBe('16');
});

test('double-clicking the handle resets the width to the default', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 20 };
  const { user } = await showApp(core);

  await user.dblClick(handle());

  expect(width()).toBe('14');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: null }]);
});

test('says so when the sidebar can’t be remembered', async () => {
  const core = new FakeCore();
  core.setSidebar = async () => {
    throw new Error('disk full');
  };
  const { user } = await showApp(core);

  await user.keyboard('{F9}');

  expect(screen.getByRole('alert')).toHaveTextContent(
    'FreeCal could not remember the sidebar: Error: disk full',
  );
});
