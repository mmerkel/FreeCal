// Hiding the sidebar and changing its width, run against the fake core with a
// stand-in grid. jsdom has no layout: widths come from the state FreeCal
// keeps, never from the page, and dragging is also checked in the real app.
import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import App from './App.svelte';
import { FakeCore } from './test/fakeCore';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

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

/** The tab, the separator's visible grip. */
function tab() {
  return handle().firstElementChild!;
}

// Pointer positions are pixels from the window's left edge, 16 to the rem.
// The sidebar's edge is at its width: 192px at the default 12rem.

async function press(target: Element, x: number) {
  await fireEvent.pointerDown(target, { button: 0, clientX: x });
}

async function move(x: number) {
  await fireEvent.pointerMove(window, { clientX: x });
}

async function release(x: number) {
  await fireEvent.pointerUp(window, { clientX: x });
}

/** Presses the edge at `from`, moves through `to` and lets go at the last of them. */
async function drag(from: number, ...to: number[]) {
  await press(handle(), from);
  for (const x of to) await move(x);
  await release(to.at(-1)!);
}

test('on first launch the sidebar shows at the default width, with the tab on its edge', async () => {
  await showApp();

  expect(sidebar()).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /sidebar/ })).not.toBeInTheDocument();
  expect(handle()).toHaveAttribute('title', 'Hide sidebar (F9)');
  expect(width()).toBe('12');
  expect(handle()).toHaveAttribute('aria-valuemax', '20');
  expect(handle()).toHaveAttribute('aria-orientation', 'vertical');
  expect(handle()).toHaveAttribute('tabindex', '0');
});

test('opens hidden when it was left hidden, and the tab shows it again at its last width', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: false, width: 15 };
  const { user } = await showApp(core);

  expect(sidebar()).not.toBeInTheDocument();
  expect(handle()).toHaveAttribute('title', 'Show sidebar (F9)');
  expect(width()).toBe('0');

  await user.click(tab());

  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('15');
  expect(handle()).toHaveAttribute('title', 'Hide sidebar (F9)');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 15 }]);
});

test('a click on the tab hides the sidebar and remembers that', async () => {
  const { core, user } = await showApp();

  await user.click(tab());

  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([{ shown: false, width: null }]);
});

test('Enter on the separator hides and shows the sidebar', async () => {
  const { core, user } = await showApp();
  handle().focus();

  await user.keyboard('{Enter}');
  expect(sidebar()).not.toBeInTheDocument();

  await user.keyboard('{Enter}');
  expect(sidebar()).toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([
    { shown: false, width: null },
    { shown: true, width: null },
  ]);
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
  expect(width()).toBe('13');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 13 }]);

  for (let press = 0; press < 10; press++) await user.keyboard('{ArrowLeft}');
  expect(width()).toBe('8');
  expect(sidebar()).toBeInTheDocument();
  expect(core.sidebarSaves.at(-1)).toEqual({ shown: true, width: 8 });

  for (let press = 0; press < 20; press++) await user.keyboard('{ArrowRight}');
  expect(width()).toBe('20');
  expect(core.sidebarSaves.at(-1)).toEqual({ shown: true, width: 20 });
});

test('a stored width beyond the limits shows within them, and stays stored', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 40 };
  await showApp(core);

  expect(width()).toBe('20');
  expect(core.sidebarSaves).toEqual([]);
});

test('a stored width below the minimum shows at the minimum', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 2 };
  await showApp(core);

  expect(width()).toBe('8');
});

test('dragging changes the width and saves once when it ends', async () => {
  const { core } = await showApp();

  await drag(192, 224, 256);

  expect(width()).toBe('16');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 16 }]);
});

test('the sidebar follows the edge from where it was pressed', async () => {
  const { core } = await showApp();

  await press(tab(), 200);
  await move(240);
  await release(240);

  expect(width()).toBe('14.5');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 14.5 }]);
});

test('a drag stops at the limits and doesn’t hide the sidebar above 4rem', async () => {
  const { core } = await showApp();

  await drag(192, 600);
  expect(width()).toBe('20');

  await drag(320, 70);
  expect(width()).toBe('8');
  expect(sidebar()).toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([
    { shown: true, width: 20 },
    { shown: true, width: 8 },
  ]);
});

test('a drag that leaves the width as shown keeps a stored width beyond the limits', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 40 };
  await showApp(core);

  await drag(320, 330);

  expect(width()).toBe('20');
  expect(core.sidebarSaves).toEqual([]);
});

test('dragging below 4rem hides the sidebar at once; letting go there keeps the width from before', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: true, width: 16 };
  const { user } = await showApp(core);

  await press(handle(), 256);
  await move(60);
  expect(sidebar()).not.toBeInTheDocument();
  expect(width()).toBe('0');
  await move(70);
  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('8');
  await move(50);
  await release(50);

  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([{ shown: false, width: 16 }]);

  await user.click(tab());
  expect(width()).toBe('16');
});

test('pulling the tab out of a hidden sidebar shows it past 4rem, at the minimum or wider', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: false, width: 16 };
  await showApp(core);

  await press(tab(), 0);
  await move(40);
  expect(sidebar()).not.toBeInTheDocument();
  await move(70);
  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('8');
  await move(240);
  await release(240);

  expect(width()).toBe('15');
  expect(core.sidebarSaves).toEqual([{ shown: true, width: 15 }]);
});

test('a pull from a hidden sidebar that ends below 4rem saves nothing', async () => {
  const core = new FakeCore();
  core.storedSidebar = { shown: false, width: 16 };
  await showApp(core);

  await press(tab(), 0);
  await move(40);
  await release(40);

  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([]);
});

test('a press on the tab that moves less than 4px is a click; 4px or more is a drag', async () => {
  const { core } = await showApp();

  await press(tab(), 200);
  await move(203);
  await release(203);
  expect(sidebar()).not.toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([{ shown: false, width: null }]);

  await press(tab(), 5);
  await move(2);
  await release(2);
  expect(sidebar()).toBeInTheDocument();

  await press(tab(), 200);
  await move(204);
  await release(204);
  expect(sidebar()).toBeInTheDocument();
  expect(width()).toBe('12.3');
  expect(core.sidebarSaves.at(-1)).toEqual({ shown: true, width: 12.25 });
});

test('a press on the edge beside the tab doesn’t hide the sidebar', async () => {
  const { core } = await showApp();

  await drag(192, 192);

  expect(sidebar()).toBeInTheDocument();
  expect(core.sidebarSaves).toEqual([]);
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
