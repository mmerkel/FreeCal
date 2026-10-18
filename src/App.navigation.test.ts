// The App's views, navigation and tracking today, run against the fake core
// with a stand-in grid (see StubGrid.svelte).
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { expect, test, vi } from 'vitest';
import App from './App.svelte';
import { FakeCore } from './test/fakeCore';
import { stubGrid } from './test/stubGrid';

vi.mock('./lib/CalendarGrid.svelte', () => import('./test/StubGrid.svelte'));

async function showApp(core = new FakeCore()) {
  render(App, { core });
  await screen.findByRole('list', { name: 'Grid' });
  return { core, user: userEvent.setup() };
}

/** What the grid is told to show. */
function shown() {
  const props = stubGrid.props!;
  return { view: props.view, date: props.date.toString(), today: props.today.toString() };
}

function todayButton() {
  return screen.getByRole('button', { name: 'Today' });
}

function viewButton(name: 'Day' | 'Week' | 'Month') {
  return screen.getByRole('button', { name });
}

function title() {
  return screen.getByRole('heading', { level: 1 }).textContent;
}

test('opens in month view on today, tracking today, on first launch', async () => {
  await showApp();

  expect(shown()).toEqual({ view: 'month', date: '2026-10-03', today: '2026-10-03' });
  expect(viewButton('Month')).toHaveAttribute('aria-pressed', 'true');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
  expect(title()).toBe('October 2026');
});

test('opens in the last-used view, always on today', async () => {
  const core = new FakeCore();
  core.lastView = 'week';
  core.currentDate = Temporal.PlainDate.from('2027-02-14');

  await showApp(core);

  expect(shown()).toEqual({ view: 'week', date: '2027-02-14', today: '2027-02-14' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('switching the view remembers it and keeps tracking today', async () => {
  const { core, user } = await showApp();

  await user.click(viewButton('Week'));

  expect(shown().view).toBe('week');
  expect(viewButton('Week')).toHaveAttribute('aria-pressed', 'true');
  expect(viewButton('Month')).toHaveAttribute('aria-pressed', 'false');
  expect(core.lastView).toBe('week');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
  // Weeks start on Monday.
  expect(title()).toMatch(/^Sep 28\s–\sOct 4, 2026$/);

  await user.click(viewButton('Day'));
  expect(core.lastView).toBe('day');
  expect(title()).toBe('Saturday, October 3, 2026');
});

test('next and previous move by the view’s period and end tracking today', async () => {
  const { user } = await showApp();

  await user.click(screen.getByRole('button', { name: 'Next month' }));
  expect(shown().date).toBe('2026-11-03');
  expect(title()).toBe('November 2026');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');

  await user.click(viewButton('Week'));
  await user.click(screen.getByRole('button', { name: 'Previous week' }));
  expect(shown().date).toBe('2026-10-27');

  await user.click(viewButton('Day'));
  await user.click(screen.getByRole('button', { name: 'Next day' }));
  expect(shown().date).toBe('2026-10-28');
});

test('moving back to the period with today still doesn’t track it', async () => {
  const { user } = await showApp();

  await user.click(screen.getByRole('button', { name: 'Next month' }));
  await user.click(screen.getByRole('button', { name: 'Previous month' }));

  expect(shown().date).toBe('2026-10-03');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');
});

test('pressing Today jumps to today and tracks it; un-pressing leaves the view where it is', async () => {
  const { user } = await showApp();
  await user.click(screen.getByRole('button', { name: 'Next month' }));

  await user.click(todayButton());
  expect(shown().date).toBe('2026-10-03');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');

  await user.click(todayButton());
  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');
});

test('clicking a day number opens that day and ends tracking today', async () => {
  const { core } = await showApp();

  stubGrid.props!.onopenday!(Temporal.PlainDate.from('2026-10-03'));
  await tick();

  expect(shown()).toMatchObject({ view: 'day', date: '2026-10-03' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');
  expect(core.lastView).toBe('day');
});

test('in day view, the month name in the title opens its month and keeps tracking today', async () => {
  const { core, user } = await showApp();
  await user.click(viewButton('Day'));

  await user.click(screen.getByRole('button', { name: 'Show October 2026' }));

  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
  expect(core.lastView).toBe('month');
});

test('in a week across two months, each month name opens its own month', async () => {
  const { user } = await showApp();
  await user.click(viewButton('Week'));
  expect(title()).toMatch(/^Sep 28\s–\sOct 4, 2026$/);

  await user.click(screen.getByRole('button', { name: 'Show September 2026' }));
  expect(shown()).toMatchObject({ view: 'month', date: '2026-09-01' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');

  await user.click(todayButton());
  await user.click(viewButton('Week'));
  await user.click(screen.getByRole('button', { name: 'Show October 2026' }));
  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('in a week within one month, its one month name opens it', async () => {
  const { user } = await showApp();
  await user.click(viewButton('Week'));
  await user.click(screen.getByRole('button', { name: 'Next week' }));
  expect(title()).toMatch(/^Oct 5\s–\s11, 2026$/);

  await user.click(screen.getByRole('button', { name: 'Show October 2026' }));
  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-10' });
});

test('in month view, the title has no month button', async () => {
  await showApp();

  expect(title()).toBe('October 2026');
  expect(screen.queryByRole('button', { name: /^Show / })).toBeNull();
});

test('on “Date changed” the view follows the new today while tracking', async () => {
  const { core } = await showApp();

  core.changeDate('2026-10-04');
  await tick();

  expect(shown()).toMatchObject({ date: '2026-10-04', today: '2026-10-04' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('on “Date changed” only the today highlight moves while not tracking', async () => {
  const { core, user } = await showApp();
  await user.click(todayButton());

  core.changeDate('2026-10-04');
  await tick();

  expect(shown()).toMatchObject({ date: '2026-10-03', today: '2026-10-04' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');
});

test('a “Date changed” during a press on the grid waits for its end to move the view', async () => {
  const { core } = await showApp();

  stubGrid.props!.oninteract!(true);
  core.changeDate('2026-10-04');
  await tick();

  // The grid holds back its own today highlight meanwhile.
  expect(shown()).toMatchObject({ date: '2026-10-03', today: '2026-10-04' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');

  stubGrid.props!.oninteract!(false);
  await tick();

  expect(shown()).toMatchObject({ date: '2026-10-04', today: '2026-10-04' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('a “Date changed” while the question for a drop is open waits for the answer', async () => {
  const core = new FakeCore();
  const home = core.addCalendar({ name: 'Home' });
  const yoga = core.addEvent({
    calendarId: home.id,
    title: 'Yoga',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:00:00' },
    repeat: { frequency: 'weekly', interval: 1, end: { kind: 'never' } },
  });
  await showApp(core);

  // The press ends as the Occurrence is dropped, and the question opens.
  stubGrid.props!.oninteract!(true);
  const dropped = stubGrid.props!.onreschedule!(yoga.id, '2026-10-05T09:00:00', {
    kind: 'timed',
    start: '2026-10-06T09:00:00',
    end: '2026-10-06T10:00:00',
  });
  stubGrid.props!.oninteract!(false);
  core.changeDate('2026-10-04');
  await tick();

  expect(stubGrid.props!.held).toBe(true);
  expect(shown()).toMatchObject({ date: '2026-10-03', today: '2026-10-04' });

  screen.getByRole('button', { name: 'Cancel' }).click();
  await expect(dropped).rejects.toBeDefined();
  await tick();

  expect(stubGrid.props!.held).toBe(false);
  expect(shown()).toMatchObject({ date: '2026-10-04', today: '2026-10-04' });
});

test('a press on the grid without a “Date changed” leaves the view where it is', async () => {
  const { core, user } = await showApp();
  await user.click(screen.getByRole('button', { name: 'Next month' }));
  const before = shown();

  stubGrid.props!.oninteract!(true);
  stubGrid.props!.oninteract!(false);
  core.changeDate('2026-10-04');
  await tick();

  expect(shown()).toEqual({ ...before, today: '2026-10-04' });
});

test('opening and closing dialogs keeps tracking today', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const { user } = await showApp(core);

  await user.keyboard('c');
  await user.click(await screen.findByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('selecting, creating and editing Events keeps tracking today', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const { user } = await showApp(core);
  const grid = screen.getByRole('list', { name: 'Grid' });

  stubGrid.props!.onselectslot!({
    kind: 'timed',
    start: '2026-10-05T09:00:00',
    end: '2026-10-05T10:00:00',
  });
  await tick();
  await user.type(screen.getByRole('textbox', { name: 'Title' }), 'Dentist{Enter}');
  await user.click(await within(grid).findByRole('button', { name: 'Dentist' }));
  await user.click(
    within(await screen.findByRole('dialog', { name: 'Dentist' })).getByRole('button', {
      name: 'Edit',
    }),
  );
  await user.type(screen.getByRole('textbox', { name: 'Title' }), ' at 9');
  await user.click(screen.getByRole('button', { name: 'Save' }));
  await within(grid).findByRole('button', { name: 'Dentist at 9' });

  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('Google’s shortcuts switch the view', async () => {
  const { user } = await showApp();

  for (const [key, view] of [
    ['d', 'day'],
    ['w', 'week'],
    ['m', 'month'],
    ['1', 'day'],
    ['2', 'week'],
    ['3', 'month'],
  ]) {
    await user.keyboard(key);
    expect(shown().view).toBe(view);
  }
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('Google’s shortcuts move to the next and previous period and to today', async () => {
  const { user } = await showApp();

  await user.keyboard('j');
  expect(shown().date).toBe('2026-11-03');
  await user.keyboard('n');
  expect(shown().date).toBe('2026-12-03');
  await user.keyboard('k');
  expect(shown().date).toBe('2026-11-03');
  await user.keyboard('p');
  expect(shown().date).toBe('2026-10-03');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'false');

  await user.keyboard('j');
  await user.keyboard('t');
  expect(shown().date).toBe('2026-10-03');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
  // t goes to today; it doesn't toggle like the button.
  await user.keyboard('t');
  expect(todayButton()).toHaveAttribute('aria-pressed', 'true');
});

test('the shortcuts do nothing with a modifier, while typing or with a dialog open', async () => {
  const core = new FakeCore();
  core.addCalendar({ name: 'Home' });
  const { user } = await showApp(core);

  await user.keyboard('{Control>}j{/Control}{Alt>}w{/Alt}');
  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });

  await user.keyboard('c');
  const title = await screen.findByRole('textbox', { name: 'Title' });
  await user.type(title, 'jw');
  expect(title).toHaveValue('jw');
  expect(shown()).toMatchObject({ view: 'month', date: '2026-10-03' });
});
